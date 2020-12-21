#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::Song;
    use test::Bencher;

    #[bench]
    fn bench_resample_multi(b: &mut Bencher) {
        let path = String::from("./data/s32_stereo_44_1_kHz.flac");
        b.iter(|| {
            Song::decode(&path).unwrap();
        });
    }
}
