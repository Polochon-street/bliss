extern crate ffmpeg4 as ffmpeg;

use ffmpeg::util;
use ffmpeg::util::format::sample::{Sample, Type};

const CHANNELS: u16 = 1;

pub struct Song {
    pub sample_array: Vec<u8>,
}

// TODO put stuff here in functions
// TODO decode tags to complete the struct Song
pub fn decode_song(path: &str) -> Song {
    ffmpeg::init().unwrap();

    let mut sample_array: Vec<u8> = Vec::new();
    let mut format = ffmpeg::format::input(&path).unwrap();
    let (mut codec, stream) = {
        let stream = format
            .streams()
            .find(|s| s.codec().medium() == ffmpeg::media::Type::Audio)
            .expect("no audio stream in the file");

        let codec = stream.codec().decoder().audio().expect("no audio stream");

        (codec, stream.index())
    };

    let mut resample_context = ffmpeg::software::resampling::context::Context::get(
        codec.format(),
        codec.channel_layout(),
        codec.rate(),
        Sample::I16(Type::Packed),
        ffmpeg::util::channel_layout::ChannelLayout::MONO,
        22050,
    )
    .unwrap();

    let mut decoded = ffmpeg::frame::Audio::empty();
    for (s, packet) in format.packets() {
        if s.index() != stream {
            continue;
        }

        match codec.decode(&packet, &mut decoded) {
            Ok(true) => {
                let mut resampled = ffmpeg::frame::Audio::empty();
                resample_context.run(&decoded, &mut resampled).unwrap();
                // Account for the padding
                let actual_size = util::format::sample::Buffer::size(
                    Sample::I16(Type::Packed),
                    CHANNELS,
                    resampled.samples(),
                    false,
                );
                sample_array.extend(
                    resampled
                        .data(0)
                        .iter()
                        .take(actual_size)
                        .collect::<Vec<&u8>>(),
                );
            }
            Ok(false) => (),
            Err(error) => println!("Could not decode packet: {}", error),
        }
    }

    let packet = ffmpeg::codec::packet::Packet::empty();
    loop {
        match codec.decode(&packet, &mut decoded) {
            Ok(true) => (), // TODO do something here
            Ok(false) => break,
            Err(error) => println!("Could not decode packet: {}", error),
        };
    }

    loop {
        let mut resampled = ffmpeg::frame::Audio::empty();
        match resample_context.flush(&mut resampled).unwrap() {
            Some(delay) => {
                let actual_size = util::format::sample::Buffer::size(
                    Sample::I16(Type::Packed),
                    CHANNELS,
                    resampled.samples(),
                    false,
                );
                sample_array.extend(
                    resampled
                        .data(0)
                        .iter()
                        .take(actual_size)
                        .collect::<Vec<&u8>>(),
                );
            }
            None => {
                if resampled.samples() > 0 {
                    let actual_size = util::format::sample::Buffer::size(
                        Sample::I16(Type::Packed),
                        CHANNELS,
                        resampled.samples(),
                        false,
                    );
                    sample_array.extend(
                        resampled
                            .data(0)
                            .iter()
                            .take(actual_size)
                            .collect::<Vec<&u8>>(),
                    );
                }
                break;
            }
        };
    }
    Song { sample_array }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ripemd160::{Digest, Ripemd160};

    fn _test_decode_song(path: &str, expected_hash: &[u8]) {
        let song = decode_song(path);
        let mut hasher = Ripemd160::new();
        hasher.input(song.sample_array);

        assert_eq!(expected_hash, hasher.result().as_slice());
    }

    #[test]
    fn resample_multi() {
        let path = String::from("data/s32_stereo_44_1_kHz.flac");
        let expected_hash = [
            0x5c, 0x54, 0x77, 0x41, 0x4a, 0x52, 0xae, 0x68, 0xbb, 0xf7, 0x24, 0xff, 0x57, 0x75,
            0x93, 0xd2, 0xad, 0x67, 0xf1, 0x48,
        ];
        _test_decode_song(&path, &expected_hash);
    }

    #[test]
    fn resample_stereo() {
        let path = String::from("data/s16_stereo_22_5kHz.flac");
        let expected_hash = [
            0x7c, 0x24, 0x25, 0x21, 0x5f, 0x5b, 0xb9, 0x0c, 0xd3, 0xab, 0x0f, 0xed, 0x01, 0xe1,
            0xcd, 0x3a, 0x8b, 0xf7, 0x93, 0xf2,
        ];
        _test_decode_song(&path, &expected_hash);
    }

    #[test]
    fn decode_mono() {
        let path = String::from("data/s16_mono_22_5kHz.flac");
        let expected_hash = [
            0x87, 0x8f, 0xfd, 0x28, 0x75, 0xad, 0x8a, 0x4f, 0x26, 0x1e, 0x09, 0xad, 0x6f, 0x27,
            0x3b, 0x6f, 0xd1, 0x08, 0x73, 0x0c,
        ];
        _test_decode_song(&path, &expected_hash);
    }
}
