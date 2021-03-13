//! Song decoding / analysis module.
//!
//! Use decoding, and features-extraction functions from other modules
//! e.g. tempo features, spectral features, etc to build a Song and its
//! corresponding Analysis.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

extern crate crossbeam;
extern crate ffmpeg_next as ffmpeg;
extern crate ndarray;
extern crate ndarray_npy;

use super::CHANNELS;
use crate::chroma::ChromaDesc;
use crate::misc::LoudnessDesc;
use crate::temporal::BPMDesc;
use crate::timbral::{SpectralDesc, ZeroCrossingRateDesc};
use crate::{BlissError, Song, SAMPLE_RATE};
use ::log::warn;
use crossbeam::thread;
use ffmpeg::codec::threading::{Config, Type as ThreadingType};
use ffmpeg::util;
use ffmpeg::util::format::sample::{Sample, Type};
use ffmpeg_next::util::channel_layout::ChannelLayout;
use ffmpeg_next::util::error::Error;
use ffmpeg_next::util::error::EINVAL;
use ffmpeg_next::util::log;
use ffmpeg_next::util::log::level::Level;
use ndarray::{arr1, Array};
use std::sync::mpsc;
use std::thread as std_thread;

fn push_to_sample_array(frame: &ffmpeg::frame::Audio, sample_array: &mut Vec<f32>) {
    if frame.samples() == 0 {
        return;
    }
    // Account for the padding
    let actual_size = util::format::sample::Buffer::size(
        Sample::F32(Type::Packed),
        CHANNELS,
        frame.samples(),
        false,
    );
    let f32_frame: Vec<f32> = frame.data(0)[..actual_size]
        .chunks_exact(4)
        .map(|x| {
            let mut a: [u8; 4] = [0; 4];
            a.copy_from_slice(x);
            f32::from_le_bytes(a)
        })
        .collect();
    sample_array.extend_from_slice(&f32_frame);
}

#[derive(Default, Debug)]
pub(crate) struct InternalSong {
    pub path: String,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub track_number: String,
    pub genre: String,
    pub sample_array: Vec<f32>,
}

impl Song {
    #[allow(dead_code)]
    pub fn distance(&self, other: &Self) -> f32 {
        let a1 = arr1(&self.analysis.to_vec());
        let a2 = arr1(&other.analysis.to_vec());
        // Could be any square symmetric positive semi-definite matrix;
        // just no metric learning has been done yet.
        // See https://lelele.io/thesis.pdf chapter 4.
        let m = Array::eye(self.analysis.len());

        (arr1(&self.analysis) - &a2).dot(&m).dot(&(&a1 - &a2))
    }

    pub fn new(path: &str) -> Result<Self, BlissError> {
        let raw_song = Song::decode(&path)?;

        Ok(Song {
            path: raw_song.path,
            artist: raw_song.artist,
            title: raw_song.title,
            album: raw_song.album,
            track_number: raw_song.track_number,
            genre: raw_song.genre,
            analysis: Song::analyse(raw_song.sample_array)?,
        })
    }

    /**
     * Analyse a song decoded in `sample_array`, with one channel @ 22050 Hz.
     *
     * The current implementation doesn't make use of it,
     * but the song can also be streamed wrt.
     * each descriptor (with the exception of the chroma descriptor which
     * yields worse results when streamed).
     *
     * Useful in the rare cases where the full song is not
     * completely available.
     **/
    fn analyse(sample_array: Vec<f32>) -> Result<Vec<f32>, BlissError> {
        let largest_window = vec![
            BPMDesc::WINDOW_SIZE,
            ChromaDesc::WINDOW_SIZE,
            SpectralDesc::WINDOW_SIZE,
            LoudnessDesc::WINDOW_SIZE,
        ]
        .into_iter()
        .max()
        .unwrap();
        if sample_array.len() < largest_window {
            return Err(BlissError::AnalysisError(String::from(
                "empty or too short song.",
            )));
        }

        thread::scope(|s| {
            let child_tempo: thread::ScopedJoinHandle<'_, Result<f32, BlissError>> =
                s.spawn(|_| {
                    let mut tempo_desc = BPMDesc::new(SAMPLE_RATE)?;
                    let windows = sample_array
                        .windows(BPMDesc::WINDOW_SIZE)
                        .step_by(BPMDesc::HOP_SIZE);

                    for window in windows {
                        tempo_desc.do_(&window)?;
                    }
                    Ok(tempo_desc.get_value())
                });

            let child_chroma: thread::ScopedJoinHandle<'_, Result<Vec<f32>, BlissError>> =
                s.spawn(|_| {
                    let mut chroma_desc = ChromaDesc::new(SAMPLE_RATE, 12);
                    chroma_desc.do_(&sample_array)?;
                    Ok(chroma_desc.get_values())
                });

            let child_timbral: thread::ScopedJoinHandle<
                '_,
                Result<(Vec<f32>, Vec<f32>, Vec<f32>), BlissError>,
            > = s.spawn(|_| {
                let mut spectral_desc = SpectralDesc::new(SAMPLE_RATE)?;
                let windows = sample_array
                    .windows(SpectralDesc::WINDOW_SIZE)
                    .step_by(SpectralDesc::HOP_SIZE);
                for window in windows {
                    spectral_desc.do_(&window)?;
                }
                let centroid = spectral_desc.get_centroid();
                let rolloff = spectral_desc.get_rolloff();
                let flatness = spectral_desc.get_flatness();
                Ok((centroid, rolloff, flatness))
            });

            let child_zcr: thread::ScopedJoinHandle<'_, Result<f32, BlissError>> = s.spawn(|_| {
                let mut zcr_desc = ZeroCrossingRateDesc::default();
                zcr_desc.do_(&sample_array);
                Ok(zcr_desc.get_value())
            });

            let child_loudness: thread::ScopedJoinHandle<'_, Result<Vec<f32>, BlissError>> = s
                .spawn(|_| {
                    let mut loudness_desc = LoudnessDesc::default();
                    let windows = sample_array.chunks(LoudnessDesc::WINDOW_SIZE);

                    for window in windows {
                        loudness_desc.do_(&window);
                    }
                    Ok(loudness_desc.get_value())
                });

            // Non-streaming approach for that one
            let tempo = child_tempo.join().unwrap()?;
            let chroma = child_chroma.join().unwrap()?;
            let (centroid, rolloff, flatness) = child_timbral.join().unwrap()?;
            let loudness = child_loudness.join().unwrap()?;
            let zcr = child_zcr.join().unwrap()?;

            let mut result = vec![tempo, zcr];
            result.extend_from_slice(&centroid);
            result.extend_from_slice(&rolloff);
            result.extend_from_slice(&flatness);
            result.extend_from_slice(&loudness);
            result.extend_from_slice(&chroma);
            Ok(result)
        })
        .unwrap()
    }

    // TODO DRY me
    pub(crate) fn decode(path: &str) -> Result<InternalSong, BlissError> {
        ffmpeg::init()
            .map_err(|e| BlissError::DecodingError(format!("ffmpeg init error: {:?}.", e)))?;
        log::set_level(Level::Quiet);
        let mut song = InternalSong {
            path: path.to_string(),
            ..Default::default()
        };
        let mut format = ffmpeg::format::input(&path)
            .map_err(|e| BlissError::DecodingError(format!("while opening format: {:?}.", e)))?;
        let (mut codec, stream, expected_sample_number) = {
            let stream = format
                .streams()
                .find(|s| s.codec().medium() == ffmpeg::media::Type::Audio)
                .ok_or(BlissError::DecodingError(String::from(
                    "No audio stream found.",
                )))?;
            stream.codec().set_threading(Config {
                kind: ThreadingType::Frame,
                count: 0,
                safe: true,
            });
            let codec =
                stream.codec().decoder().audio().map_err(|e| {
                    BlissError::DecodingError(format!("when finding codec: {:?}.", e))
                })?;
            // Add SAMPLE_RATE to have one second margin to avoid reallocating if
            // the duration is slightly more than estimated
            // TODO another way to get the exact number of samples is to decode
            // everything once, compute the real number of samples from that,
            // allocate the array with that number, and decode again. Check
            // what's faster between reallocating, and just have one second
            // leeway.
            let expected_sample_number = (SAMPLE_RATE as f32 * stream.duration() as f32
                / stream.time_base().denominator() as f32)
                .ceil()
                + SAMPLE_RATE as f32;
            (codec, stream.index(), expected_sample_number)
        };
        let mut sample_array: Vec<f32> = Vec::with_capacity(expected_sample_number as usize);
        if let Some(title) = format.metadata().get("title") {
            song.title = title.to_string();
        };
        if let Some(artist) = format.metadata().get("artist") {
            song.artist = artist.to_string();
        };
        if let Some(album) = format.metadata().get("album") {
            song.album = album.to_string();
        };
        if let Some(genre) = format.metadata().get("genre") {
            song.genre = genre.to_string();
        };
        if let Some(track_number) = format.metadata().get("track") {
            song.track_number = track_number.to_string();
        };
        let in_channel_layout = {
            if codec.channel_layout() == ChannelLayout::empty() {
                ChannelLayout::default(codec.channels().into())
            } else {
                codec.channel_layout()
            }
        };
        codec.set_channel_layout(in_channel_layout);
        let mut resample_context = ffmpeg::software::resampling::context::Context::get(
            codec.format(),
            in_channel_layout,
            codec.rate(),
            Sample::F32(Type::Packed),
            ffmpeg::util::channel_layout::ChannelLayout::MONO,
            SAMPLE_RATE,
        )
        .map_err(|e| {
            BlissError::DecodingError(format!(
                "while trying to allocate resampling context: {:?}",
                e
            ))
        })?;

        let (tx, rx) = mpsc::channel();
        let child = std_thread::spawn(move || -> Result<Vec<f32>, BlissError> {
            let mut resampled = ffmpeg::frame::Audio::empty();
            for decoded in rx.iter() {
                resample_context
                    .run(&decoded, &mut resampled)
                    .map_err(|e| {
                        BlissError::DecodingError(format!("while trying to resample song: {:?}", e))
                    })?;
                push_to_sample_array(&resampled, &mut sample_array);
            }
            loop {
                match resample_context.flush(&mut resampled).map_err(|e| {
                    BlissError::DecodingError(format!("while trying to resample song: {:?}", e))
                })? {
                    Some(_) => {
                        push_to_sample_array(&resampled, &mut sample_array);
                    }
                    None => {
                        if resampled.samples() == 0 {
                            break;
                        }
                        push_to_sample_array(&resampled, &mut sample_array);
                    }
                };
            }
            Ok(sample_array)
        });
        for (s, packet) in format.packets() {
            if s.index() != stream {
                continue;
            }
            match codec.send_packet(&packet) {
                Ok(_) => (),
                Err(Error::Other { errno: EINVAL }) => {
                    return Err(BlissError::DecodingError(String::from(
                        "wrong codec opened.",
                    )))
                }
                Err(Error::Eof) => {
                    warn!("Premature EOF reached while decoding.");
                    drop(tx);
                    song.sample_array = child.join().unwrap()?;
                    return Ok(song);
                }
                // Silently fail on decoding errors; pray for the best
                Err(_) => (),
            };

            loop {
                let mut decoded = ffmpeg::frame::Audio::empty();
                match codec.receive_frame(&mut decoded) {
                    Ok(_) => {
                        tx.send(decoded).map_err(|e| {
                            BlissError::DecodingError(format!(
                                "while sending decoded frame to the resampling thread: {:?}",
                                e
                            ))
                        })?;
                    }
                    Err(_) => break,
                }
            }
        }

        // Flush the stream
        // TODO check that it's still how to do this
        let packet = ffmpeg::codec::packet::Packet::empty();
        match codec.send_packet(&packet) {
            Ok(_) => (),
            Err(Error::Other { errno: EINVAL }) => {
                return Err(BlissError::DecodingError(String::from(
                    "wrong codec opened.",
                )))
            }
            Err(Error::Eof) => {
                warn!("Premature EOF reached while decoding.");
                drop(tx);
                song.sample_array = child.join().unwrap()?;
                return Ok(song);
            }
            // Silently fail on decoding errors; pray for the best
            Err(_) => (),
        };

        loop {
            let mut decoded = ffmpeg::frame::Audio::empty();
            match codec.receive_frame(&mut decoded) {
                Ok(_) => {
                    tx.send(decoded).map_err(|e| {
                        BlissError::DecodingError(format!(
                            "while sending decoded frame to the resampling thread: {:?}",
                            e
                        ))
                    })?;
                }
                Err(_) => break,
            }
        }

        drop(tx);
        song.sample_array = child.join().unwrap()?;
        Ok(song)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ripemd160::{Digest, Ripemd160};

    #[test]
    fn test_analysis_too_small() {
        let error = Song::analyse(vec![0.]).unwrap_err();
        assert_eq!(
            error,
            BlissError::AnalysisError(String::from("empty or too short song."))
        );

        let error = Song::analyse(vec![]).unwrap_err();
        assert_eq!(
            error,
            BlissError::AnalysisError(String::from("empty or too short song."))
        );
    }

    #[test]
    fn test_analyse() {
        let song = Song::new("data/s16_mono_22_5kHz.flac").unwrap();
        let expected_analysis = vec![
            0.3709823,
            -0.849141,
            -0.75481045,
            -0.8790748,
            -0.63258266,
            -0.7258959,
            -0.775738,
            -0.8146726,
            0.2716726,
            0.25779057,
            -0.35661936,
            -0.63578653,
            -0.29593682,
            0.06421304,
            0.21852458,
            -0.581239,
            -0.9466835,
            -0.9481153,
            -0.9820945,
            -0.95968974,
        ];
        for (x, y) in song.analysis.iter().zip(expected_analysis) {
            assert!(0.01 > (x - y).abs());
        }
    }

    fn _test_decode(path: &str, expected_hash: &[u8]) {
        let song = Song::decode(path).unwrap();
        let mut hasher = Ripemd160::new();
        for sample in song.sample_array.iter() {
            hasher.update(sample.to_le_bytes().to_vec());
        }

        assert_eq!(expected_hash, hasher.finalize().as_slice());
    }

    #[test]
    fn test_tags() {
        let song = Song::decode("data/s16_mono_22_5kHz.flac").unwrap();
        assert_eq!(song.artist, "David TMX");
        assert_eq!(song.title, "Renaissance");
        assert_eq!(song.album, "Renaissance");
        assert_eq!(song.track_number, "02");
        assert_eq!(song.genre, "Pop");
    }

    #[test]
    fn test_resample_multi() {
        let path = String::from("data/s32_stereo_44_1_kHz.flac");
        let expected_hash = [
            0xc5, 0xf8, 0x23, 0xce, 0x63, 0x2c, 0xf4, 0xa0, 0x72, 0x66, 0xbb, 0x49, 0xad, 0x84,
            0xb6, 0xea, 0x48, 0x48, 0x9c, 0x50,
        ];
        _test_decode(&path, &expected_hash);
    }

    #[test]
    fn test_resample_stereo() {
        let path = String::from("data/s16_stereo_22_5kHz.flac");
        let expected_hash = [
            0x24, 0xed, 0x45, 0x58, 0x06, 0xbf, 0xfb, 0x05, 0x57, 0x5f, 0xdc, 0x4d, 0xb4, 0x9b,
            0xa5, 0x2b, 0x05, 0x56, 0x10, 0x4f,
        ];
        _test_decode(&path, &expected_hash);
    }

    #[test]
    fn test_decode_mono() {
        let path = String::from("data/s16_mono_22_5kHz.flac");
        // Obtained through
        // ffmpeg -i data/s16_mono_22_5kHz.flac -ar 22050 -ac 1 -c:a pcm_f32le
        // -f hash -hash ripemd160 -
        let expected_hash = [
            0x9d, 0x95, 0xa5, 0xf2, 0xd2, 0x9c, 0x68, 0xe8, 0x8a, 0x70, 0xcd, 0xf3, 0x54, 0x2c,
            0x5b, 0x45, 0x98, 0xb4, 0xf3, 0xb4,
        ];
        _test_decode(&path, &expected_hash);
    }

    #[test]
    fn test_decode_mp3() {
        let path = String::from("data/s32_stereo_44_1_kHz.mp3");
        // Obtained through
        // ffmpeg -i data/s16_mono_22_5kHz.mp3 -ar 22050 -ac 1 -c:a pcm_f32le
        // -f hash -hash ripemd160 -
        let expected_hash = [
            0x28, 0x25, 0x6b, 0x7b, 0x6e, 0x37, 0x1c, 0xcf, 0xc7, 0x06, 0xdf, 0x62, 0x8c, 0x0e,
            0x91, 0xf7, 0xd6, 0x1f, 0xac, 0x5b,
        ];
        _test_decode(&path, &expected_hash);
    }

    #[test]
    fn test_dont_panic_no_channel_layout() {
        let path = String::from("data/no_channel.wav");
        Song::decode(&path).unwrap();
    }

    #[test]
    fn test_decode_right_capacity_vec() {
        let path = String::from("data/s16_mono_22_5kHz.flac");
        let song = Song::decode(&path).unwrap();
        let sample_array = song.sample_array;
        assert_eq!(
            sample_array.len() + SAMPLE_RATE as usize,
            sample_array.capacity()
        );

        let path = String::from("data/s32_stereo_44_1_kHz.flac");
        let song = Song::decode(&path).unwrap();
        let sample_array = song.sample_array;
        assert_eq!(
            sample_array.len() + SAMPLE_RATE as usize,
            sample_array.capacity()
        );

        let path = String::from("data/capacity_fix.ogg");
        let song = Song::decode(&path).unwrap();
        let sample_array = song.sample_array;
        assert!(sample_array.len() as f32 / sample_array.capacity() as f32 > 0.90);
        assert!(sample_array.len() as f32 / (sample_array.capacity() as f32) < 1.);
    }

    #[test]
    fn test_analysis_distance() {
        let mut a = Song::default();
        a.analysis = vec![
            0.37860596,
            -0.75483,
            -0.85036564,
            -0.6326486,
            -0.77610075,
            0.27126348,
            -1.,
            0.,
            1.,
        ];

        let mut b = Song::default();
        b.analysis = vec![
            0.31255,
            0.15483,
            -0.15036564,
            -0.0326486,
            -0.87610075,
            -0.27126348,
            1.,
            0.,
            1.,
        ];
        assert_eq!(a.distance(&b), 5.986180)
    }

    #[test]
    fn test_analysis_distance_indiscernible() {
        let mut a = Song::default();
        a.analysis = vec![
            0.37860596,
            -0.75483,
            -0.85036564,
            -0.6326486,
            -0.77610075,
            0.27126348,
            -1.,
            0.,
            1.,
        ];

        assert_eq!(a.distance(&a), 0.)
    }

    #[test]
    fn test_decode_errors() {
        assert_eq!(
            Song::decode("nonexistent").unwrap_err(),
            BlissError::DecodingError(String::from(
                "while opening format: ffmpeg::Error(2: No such file or directory)."
            )),
        );
        assert_eq!(
            Song::decode("data/picture.png").unwrap_err(),
            BlissError::DecodingError(String::from("No audio stream found.")),
        );
    }
}

#[cfg(all(feature = "bench", test))]
mod bench {
    extern crate test;
    use crate::Song;
    use test::Bencher;

    #[bench]
    fn bench_resample_multi(b: &mut Bencher) {
        let path = String::from("./data/s32_stereo_44_1_kHz.flac");
        b.iter(|| {
            Song::decode(&path).unwrap();
        });
    }
}
