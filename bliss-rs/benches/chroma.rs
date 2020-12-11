#![feature(test)]

// TODO use cfg(bench) to make pub functions not-pub depending on context
#[cfg(test)]
mod test {
    extern crate test;
    use bliss_rs::chroma::*;
    use bliss_rs::utils::TEMPLATES_MAJMIN;
    use ndarray::{arr2, Array, Array1, Array2};
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

    #[bench]
    fn bench_pip_track(b: &mut Bencher) {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let spectrum = Array2::<f64>::read_npy(file).unwrap();

        b.iter(|| {
            pip_track(22050, &spectrum, 2048);
        });
    }

    #[bench]
    fn bench_chroma_filter(b: &mut Bencher) {
        b.iter(|| {
            chroma_filter(22050, 2048, 12, -0.1);
        });
    }

    #[bench]
    fn bench_analysis_template_match(b: &mut Bencher) {
        let file = File::open("data/chroma.npy").unwrap();
        let chroma = Array2::<f64>::read_npy(file).unwrap();

        let templates = Array::from_shape_vec((12, 24), TEMPLATES_MAJMIN.to_vec()).unwrap();
        b.iter(|| {
            analysis_template_match(&chroma, &templates, true);
        });
    }

    #[bench]
    fn bench_normalize_feature_sequence(b: &mut Bencher) {
        let array = arr2(&[[0.1, 0.3, 0.4], [1.1, 0.53, 1.01]]);
        b.iter(|| {
            normalize_feature_sequence(&array);
        });
    }

    #[bench]
    fn bench_smooth_downsample_feature_sequence(b: &mut Bencher) {
        let file = File::open("data/chroma.npy").unwrap();
        let chroma = Array2::<f64>::read_npy(file).unwrap();

        b.iter(|| {
            smooth_downsample_feature_sequence(&chroma, 45, 15);
        });
    }

    #[bench]
    fn bench_sort_by_fifths(b: &mut Bencher) {
        let file = File::open("data/filtered_features.npy").unwrap();
        let features = Array2::<f64>::read_npy(file).unwrap();

        b.iter(|| {
            sort_by_fifths(&features);
        });
    }
}
