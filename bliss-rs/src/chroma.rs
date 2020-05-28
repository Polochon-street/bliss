#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use crate::utils::hz_to_octs;

// chroma(22050, n_fft=5, n_chroma=12)
pub fn chroma_filter(sample_rate: u32, n_fft: u32, n_chroma: u32, tuning: f64) -> Vec<Vec<f32>> {
    let step = sample_rate as f64 / n_fft as f64;
    let ctroct = 5.0;
    let octwidth = 2;

    // [4410.0, 8820.0, 13230.0, 17640.0]
    let frequencies = (1..n_fft).map(|i| i as f64 * step).collect::<Vec<f64>>();

    // [87.90243872,  99.90243872, 106.92198873, 111.90243872]
    let temp_freq_bins = hz_to_octs(&frequencies, tuning, n_chroma)
        .iter()
        .map(|i| i * n_chroma as f64)
        .collect::<Vec<f64>>();
    // [69.90243872,  87.90243872,  99.90243872, 106.92198873, 111.90243872]
    let mut freq_bins = vec![temp_freq_bins[0] - 1.5 * n_chroma as f64];
    freq_bins.extend_from_slice(temp_freq_bins.as_slice());

    // [18., 12., 7.01955001, 4.98044999, 1.]
    let mut binwidth_bin = (&freq_bins[1..freq_bins.len()])
        .iter()
        .zip(&freq_bins[0..freq_bins.len() - 1])
        .map(|(x, y)| (x - y).max(1.0))
        .collect::<Vec<f64>>();
    binwidth_bin.extend_from_slice(&[1.0]);

    // [[69.90243872, 87.90243872, 99.90243872, 106.92198873, 111.90243872]
    // ...
    // [58.90243872, 76.90243872, 88.90243872, 95.92198873, 100.90243872]]
    let d = (0..n_chroma)
        .map(|i| freq_bins.iter().map(|f| f - i as f64).collect::<Vec<f64>>())
        .collect::<Vec<Vec<f64>>>();

    let n_chroma2 = (n_chroma as f64 / 2.0).round() as u32;

    // [[-2.09756128,  3.90243872,  3.90243872, -1.07801127,  3.90243872]
    // ...
    // [-1.09756128,  4.90243872,  4.90243872, -0.07801127,  4.90243872]]
    let d = d
        .iter()
        .map(|s| {
            s.iter()
                .map(|v| {
                    (v + n_chroma2 as f64 + 10. * n_chroma as f64) % n_chroma as f64
                        - n_chroma2 as f64
                })
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<Vec<f64>>>();

    // [[9.73206457e-01, 8.09357725e-01, 5.38948409e-01, 9.10555920e-01, 5.91880948e-14],
    // ...
    // [9.92591525e-01, 7.16193969e-01, 3.76996585e-01, 9.99509430e-01, 1.33172632e-21]]
    let wts = d
        .iter()
        .map(|s| {
            s.iter()
                .zip(&binwidth_bin)
                .map(|(v, b)| (-0.5 * ((2.0 * v / b) as f64).powf(2.0)).exp())
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<Vec<f64>>>();

    // Normalize by computing the l2-norm over the columns
    let mut length = vec![0.; n_fft as usize];
    for i in 0..(n_fft as usize) {
        let mut vec = vec![0.; n_fft as usize];
        for x in &wts {
            vec.push(x[i].powf(2.0));
        }
        length[i] = vec.iter().map(|x| x).sum::<f64>().sqrt();
    }

    // [3.01362739e-01, 2.70504696e-01, 2.17863058e-01, 4.33578438e-01, 5.89176039e-14]
    // ...
    // [3.07365511e-01, 2.39367373e-01, 1.52396088e-01, 4.75935336e-01, 1.32564030e-21]
    let wts = wts
        .iter()
        .map(|s| {
            s.iter()
                .enumerate()
                .map(|(i, v)| v / (&length)[i])
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<Vec<f64>>>();

    // [0.91840203, 0.50873844, 0.25104525, 0.14790657, 0.09647968]
    let scaling = &freq_bins
        .iter()
        .map(|f| (-0.5 * (((f / n_chroma as f64 - ctroct) / octwidth as f64).powf(2.0))).exp())
        .collect::<Vec<f64>>();

    // [[2.76772150e-01, 1.37616138e-01, 5.46934868e-02, 6.41290987e-02, 5.68435153e-15]
    // ...
    // [2.82285109e-01, 1.21775385e-01, 3.82583145e-02, 7.03939621e-02, 1.27897351e-22]]
    let mut wts = wts
        .iter()
        .map(|s| {
            s.iter()
                .zip(scaling)
                .map(|(v, f)| v * f)
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<Vec<f64>>>();
    // "if base_c"
    // [[2.42245481e-01, 1.68118894e-01, 9.81821393e-02, 1.84251933e-02, 1.88395920e-02]
    // ...
    // [2.56393048e-01, 1.61695478e-01, 8.76171131e-02, 3.28090351e-02, 6.89899611e-05]]
    wts.rotate_left(3);

    let non_aliased = 1 + n_fft / 2;

    wts.iter()
        .map(|s| {
            s.iter()
                .take(non_aliased as usize)
                .map(|f| *f as f32)
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<Vec<f32>>>()
}

mod test {
    use super::*;

    #[test]
    fn test_generate_chroma() {
        let expected_chroma: Vec<Vec<f32>> = vec![
            vec![0.24224548, 0.16811889, 0.09818214],
            vec![0.22936151, 0.17000881, 0.10144266],
            vec![0.24518087, 0.16721014, 0.09663921],
            vec![0.25887551, 0.15995214, 0.08488495],
            vec![0.2699813, 0.14881742, 0.06874682],
            vec![0.27810881, 0.13466469, 0.05133566],
            vec![0.28296593, 0.11851955, 0.03534518],
            vec![0.28437531, 0.10480594, 0.0246752],
            vec![0.28228511, 0.12177538, 0.03825831],
            vec![0.27677215, 0.13761614, 0.05469349],
            vec![0.26803725, 0.151257, 0.07209248],
            vec![0.25639305, 0.16169548, 0.08761711],
        ];
        let filter = chroma_filter(22050, 5, 12, 0.0);

        for (column1, column2) in expected_chroma.iter().zip(filter.iter()) {
            for (val1, val2) in column1.iter().zip(column2) {
                assert!(0.0001 > (val1 - val2).abs());
            }
        }
    }
}