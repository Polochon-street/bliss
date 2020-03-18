use bliss_rs::decode::decode_song;
use ripemd160::{Digest, Ripemd160};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let song = decode_song(&path).unwrap();
    let mut hasher = Ripemd160::new();
    for sample in song.sample_array.iter() {
        hasher.input(sample.to_le_bytes().to_vec());
    }
    println!("{:02x?}", hasher.result().as_slice());
}
