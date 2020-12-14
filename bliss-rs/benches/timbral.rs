#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::timbral::SpectralDesc;
    use test::Bencher;

    #[bench]
    fn bench_spectral_desc(b: &mut Bencher) {
        let mut spectral_desc = SpectralDesc::new(10);
        let chunk = vec![0.; 512];

        b.iter(|| {
            spectral_desc.do_(&chunk);
        });
    }
}
