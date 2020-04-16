extern crate ffmpeg4 as ffmpeg;

use ffmpeg::util;
use ffmpeg::util::format::sample::{Sample, Type};

use super::{Song, CHANNELS, SAMPLE_RATE};

fn push_to_sample_array(frame: ffmpeg::frame::Audio, sample_array: &mut Vec<f32>) {
    if frame.samples() == 0 {
        return
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

// TODO maybe an impl on Song?
pub fn decode_song(path: &str) -> Result<Song, String> {
    ffmpeg::init().map_err(|e| format!("FFmpeg init error: {:?}.", e))?;

    let mut song = Song::default();
    song.path = path.to_string();
    let mut sample_array: Vec<f32> = Vec::new();
    let mut format = ffmpeg::format::input(&path)
        .map_err(|e| format!("FFmpeg error while opening format: {:?}.", e))?;
    let (mut codec, stream) = {
        let stream = format
            .streams()
            .find(|s| s.codec().medium() == ffmpeg::media::Type::Audio)
            .ok_or("No audio stream found.")?;

        let codec = stream
            .codec()
            .decoder()
            .audio()
            .map_err(|e| format!("FFmpeg error when finding codec: {:?}.", e))?;

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
                push_to_sample_array(resampled, &mut sample_array);
                break;
            }
        };
    }
    song.sample_array = sample_array;
    song.sample_rate = SAMPLE_RATE;
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
            0xc5, 0xf8, 0x23, 0xce, 0x63, 0x2c, 0xf4, 0xa0, 0x72, 0x66, 0xbb, 0x49, 0xad, 0x84,
            0xb6, 0xea, 0x48, 0x48, 0x9c, 0x50,
        ];
        _test_decode_song(&path, &expected_hash);
    }

    #[test]
    fn resample_stereo() {
        let path = String::from("data/s16_stereo_22_5kHz.flac");
        let expected_hash = [
            0x24, 0xed, 0x45, 0x58, 0x06, 0xbf, 0xfb, 0x05, 0x57, 0x5f, 0xdc, 0x4d, 0xb4, 0x9b,
            0xa5, 0x2b, 0x05, 0x56, 0x10, 0x4f,
        ];
        _test_decode_song(&path, &expected_hash);
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
        _test_decode_song(&path, &expected_hash);
    }
}
