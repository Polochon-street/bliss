use std::env;
use bliss_rs::Song;

/**
 * Simple utility to print distance between two songs according to bliss.
 *
 * Takes two file paths, and analyze the corresponding songs, printing
 * the distance between the two files according to bliss.
 */
fn main() -> Result<(), String> {
    let mut paths = env::args().skip(1).take(2);

    let first_path = paths.next().ok_or("Help: ./distance <song1> <song2>")?;
    let second_path = paths.next().ok_or("Help: ./distance <song1> <song2>")?;

    let song1 = Song::new(&first_path)?;
    let song2 = Song::new(&second_path)?;

    println!("d({}, {}) = {}", song1.path, song2.path, song1.analysis.distance(&song2.analysis));
    Ok(())
}
