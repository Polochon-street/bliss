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
use crate::SAMPLE_RATE;
use crate::{Analysis, Song};
use crossbeam::thread;
use ffmpeg::util;
use ffmpeg::util::format::sample::{Sample, Type};
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
    pub fn new(path: &str) -> Result<Self, String> {
        // TODO error handling here
        let mut song = Song::decode(&path)?;

        song.analysis = (&song).analyze();
        Ok(song)
    }

    pub fn analyze(&self) -> Analysis {
        thread::scope(|s| {
            let child_chroma = s.spawn(|_| {
                let mut chroma_desc = ChromaDesc::new(self.sample_rate, 12);
                chroma_desc.do_(&self.sample_array);
                chroma_desc.get_values()
            });

            // These descriptors can be streamed
            let child_timbral = s.spawn(|_| {
                let mut spectral_desc = SpectralDesc::new(self.sample_rate);
                let mut zcr_desc = ZeroCrossingRateDesc::default();
                let windows = self
                    .sample_array
                    .windows(SpectralDesc::WINDOW_SIZE)
                    .step_by(SpectralDesc::HOP_SIZE);
                for window in windows {
                    spectral_desc.do_(&window);
                    zcr_desc.do_(&window);
                }
                let centroid = spectral_desc.get_centroid();
                let rolloff = spectral_desc.get_rolloff();
                let flatness = spectral_desc.get_flatness();
                let zcr = zcr_desc.get_value();
                (centroid, rolloff, flatness, zcr)
            });

            let child_tempo = s.spawn(|_| {
                let mut tempo_desc = BPMDesc::new(self.sample_rate);
                let windows = self
                    .sample_array
                    .windows(BPMDesc::WINDOW_SIZE)
                    .step_by(BPMDesc::HOP_SIZE);

                for window in windows {
                    tempo_desc.do_(&window);
                }
                tempo_desc.get_value()
            });

            let child_loudness = s.spawn(|_| {
                let mut loudness_desc = LoudnessDesc::default();
                let windows = self.sample_array.chunks(LoudnessDesc::WINDOW_SIZE);

                for window in windows {
                    loudness_desc.do_(&window);
                }
                loudness_desc.get_value()
            });

            // Non-streaming approach for that one
            let (is_major, fifth) = child_chroma.join().unwrap();
            let (centroid, rolloff, flatness, zcr) = child_timbral.join().unwrap();
            let tempo = child_tempo.join().unwrap();
            let loudness = child_loudness.join().unwrap();

            Analysis {
                tempo,
                spectral_centroid: centroid,
                zero_crossing_rate: zcr,
                spectral_rolloff: rolloff,
                spectral_flatness: flatness,
                loudness,
                is_major,
                fifth,
            }
        })
        .unwrap()
    }

    pub fn decode(path: &str) -> Result<Song, String> {
        ffmpeg::init().map_err(|e| format!("FFmpeg init error: {:?}.", e))?;

        let mut song = Song::default();
        song.path = path.to_string();
        let mut format = ffmpeg::format::input(&path)
            .map_err(|e| format!("FFmpeg error while opening format: {:?}.", e))?;
        let (mut codec, stream, duration) = {
            let stream = format
                .streams()
                .find(|s| s.codec().medium() == ffmpeg::media::Type::Audio)
                .ok_or("No audio stream found.")?;

            let codec = stream
                .codec()
                .decoder()
                .audio()
                .map_err(|e| format!("FFmpeg error when finding codec: {:?}.", e))?;
            (codec, stream.index(), stream.duration())
        };
        let mut sample_array: Vec<f32> = Vec::with_capacity(duration as usize);

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

        // TODO handle WAV without a channel layout set (cf bruiblan.wav)
        let mut resample_context = ffmpeg::software::resampling::context::Context::get(
            codec.format(),
            codec.channel_layout(),
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
        let mut decoded = ffmpeg::frame::Audio::empty();
        for (s, packet) in format.packets() {
            if s.index() != stream {
                continue;
            }
            codec.send_packet(&packet).unwrap();
            while codec.receive_frame(&mut decoded).is_ok() {
                tx.send(decoded.clone()).map_err(|e| {
                    format!(
                        "Error while sending decoded frame to the resampling thread: {:?}",
                        e
                    )
                })?;
            }
        }

        // Flush the stream
        // TODO check that it's still how to do this
        let packet = ffmpeg::codec::packet::Packet::empty();
        loop {
            codec.send_packet(&packet).unwrap();
            while codec.receive_frame(&mut decoded).is_ok() {
                tx.send(decoded.clone()).map_err(|e| {
                    format!(
                        "Error while sending decoded frame to the resampling thread: {:?}",
                        e
                    )
                })?;
            }
            break;
        }

        drop(tx);
        song.sample_array = child.join().unwrap()?;
        song.sample_rate = SAMPLE_RATE;
        Ok(song)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ripemd160::{Digest, Ripemd160};
    use std::f32::consts::PI;

    #[test]
    fn test_analyze() {
        let song = Song::decode("data/s16_mono_22_5kHz.flac").unwrap();
        let expected_analysis = Analysis {
            tempo: 0.37860596,
            spectral_centroid: -0.75483,
            zero_crossing_rate: -0.85036564,
            spectral_rolloff: -0.6326486,
            spectral_flatness: -0.77610075,
            loudness: 0.27126348,
            is_major: -1.,
            fifth: (f32::cos(5. * PI / 3.), f32::sin(5. * PI / 3.)),
        };
        assert!(expected_analysis.approx_eq(&song.analyze()));
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
}