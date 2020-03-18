use bliss_rs::decode::decode_song;

fn main() {
    decode_song("data/s16_stereo_22_5kHz.flac").unwrap();
}
