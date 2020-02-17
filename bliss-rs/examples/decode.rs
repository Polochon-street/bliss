use bliss_rs::decode_song;
use ripemd160::{Digest, Ripemd160};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let song = decode_song(&path);
    let mut hasher = Ripemd160::new();

    hasher.input(song.sample_array);
    //println!("{:?}", hasher.result().as_slice());
    println!("{:02x?}", hasher.result().as_slice());
}
