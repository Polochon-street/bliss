#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::utils::*;
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
}
