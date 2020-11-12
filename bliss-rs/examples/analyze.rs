use bliss_rs::analyze::decode_and_analyze;
use std::env;

/**
 * Simple utility to print the result or the field of an Analysis.
 *
 * Takes a list of files to analyze and outputs a field // the result of
 * the corresponding Analysis, depending of what you chose to `println!` below.
 */
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    for path in &args {
        match decode_and_analyze(&path) {
            Ok(song) => println!(
                "{}: {}",
                path, song.analysis.tempo,
            ),
            Err(e) => println!("{}: {}", path, e),
        }
    }
}
