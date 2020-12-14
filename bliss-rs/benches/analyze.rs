#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::analyze::*;
    use bliss_rs::decode::decode_song;
    use test::Bencher;

    #[bench]
    fn bench_compute_stft(b: &mut Bencher) {
        let song = decode_song("data/piano.flac").unwrap();

        b.iter(|| {
            stft(&song.sample_array, 2048, 512);
        });
    }
}
