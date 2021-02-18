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
use crate::Song;
use crate::SAMPLE_RATE;
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

pub fn push_to_sample_array(frame: &ffmpeg::frame::Audio, sample_array: &mut Vec<f32>) {
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

    pub fn new(path: &str) -> Result<Self, String> {
        // TODO error handling here
        let mut song = Song::decode(&path)?;

        song.analysis = (&song).analyse()?;
        song.sample_array = None;
        Ok(song)
    }

    // TODO write down somewhere that this can be done windows by windows
    pub fn analyse(&self) -> Result<Vec<f32>, String> {
        thread::scope(|s| {
            let child_tempo: thread::ScopedJoinHandle<'_, Result<f32, String>> = s.spawn(|_| {
                let sample_array = self
                    .sample_array
                    .as_ref()
                    .ok_or("Error: tried to analyse an empty song.".to_string())?;
                let mut tempo_desc = BPMDesc::new(self.sample_rate);
                let windows = sample_array
                    .windows(BPMDesc::WINDOW_SIZE)
                    .step_by(BPMDesc::HOP_SIZE);

                for window in windows {
                    tempo_desc.do_(&window);
                }
                Ok(tempo_desc.get_value())
            });

            let child_chroma: thread::ScopedJoinHandle<'_, Result<Vec<f32>, String>> =
                s.spawn(|_| {
                    let sample_array = self
                        .sample_array
                        .as_ref()
                        .ok_or("Error: tried to analyse an empty song.".to_string())?;
                    let mut chroma_desc = ChromaDesc::new(self.sample_rate, 12);
                    chroma_desc.do_(&sample_array);
                    Ok(chroma_desc.get_values())
                });

            let child_timbral: thread::ScopedJoinHandle<'_, Result<(f32, f32, f32), String>> = s
                .spawn(|_| {
                    let sample_array = self
                        .sample_array
                        .as_ref()
                        .ok_or("Error: tried to analyse an empty song.")?;
                    let mut spectral_desc = SpectralDesc::new(self.sample_rate);
                    let windows = sample_array
                        .windows(SpectralDesc::WINDOW_SIZE)
                        .step_by(SpectralDesc::HOP_SIZE);
                    for window in windows {
                        spectral_desc.do_(&window);
                    }
                    let centroid = spectral_desc.get_centroid();
                    let rolloff = spectral_desc.get_rolloff();
                    let flatness = spectral_desc.get_flatness();
                    Ok((centroid, rolloff, flatness))
                });

            let child_zcr: thread::ScopedJoinHandle<'_, Result<f32, String>> = s.spawn(|_| {
                let sample_array = self
                    .sample_array
                    .as_ref()
                    .ok_or("Error: tried to analyse an empty song.")?;
                let mut zcr_desc = ZeroCrossingRateDesc::default();
                zcr_desc.do_(&sample_array);
                Ok(zcr_desc.get_value())
            });

            let child_loudness: thread::ScopedJoinHandle<'_, Result<f32, String>> = s.spawn(|_| {
                let mut loudness_desc = LoudnessDesc::default();
                let sample_array = self
                    .sample_array
                    .as_ref()
                    .ok_or("Error: tried to analyse an empty song.")?;
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

            let mut result = vec![tempo, centroid, zcr, rolloff, flatness, loudness];
            result.extend_from_slice(&chroma);
            Ok(result)
        })
        .unwrap()
    }

    // TODO DRY me
    pub fn decode(path: &str) -> Result<Song, String> {
        ffmpeg::init().map_err(|e| format!("FFmpeg init error: {:?}.", e))?;
        log::set_level(Level::Quiet);
        let mut song = Song {
            path: path.to_string(),
            ..Default::default()
        };
        let mut format = ffmpeg::format::input(&path)
            .map_err(|e| format!("FFmpeg error while opening format: {:?}.", e))?;
        let (mut codec, stream, expected_sample_number) = {
            let stream = format
                .streams()
                .find(|s| s.codec().medium() == ffmpeg::media::Type::Audio)
                .ok_or("No audio stream found.")?;
            stream.codec().set_threading(Config {
                kind: ThreadingType::Frame,
                count: 0,
                safe: true,
            });
            let codec = stream
                .codec()
                .decoder()
                .audio()
                .map_err(|e| format!("FFmpeg error when finding codec: {:?}.", e))?;
            let expected_sample_number = (SAMPLE_RATE as f32 * stream.duration() as f32
                / stream.time_base().denominator() as f32)
                .ceil();
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
            format!(
                "FFmpeg error trying to allocate resampling context: {:?}",
                e
            )
        })?;

        let (tx, rx) = mpsc::channel();
        let child = std_thread::spawn(move || -> Result<Vec<f32>, String> {
            let mut resampled = ffmpeg::frame::Audio::empty();
            for decoded in rx.iter() {
                resample_context
                    .run(&decoded, &mut resampled)
                    .map_err(|e| format!("FFmpeg error while trying to resample song: {:?}", e))?;
                push_to_sample_array(&resampled, &mut sample_array);
            }

            loop {
                match resample_context
                    .flush(&mut resampled)
                    .map_err(|e| format!("FFmpeg error while trying to resample song: {:?}", e))?
                {
                    Some(_) => {
                        push_to_sample_array(&resampled, &mut sample_array);
                    }
                    None => {
                        push_to_sample_array(&resampled, &mut sample_array);
                        break;
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
                    return Err(String::from("Wrong codec opened."))
                }
                Err(Error::Eof) => {
                    println!("Premature EOF reached while decoding.");
                    drop(tx);
                    song.sample_array = Some(child.join().unwrap()?);
                    song.sample_rate = SAMPLE_RATE;
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
                            format!(
                                "Error while sending decoded frame to the resampling thread: {:?}",
                                e
                            )
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
            Err(Error::Other { errno: EINVAL }) => return Err(String::from("Wrong codec opened.")),
            Err(Error::Eof) => {
                println!("Premature EOF reached while decoding.");
                drop(tx);
                song.sample_array = Some(child.join().unwrap()?);
                song.sample_rate = SAMPLE_RATE;
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
                        format!(
                            "Error while sending decoded frame to the resampling thread: {:?}",
                            e
                        )
                    })?;
                }
                Err(_) => break,
            }
        }

        drop(tx);
        song.sample_array = Some(child.join().unwrap()?);
        song.sample_rate = SAMPLE_RATE;
        Ok(song)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ripemd160::{Digest, Ripemd160};

    #[test]
    fn test_analyse() {
        let song = Song::new("data/s16_mono_22_5kHz.flac").unwrap();
        let expected_analysis = vec![
            0.37860596,
            -0.75483,
            -0.85036564,
            -0.6326486,
            -0.77610075,
            0.27126348,
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
        for sample in song.sample_array.unwrap().iter() {
            hasher.update(sample.to_le_bytes().to_vec());
        }

        assert_eq!(expected_hash, hasher.finalize().as_slice());
    }

    #[test]
    fn tags() {
        let song = Song::decode("data/s16_mono_22_5kHz.flac").unwrap();
        assert_eq!(song.artist, "David TMX");
        assert_eq!(song.title, "Renaissance");
        assert_eq!(song.album, "Renaissance");
        assert_eq!(song.track_number, "02");
        assert_eq!(song.genre, "Pop");
    }

    #[test]
    fn resample_multi() {
        let path = String::from("data/s32_stereo_44_1_kHz.flac");
        let expected_hash = [
            0xc5, 0xf8, 0x23, 0xce, 0x63, 0x2c, 0xf4, 0xa0, 0x72, 0x66, 0xbb, 0x49, 0xad, 0x84,
            0xb6, 0xea, 0x48, 0x48, 0x9c, 0x50,
        ];
        _test_decode(&path, &expected_hash);
    }

    #[test]
    fn resample_stereo() {
        let path = String::from("data/s16_stereo_22_5kHz.flac");
        let expected_hash = [
            0x24, 0xed, 0x45, 0x58, 0x06, 0xbf, 0xfb, 0x05, 0x57, 0x5f, 0xdc, 0x4d, 0xb4, 0x9b,
            0xa5, 0x2b, 0x05, 0x56, 0x10, 0x4f,
        ];
        _test_decode(&path, &expected_hash);
    }

    #[test]
    fn decode_mono() {
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
    fn test_dont_panic_no_channel_layout() {
        let path = String::from("data/no_channel.wav");
        Song::decode(&path).unwrap();
    }

    #[test]
    fn test_decode_right_capacity_vec() {
        let path = String::from("data/s16_mono_22_5kHz.flac");
        let song = Song::decode(&path).unwrap();
        let sample_array = song.sample_array.unwrap();
        assert_eq!(sample_array.len(), sample_array.capacity());

        let path = String::from("data/s32_stereo_44_1_kHz.flac");
        let song = Song::decode(&path).unwrap();
        let sample_array = song.sample_array.unwrap();
        assert_eq!(sample_array.len(), sample_array.capacity());

        // Not 100% sure that the number of samples for ogg is known
        // precisely in advance
        let path = String::from("data/capacity_fix.ogg");
        let song = Song::decode(&path).unwrap();
        let sample_array = song.sample_array.unwrap();
        assert!(sample_array.len() as f32 / sample_array.capacity() as f32 > 0.95);
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
}
