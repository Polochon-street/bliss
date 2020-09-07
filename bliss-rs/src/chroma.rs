#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

extern crate ndarray;
extern crate ndarray_npy;

use crate::utils::{hz_to_octs, median};
use aubio_rs::vec::CVec;
use aubio_rs::PVoc;

pub struct ChromaDesc {
    sample_rate: u32,
    n_chroma: u32,
    phase_vocoder: PVoc,
    // 12 * nb_fft chroma values
    values_chroma: Vec<Vec<f32>>,
    collected_spectra: Vec<Vec<f32>>,
}

impl ChromaDesc {
    pub const WINDOW_SIZE: usize = 2048;
    pub const HOP_SIZE: usize = 512;

    pub fn new(sample_rate: u32, n_chroma: u32) -> ChromaDesc {
        ChromaDesc {
            sample_rate,
            n_chroma,
            values_chroma: Vec::new(),
            phase_vocoder: PVoc::new(ChromaDesc::WINDOW_SIZE, ChromaDesc::HOP_SIZE).unwrap(),
            collected_spectra: vec![],
        }
    }

    pub fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; ChromaDesc::WINDOW_SIZE + 2];
        self.phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let cvec: CVec = fftgrain.as_slice().into();
        let norm: Vec<f32> = cvec.norm().to_vec();
        self.collected_spectra.push(norm);

        if self.collected_spectra.len() >= 100 {
            let mut transpose_collected_spectra =
                vec![vec![0.; self.collected_spectra.len()]; self.collected_spectra[0].len()];
            for i in 0..self.collected_spectra.len() {
                for j in 0..self.collected_spectra[0].len() {
                    transpose_collected_spectra[j][i] = self.collected_spectra[i][j];
                }
            }

            let chroma = chroma_stft(
                self.sample_rate,
                &transpose_collected_spectra,
                ChromaDesc::WINDOW_SIZE as u32,
                self.n_chroma,
            );
            self.values_chroma.extend(chroma);
            self.collected_spectra = vec![];
        }
    }

    pub fn finish(&mut self) {
        if self.collected_spectra.len() <= 2 {
            return;
        }
        let mut transpose_collected_spectra =
            vec![vec![0.; self.collected_spectra.len()]; self.collected_spectra[0].len()];
        for i in 0..self.collected_spectra.len() {
            for j in 0..self.collected_spectra[0].len() {
                transpose_collected_spectra[j][i] = self.collected_spectra[i][j];
            }
        }
        let chroma = chroma_stft(
            self.sample_rate,
            &transpose_collected_spectra,
            ChromaDesc::WINDOW_SIZE as u32,
            self.n_chroma,
        );
        self.values_chroma.extend(chroma);
        self.collected_spectra = vec![];
    }

    // Doesn't make any sense now! Only here for the test
    pub fn get_value(&mut self) -> f32 {
        self.values_chroma
            .iter()
            .map(|s| s.iter().sum::<f32>())
            .sum()
    }
}

// All the functions below are more than heavily inspired from
// librosa's code: https://github.com/librosa/librosa/blob/main/librosa/feature/spectral.py#L1165
// chroma(22050, n_fft=5, n_chroma=12)
fn chroma_filter(sample_rate: u32, n_fft: u32, n_chroma: u32, tuning: f64) -> Vec<Vec<f32>> {
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

fn pip_track(
    sample_rate: u32,
    spectrum: &[Vec<f32>],
    n_fft: u32,
) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
    let fmin = 150.0_f64;
    let fmax = 4000.0_f64.min(sample_rate as f64 / 2.0);
    let threshold = 0.1;

    let step = sample_rate as f64 / (2.0 * (n_fft / 2) as f64);
    // [0.0, 10.7666016e, 21.5332031 (...) 1100.34668, 1101.42334, 11025.0000]
    let fft_freqs = (0..(1 + n_fft / 2))
        .map(|i| i as f64 * step)
        .collect::<Vec<f64>>();

    let t_avg = spectrum[2..spectrum.len()]
        .iter()
        .zip(spectrum[0..spectrum.len() - 2].iter())
        .map(|(c1, c2)| {
            c1.iter()
                .zip(c2.iter())
                .map(|(x, y)| (x - y) / 2.0)
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<Vec<f32>>>();

    let t_shift = spectrum[1..spectrum.len()]
        .iter()
        .zip(spectrum[2..spectrum.len()].iter())
        .zip(spectrum[0..spectrum.len() - 2].iter())
        .map(|((c1, c2), c3)| {
            c1.iter()
                .zip(c2.iter())
                .zip(c3.iter())
                .map(|((x, y), z)| 2. * x - y - z)
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<Vec<f32>>>();

    let t_shift = t_avg
        .iter()
        .zip(t_shift.iter())
        .map(|(c1, c2)| {
            c1.iter()
                .zip(c2.iter())
                .map(|(&x, &y)| {
                    if y.abs() < f32::MIN_POSITIVE {
                        x
                    } else {
                        x / y
                    }
                })
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<Vec<f32>>>();

    let mut avg = vec![vec![0. as f32; spectrum[0].len()]];
    avg.extend(t_avg);
    avg.push(vec![0. as f32; spectrum[0].len()]);

    let mut shift = vec![vec![0. as f32; spectrum[0].len()]];
    shift.extend(t_shift);
    shift.push(vec![0. as f32; spectrum[0].len()]);

    let dskew = &mut avg
        .iter()
        .zip(shift.iter())
        .map(|(c1, c2)| {
            c1.iter()
                .zip(c2.iter())
                .map(|(x, y)| 0.5 * x * y)
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<Vec<f32>>>();

    let freq_mask = fft_freqs
        .iter()
        .map(|&f| (fmin <= f) && (f < fmax))
        .collect::<Vec<bool>>();

    let mut ref_value = Vec::with_capacity(spectrum[0].len());
    for i in 0..spectrum[0].len() {
        ref_value.push(
            threshold
                * spectrum
                    .iter()
                    .map(|c| c[i])
                    .max_by(|x, y| x.partial_cmp(y).unwrap())
                    .unwrap(),
        );
    }
    let mut idx = Vec::new();

    for j in 0..spectrum[0].len() {
        for i in 0..spectrum.len() {
            if i == 0 {
                {}
            } else if i + 1 >= spectrum.len() {
                if spectrum[i - 1][j] < spectrum[i][j]
                    && spectrum[i][j] > ref_value[j]
                    && freq_mask[i]
                {
                    idx.push((i, j));
                }
            } else if spectrum[i - 1][j] < spectrum[i][j]
                && spectrum[i + 1][j] <= spectrum[i][j]
                && spectrum[i][j] > ref_value[j]
                && freq_mask[i]
            {
                idx.push((i, j));
            }
        }
    }

    let mut pitches = vec![vec![0.; spectrum[0].len()]; spectrum.len()];
    let mut mags = vec![vec![0.; spectrum[0].len()]; spectrum.len()];

    for (i, j) in idx {
        pitches[i][j] = (i as f32 + shift[i][j]) * sample_rate as f32 / n_fft as f32;
        mags[i][j] = spectrum[i][j] + dskew[i][j];
    }
    (pitches, mags)
}

fn pitch_tuning(frequencies: &[f32], resolution: f32, bins_per_octave: u32) -> f32 {
    let frequencies = frequencies
        .iter()
        .filter(|x| **x > 0.)
        .map(|x| *x as f64)
        .collect::<Vec<f64>>();

    if frequencies.is_empty() {
        return 0.0;
    }
    let frequencies = hz_to_octs(&frequencies, 0.0, 12)
        .iter()
        .map(|x| (*x as f32 * bins_per_octave as f32) % 1.0)
        .collect::<Vec<f32>>();

    let residual = frequencies
        .iter()
        .map(|x| if *x >= 0.5 { *x - 1.0 } else { *x })
        .collect::<Vec<f32>>();

    let nb_iter = (1. / resolution).ceil() as i32 + 1;
    let bins = std::iter::repeat(-50)
        .zip(0..nb_iter)
        .map(|(a, b)| ((a + b) as f32) / 100.)
        .collect::<Vec<f32>>();

    let intervals = bins
        .iter()
        .zip(&bins[1..bins.len()])
        .collect::<Vec<(&f32, &f32)>>();
    let mut counts = vec![0; bins.len() - 1];
    for res in residual {
        for (i, (a, b)) in intervals.iter().enumerate() {
            if i == intervals.len() - 1 {
                if **a <= res && **b >= res {
                    counts[i] += 1;
                }
            } else if **a <= res && **b > res {
                counts[i] += 1;
            }
        }
    }
    let max_index = counts
        .iter()
        .enumerate()
        .rev()
        .max_by_key(|(_, v)| *v)
        .map(|(p, _)| p)
        .unwrap();
    bins[max_index]
}

fn estimate_tuning(
    sample_rate: u32,
    spectrum: &[Vec<f32>],
    n_fft: u32,
    resolution: f32,
    bins_per_octave: u32,
) -> f32 {
    let (pitch, mag) = pip_track(sample_rate, &spectrum, n_fft);

    let pitch_indexes = pitch
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.iter()
                .enumerate()
                .filter(|(_, x)| **x > 0.)
                .map(|(j, _)| (i, j))
                .collect::<Vec<(usize, usize)>>()
        })
        .flatten()
        .collect::<Vec<(usize, usize)>>();

    let mut threshold = 0.0;
    if !pitch_indexes.is_empty() {
        let mags = pitch_indexes
            .iter()
            .map(|(i, j)| mag[*i][*j])
            .collect::<Vec<f32>>();
        threshold = median(&mags);
    }
    let pitch = pitch_indexes
        .iter()
        .filter(|(i, j)| mag[*i][*j] >= threshold)
        .map(|(i, j)| pitch[*i][*j])
        .collect::<Vec<f32>>();
    pitch_tuning(&pitch, resolution, bins_per_octave)
}

pub fn chroma_stft(
    sample_rate: u32,
    spectrum: &[Vec<f32>],
    n_fft: u32,
    n_chroma: u32,
) -> Vec<Vec<f32>> {
    let tuning = estimate_tuning(sample_rate, &spectrum, n_fft, 0.01, n_chroma);
    let chromafb = chroma_filter(sample_rate, n_fft, n_chroma, tuning as f64);

    let mut raw_chroma = vec![vec![0.; spectrum[0].len()]; chromafb.len()];
    for i in 0..chromafb.len() {
        for j in 0..spectrum[0].len() {
            for k in 0..spectrum.len() {
                raw_chroma[i][j] += chromafb[i][k] * spectrum[k][j];
            }
        }
    }

    let mut length = vec![0.; raw_chroma[0].len()];
    for i in 0..(raw_chroma[0].len() as usize) {
        let mut vec = vec![0.; raw_chroma[0].len() as usize];
        for x in &raw_chroma {
            vec.push(x[i].abs());
        }
        let max = *vec.iter().max_by(|x, y| x.partial_cmp(y).unwrap()).unwrap();
        if max.abs() < f32::MIN_POSITIVE {
            length[i] = 1.;
        } else {
            length[i] = max;
        }
    }

    let normalized = raw_chroma
        .iter()
        .map(|s| {
            s.iter()
                .enumerate()
                .map(|(i, v)| v / (&length)[i])
                .collect::<Vec<f32>>()
        })
        .collect::<Vec<Vec<f32>>>();

    let mut transpose_normalized = vec![vec![0.; normalized.len()]; normalized[0].len()];
    for i in 0..normalized.len() {
        for j in 0..normalized[0].len() {
            transpose_normalized[j][i] = normalized[i][j];
        }
    }

    transpose_normalized
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::decode::decode_song;
    use ndarray::{Array2, Axis};
    use ndarray_npy::ReadNpyExt;
    use std::fs::File;

    #[test]
    fn test_chroma_desc() {
        let song = decode_song("data/piano.flac").unwrap();
        let mut chroma_desc = ChromaDesc::new(song.sample_rate, 12);
        for chunk in song.sample_array.chunks_exact(ChromaDesc::HOP_SIZE) {
            chroma_desc.do_(&chunk);
        }
        chroma_desc.finish();

        // Temporary get_value()
        assert!((481.0289 - chroma_desc.get_value()).abs() < 0.001);
        //for val in chroma_desc.values_chroma {
        //    let joined = val
        //        .iter()
        //        .map(|x| x.to_string())
        //        .collect::<Vec<String>>()
        //        .join(",");
        //    println!("{}", joined);
        //}
    }

    #[test]
    fn test_chroma_stft() {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let arr = Array2::<f32>::read_npy(file).unwrap();
        let len = arr.len_of(Axis(0));
        let mut vec = vec![];
        for i in 0..len {
            vec.push(arr.row(i).to_vec());
        }

        let chroma = chroma_stft(22050, &vec, 2048, 12);

        let chroma_stft_file = File::open("data/chroma-stft-normalized-expected.npy").unwrap();
        let arr = Array2::<f32>::read_npy(chroma_stft_file).unwrap();
        let len = arr.len_of(Axis(0));
        let mut expected_chroma = vec![];
        for i in 0..len {
            expected_chroma.push(arr.row(i).to_vec());
        }
        for (column1, column2) in expected_chroma.iter().zip(chroma.iter()) {
            for (val1, val2) in column1.iter().zip(column2) {
                assert!(0.001 > (val1 - val2).abs());
            }
        }
    }

    #[test]
    fn test_estimate_tuning() {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let arr = Array2::<f32>::read_npy(file).unwrap();
        let len = arr.len_of(Axis(0));
        let mut vec = vec![];
        for i in 0..len {
            vec.push(arr.row(i).to_vec());
        }

        let tuning = estimate_tuning(22050, &vec, 2048, 0.01, 12);
        assert!(0.0001 > (-0.09999999999999998 - tuning).abs());
    }

    #[test]
    fn test_pitch_tuning() {
        let frequencies = [
            0.0000000e+00,
            -1.0000000e+00,
            1.4867015e+02,
            0.0000000e+00,
            1.5185892e+02,
            1.5036792e+02,
            1.4787907e+02,
            3.1000000e+03,
            0.0000000e+00,
            1.4916122e+02,
            1.4862317e+02,
            3.1000000e+03,
            1.4717630e+02,
            1.4680702e+02,
            0.0000000e+00,
            -1.0000000e+00,
            3.9000000e+03,
            0.0000000e+00,
        ];
        assert_eq!(-0.2, pitch_tuning(&frequencies, 0.01, 12));
    }

    #[test]
    fn test_pitch_tuning_no_frequencies() {
        let frequencies = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(0.0, pitch_tuning(&frequencies, 0.01, 12));
    }

    #[test]
    fn test_pip_track() {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let arr = Array2::<f32>::read_npy(file).unwrap();
        let len = arr.len_of(Axis(0));
        let mut vec = vec![];
        for i in 0..len {
            vec.push(arr.row(i).to_vec());
        }

        let mags_file = File::open("data/spectrum-chroma-mags.npy").unwrap();
        let arr = Array2::<f32>::read_npy(mags_file).unwrap();
        let len = arr.len_of(Axis(0));
        let mut expected_mags = vec![];
        for i in 0..len {
            expected_mags.push(arr.row(i).to_vec());
        }

        let pitches_file = File::open("data/spectrum-chroma-pitches.npy").unwrap();
        let arr = Array2::<f32>::read_npy(pitches_file).unwrap();
        let len = arr.len_of(Axis(0));
        let mut expected_pitches = vec![];
        for i in 0..len {
            expected_pitches.push(arr.row(i).to_vec());
        }
        let (pitches, mags) = pip_track(22050, &vec, 2048);

        for (column1, column2) in expected_mags.iter().zip(mags.iter()) {
            for (val1, val2) in column1.iter().zip(column2) {
                assert!(0.0001 > (val1 - val2).abs());
            }
        }
        for (column1, column2) in expected_pitches.iter().zip(pitches.iter()) {
            for (val1, val2) in column1.iter().zip(column2) {
                assert!(0.001 > (val1 - val2).abs());
            }
        }
    }

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
