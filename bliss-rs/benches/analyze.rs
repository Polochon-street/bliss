#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::utils::*;
    use bliss_rs::Song;
    use ndarray::Array;
    use test::Bencher;

    #[bench]
    fn bench_compute_stft(b: &mut Bencher) {
        let signal = Song::decode("data/piano.flac")
            .unwrap()
            .sample_array
            .unwrap();

        b.iter(|| {
            stft(&signal, 2048, 512);
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
