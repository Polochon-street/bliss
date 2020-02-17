extern crate ffmpeg4 as ffmpeg;

use ffmpeg::util::format::sample::{Sample, Type};
use ffmpeg::{util};
use ripemd160::{Digest, Ripemd160};

const CHANNELS: u16 = 1;

struct Song {
    sample_array: Vec<u8>,
}

fn decode_song(path: &str) -> Song {
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

    let mut decoded = ffmpeg::frame::Audio::empty();
    for (s, packet) in format.packets() {
        if s.index() != stream {
            continue;
        }

        match codec.decode(&packet, &mut decoded) {
            Ok(true) => {
                // Account for the padding
                let actual_size = util::format::sample::Buffer::size(
                    Sample::I16(Type::Packed),
                    CHANNELS,
                    decoded.samples(),
                    false,
                );
                sample_array.extend(
                    decoded
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
    Song { sample_array }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_s16_mono() {
        let path = String::from("song_mono.flac");
        let song = decode_song(&path);
        let mut hasher = Ripemd160::new();

        hasher.input(song.sample_array);
        let result: [u8; 20] = [
            0x87, 0x8f, 0xfd, 0x28, 0x75, 0xad, 0x8a, 0x4f, 0x26, 0x1e, 0x09, 0xad, 0x6f, 0x27, 0x3b,
            0x6f, 0xd1, 0x08, 0x73, 0x0c,
        ];
        assert_eq!(result, hasher.result().as_slice());
    }

}
