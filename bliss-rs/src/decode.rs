extern crate ffmpeg4 as ffmpeg;

use ffmpeg::util;
use ffmpeg::util::format::sample::{Sample, Type};

use super::{Song, CHANNELS, SAMPLE_RATE};

fn push_to_sample_array(frame: ffmpeg::frame::Audio, sample_array: &mut Vec<i16>) {
    // Account for the padding
    let actual_size = util::format::sample::Buffer::size(
        Sample::I16(Type::Packed),
        CHANNELS,
        frame.samples(),
        false,
    );
    let i16_frame: Vec<i16> = frame.data(0)[..actual_size]
        .chunks_exact(2)
        .map(|x| {
            let mut a: [u8; 2] = [0; 2];
            a.copy_from_slice(x);
            i16::from_le_bytes(a)
        })
        .collect();
    sample_array.extend_from_slice(&i16_frame);
}

pub fn decode_song(path: &str) -> Result<Song, String> {
    ffmpeg::init().map_err(|e| format!("FFmpeg init error: {:?}", e))?;

    let mut song = Song::default();
    let mut sample_array: Vec<i16> = Vec::new();
    let mut format = ffmpeg::format::input(&path)
        .map_err(|e| format!("FFmpeg error while opening format: {:?}", e))?;
    let (mut codec, stream) = {
        let stream = format
            .streams()
            .find(|s| s.codec().medium() == ffmpeg::media::Type::Audio)
            .expect("no audio stream in the file");

        let codec = stream.codec().decoder().audio().expect("no audio stream");

        (codec, stream.index())
    };

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

    let mut resample_context = ffmpeg::software::resampling::context::Context::get(
        codec.format(),
        codec.channel_layout(),
        codec.rate(),
        Sample::I16(Type::Packed),
        ffmpeg::util::channel_layout::ChannelLayout::MONO,
        SAMPLE_RATE,
    )
    .map_err(|e| {
        format!(
            "FFmpeg error trying to allocate resampling context: {:?}",
            e
        )
    })?;

    let mut decoded = ffmpeg::frame::Audio::empty();
    for (s, packet) in format.packets() {
        if s.index() != stream {
            continue;
        }

        match codec.decode(&packet, &mut decoded) {
            Ok(true) => {
                let mut resampled = ffmpeg::frame::Audio::empty();
                resample_context
                    .run(&decoded, &mut resampled)
                    .map_err(|e| format!("FFmpeg error while trying to resample song: {:?}", e))?;
                push_to_sample_array(resampled, &mut sample_array);
            }
            Ok(false) => (),
            Err(error) => println!("Could not decode packet: {}", error),
        }
    }

    // Flush the stream
    let packet = ffmpeg::codec::packet::Packet::empty();
    loop {
        match codec.decode(&packet, &mut decoded) {
            Ok(true) => {
                let mut resampled = ffmpeg::frame::Audio::empty();
                resample_context
                    .run(&decoded, &mut resampled)
                    .map_err(|e| format!("FFmpeg error while trying to resample song: {:?}", e))?;
                push_to_sample_array(resampled, &mut sample_array);
            }
            Ok(false) => break,
            Err(error) => println!("Could not decode packet: {}", error),
        };
    }

    loop {
        let mut resampled = ffmpeg::frame::Audio::empty();
        match resample_context
            .flush(&mut resampled)
            .map_err(|e| format!("FFmpeg error while trying to resample song: {:?}", e))?
        {
            Some(_) => {
                push_to_sample_array(resampled, &mut sample_array);
            }
            None => {
                if resampled.samples() > 0 {
                    push_to_sample_array(resampled, &mut sample_array);
                }
                break;
            }
        };
    }
    song.sample_array = sample_array;
    Ok(song)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ripemd160::{Digest, Ripemd160};

    fn _test_decode_song(path: &str, expected_hash: &[u8]) {
        let song = decode_song(path).unwrap();
        let mut hasher = Ripemd160::new();
        for sample in song.sample_array.iter() {
            hasher.input(sample.to_le_bytes().to_vec());
        }

        assert_eq!(expected_hash, hasher.result().as_slice());
    }

    #[test]
    fn tags() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
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
