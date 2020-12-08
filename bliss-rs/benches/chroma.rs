#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::chroma::*;
    use ndarray::{Array1, Array2};
    use ndarray_npy::ReadNpyExt;
    use std::fs::File;
    use test::Bencher;

    #[bench]
    fn bench_estimate_tuning(b: &mut Bencher) {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let arr = Array2::<f64>::read_npy(file).unwrap();

        b.iter(|| {
            estimate_tuning(22050, &arr, 2048, 0.01, 12);
        });
    }

    #[bench]
    fn bench_pitch_tuning(b: &mut Bencher) {
        let file = File::open("data/pitch-tuning.npy").unwrap();
        let pitch = Array1::<f64>::read_npy(file).unwrap();
        b.iter(|| {
            pitch_tuning(&mut pitch.to_owned(), 0.05, 12);
        });
    }
}
