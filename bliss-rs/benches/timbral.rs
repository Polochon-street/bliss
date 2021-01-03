#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::timbral::SpectralDesc;
    use bliss_rs::timbral::ZeroCrossingRateDesc;
    use test::Bencher;

    #[bench]
    fn bench_spectral_desc(b: &mut Bencher) {
        let mut spectral_desc = SpectralDesc::new(10);
        let chunk = vec![0.; 512];

        b.iter(|| {
            spectral_desc.do_(&chunk);
        });
    }

    #[bench]
    fn bench_zcr_desc(b: &mut Bencher) {
        let mut zcr_desc = ZeroCrossingRateDesc::new(10);
        let chunk = vec![0.; 512];

        b.iter(|| {
            zcr_desc.do_(&chunk);
        });
    }
}
