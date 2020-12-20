#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::utils::*;
    use bliss_rs::Song;
    use ndarray::{Array, Array1};
    use test::Bencher;

    #[bench]
    fn bench_convolve(b: &mut Bencher) {
        let input: Array1<f64> = Array::range(0., 1000., 0.5);
        let kernel: Array1<f64> = Array::ones(100);

        b.iter(|| {
            convolve(&input, &kernel);
        });
    }

    #[bench]
    fn bench_compute_stft(b: &mut Bencher) {
        let song = Song::decode("data/piano.flac").unwrap();

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
