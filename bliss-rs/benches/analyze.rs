#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::analyze::*;
    use bliss_rs::decode::decode_song;
    use ndarray::Array;
    use test::Bencher;

    #[bench]
    fn bench_compute_stft(b: &mut Bencher) {
        let song = decode_song("data/piano.flac").unwrap();

        b.iter(|| {
            stft(&song.sample_array, 2048, 512);
        });
    }

    #[bench]
    fn bench_reflect_pad(b: &mut Bencher) {
        let array = Array::range(0., 1000000., 1.);

        b.iter(|| {
            reflect_pad(array.as_slice().unwrap(), 3);
        });
    }
}
