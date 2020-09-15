#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

extern crate ndarray;
extern crate ndarray_npy;
extern crate ndarray_stats;

use crate::utils::{hz_to_octs, median};
use aubio_rs::vec::CVec;
use aubio_rs::PVoc;
use ndarray::{arr1, arr2, s, stack, Array, Array1, Array2, Axis, RemoveAxis};
use ndarray_stats::QuantileExt;

pub struct ChromaDesc {
    sample_rate: u32,
    n_chroma: u32,
    phase_vocoder: PVoc,
    // 12 * nb_fft chroma values
    values_chroma: Array2<f64>,
    collected_spectra: Array2<f64>,
}

impl ChromaDesc {
    pub const WINDOW_SIZE: usize = 2048;
    pub const HOP_SIZE: usize = 512;

    pub fn new(sample_rate: u32, n_chroma: u32) -> ChromaDesc {
        ChromaDesc {
            sample_rate,
            n_chroma,
            values_chroma: arr2(&[[]]),
            phase_vocoder: PVoc::new(ChromaDesc::WINDOW_SIZE, ChromaDesc::HOP_SIZE).unwrap(),
            collected_spectra: arr2(&[[]]),
        }
    }

    pub fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; ChromaDesc::WINDOW_SIZE + 2];
        self.phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let cvec: CVec = fftgrain.as_slice().into();
        let norm: Array1<f64> = cvec.norm().iter().map(|x| *x as f64).collect();
        if self.collected_spectra.is_empty() {
            self.collected_spectra = norm.insert_axis(Axis(1));
        }
        else {
            self.collected_spectra = stack![Axis(1), self.collected_spectra, norm.insert_axis(Axis(1))];
        }
        // If we have collected more than a hundred spectra
        if self.collected_spectra.len_of(Axis(1)) >= 100 {
            let chroma = chroma_stft(
                self.sample_rate,
                &self.collected_spectra,
                ChromaDesc::WINDOW_SIZE as u32,
                self.n_chroma,
            );
            if self.values_chroma.is_empty() {
                self.values_chroma = chroma;
            }
            else {
                self.values_chroma = stack![Axis(1), self.values_chroma, chroma];
            }

            self.collected_spectra = arr2(&[[]]);
        }
    }

    pub fn finish(&mut self) {
        if self.collected_spectra.len() <= 2 {
            return;
        }
        let chroma = chroma_stft(
            self.sample_rate,
            &self.collected_spectra,
            ChromaDesc::WINDOW_SIZE as u32,
            self.n_chroma,
        );
        if self.values_chroma.is_empty() {
                self.values_chroma = chroma;
        }
        else {
            self.values_chroma = stack![Axis(1), self.values_chroma, chroma];
        }
        self.collected_spectra = arr2(&[[]]);
    }

    // Doesn't make any sense now! Only here for the test
    pub fn get_value(&mut self) -> f64 {
        self.values_chroma
            .iter()
            .sum()
    }
}

// All the functions below are more than heavily inspired from
// librosa's code: https://github.com/librosa/librosa/blob/main/librosa/feature/spectral.py#L1165
// chroma(22050, n_fft=5, n_chroma=12)
fn chroma_filter(sample_rate: u32, n_fft: u32, n_chroma: u32, tuning: f64) -> Array2<f64> {
    let ctroct = 5.0;
    let octwidth = 2;

    let frequencies = Array::linspace(0., sample_rate as f64, (n_fft + 1) as usize);
    let frequencies = frequencies.slice_move(s![1..-1]);

    let freq_bins = n_chroma as f64 * hz_to_octs(&frequencies, tuning, n_chroma);
    let freq_bins = stack![
        Axis(0),
        arr1(&[freq_bins[0] - 1.5 * n_chroma as f64]),
        freq_bins
    ];

    let binwidth_bins = stack![
        Axis(0),
        (&freq_bins.slice(s![1..]) - &freq_bins.slice(s![..-1])).mapv(|x| if x <= 1. { 1. } else { x }),
        arr1(&[1.])
    ];

    let mut a: Array2<f64> = Array::zeros((n_chroma as usize, (&freq_bins).len()));
    for (idx, mut row) in a.genrows_mut().into_iter().enumerate() {
        row.fill(idx as f64);
    }
    let d = -a + &freq_bins;
    let n_chroma2 = (n_chroma as f64 / 2.0).round() as u32;

    let d = (d + n_chroma2 as f64 + 10. * n_chroma as f64) % n_chroma as f64 - n_chroma2 as f64;
    let mut a: Array2<f64> = Array::zeros((n_chroma as usize, binwidth_bins.len()));
    for mut row in a.genrows_mut() {
        row.assign(&binwidth_bins);
    }
    let mut wts = (-0.5 * (2. * d / a).mapv(|x| x.powf(2.))).mapv(f64::exp);
    //// Normalize by computing the l2-norm over the columns
    for mut col in wts.gencolumns_mut() {
        let mut sum = (&col * &col).sum().sqrt();
        if sum < f64::MIN_POSITIVE {
            sum = 1.;
        }
        col.assign(&(&col / sum));
    }

    let mut scaling: Array2<f64> = Array::zeros((n_chroma as usize, freq_bins.len()));
    for mut row in scaling.genrows_mut() {
        row.assign(
            &(-0.5
                * ((&freq_bins / n_chroma as f64 - ctroct) / octwidth as f64).mapv(|x| x.powf(2.)))
            .mapv(f64::exp),
        );
    }

    let wts = wts * scaling;

    // np.roll(), np bro
    let mut uninit: Vec<f64> = Vec::with_capacity((&wts).len());
    unsafe {
        uninit.set_len(wts.len());
    }
    let mut b = Array::from(uninit).into_shape(wts.dim()).unwrap();
    b.slice_mut(s![-3.., ..]).assign(&wts.slice(s![..3, ..]));
    b.slice_mut(s![..-3, ..]).assign(&wts.slice(s![3.., ..]));

    let wts = b;
    let non_aliased = (1 + n_fft / 2) as usize;
    wts.slice_move(s![.., ..non_aliased])
}

fn pip_track(sample_rate: u32, spectrum: &Array2<f64>, n_fft: u32) -> (Array2<f64>, Array2<f64>) {
    let fmin = 150.0_f64;
    let fmax = 4000.0_f64.min(sample_rate as f64 / 2.0);
    let threshold = 0.1;

    let fft_freqs = Array::linspace(0., sample_rate as f64 / 2., (1 + n_fft / 2) as usize);

    let avg = 0.5 * (&spectrum.slice(s![2.., ..]) - &spectrum.slice(s![..-2, ..]));
    let shift = 2. * &spectrum.slice(s![1..-1, ..])
        - &spectrum.slice(s![2.., ..])
        - &spectrum.slice(s![0..-2, ..]);

    // TODO find more optimal stuff
    let shift = &avg
        / &shift.mapv(|x| {
            if x.abs() < f64::MIN_POSITIVE {
                x + 1.
            } else {
                x
            }
        });
    let zeros: Array2<f64> = Array::zeros((1, shift.shape()[1]));

    let avg = stack![Axis(0), zeros, stack![Axis(0), avg, zeros]];
    let shift = stack![Axis(0), zeros, stack![Axis(0), shift, zeros]];

    let dskew = 0.5 * &avg * &shift;

    let freq_mask = fft_freqs
        .iter()
        .map(|&f| (fmin <= f) && (f < fmax))
        .collect::<Vec<bool>>();

    let mut ref_value = Array::zeros(spectrum.raw_dim().remove_axis(Axis(0)));
    for (i, row) in spectrum.axis_iter(Axis(1)).enumerate() {
        ref_value[i] = threshold * *row.max().unwrap();
    }

    let mut idx = Vec::new();
    let length_spectrum = spectrum.len_of(Axis(0));
    for ((i, j), elem) in spectrum.indexed_iter() {
        if i == 0 {
            {}
        } else if i + 1 >= length_spectrum {
            if spectrum[[i - 1, j]] < *elem && *elem > ref_value[j] && freq_mask[i] {
                idx.push((i, j));
            }
        } else if spectrum[[i - 1, j]] < *elem
            && spectrum[[i + 1, j]] <= *elem
            && *elem > ref_value[j]
            && freq_mask[i]
        {
            idx.push((i, j));
        }
    }

    let mut pitches = Array::zeros(spectrum.raw_dim());
    let mut mags = Array::zeros(spectrum.raw_dim());

    for (i, j) in idx {
        pitches[[i, j]] = (i as f64 + shift[[i, j]]) * sample_rate as f64 / n_fft as f64;
        mags[[i, j]] = spectrum[[i, j]] + dskew[[i, j]];
    }
    (pitches, mags)
}

fn pitch_tuning(frequencies: &Array1<f64>, resolution: f64, bins_per_octave: u32) -> f64 {
    let frequencies = frequencies
        .iter()
        .filter(|x| **x > 0.)
        .map(|x| *x as f64)
        .collect::<Array1<f64>>();
    
    if frequencies.is_empty() {
        return 0.0;
    }
    let frequencies = bins_per_octave as f64 * hz_to_octs(&frequencies, 0.0, 12) % 1.0;

    let residual = frequencies.mapv(|x| if x >= 0.5 { x - 1. } else { x });

    let bins = Array::linspace(-50., 50., (1. / resolution).ceil() as usize + 1) / 100.;

    let mut counts: Array1<usize> = Array::zeros(bins.len() - 1);
    for res in residual.iter() {
        let idx = ((res - -0.5) / resolution) as usize;
        counts[idx] += 1;
    }

    let max_index = counts.argmax().unwrap();
    bins[max_index]
}

fn estimate_tuning(
    sample_rate: u32,
    spectrum: &Array2<f64>,
    n_fft: u32,
    resolution: f64,
    bins_per_octave: u32,
) -> f64 {
    let (pitch, mag) = pip_track(sample_rate, &spectrum, n_fft);

    let pitches_index = pitch
        .indexed_iter()
        .filter(|(_, item)| **item > 0.)
        .map(|((i, j), _)| (i, j))
        .collect::<Vec<(usize, usize)>>();

    let mut threshold = 0.0;
    
    // TODO change that to Array1 stuff when bulk-indexing will be supported
    if !pitches_index.is_empty() {
        let mags = pitches_index.iter()
            .map(|(i, j)| mag[[*i, *j]])
            .collect::<Vec<f64>>();
        threshold = median(&mags);
    }

    let pitch = pitches_index
        .iter()
        .filter(|(i, j)| mag[[*i, *j]] >= threshold)
        .map(|(i, j)| pitch[[*i, *j]])
        .collect::<Array1<f64>>();

    pitch_tuning(&pitch, resolution, bins_per_octave)
}

pub fn chroma_stft(
    sample_rate: u32,
    spectrum: &Array2<f64>,
    n_fft: u32,
    n_chroma: u32,
) -> Array2<f64> {
    let tuning = estimate_tuning(sample_rate, &spectrum, n_fft, 0.01, n_chroma);
    let chromafb = chroma_filter(sample_rate, n_fft, n_chroma, tuning);

    let mut raw_chroma = chromafb.dot(spectrum);
    for mut row in raw_chroma.gencolumns_mut() {
        let mut max = *row.mapv(f64::abs).max().unwrap();
        if max < f64::MIN_POSITIVE {
            max = 1.;
        }
        let max_row = Array::from_elem(row.raw_dim(), max);
        row.assign(&(row.to_owned() / max_row));
    }
    raw_chroma
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::decode::decode_song;
    use ndarray::{Array2};
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

        assert!((481.0289 - chroma_desc.get_value()).abs() < 0.001);
    }

    #[test]
    fn test_chroma_stft() {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let spectrum = Array2::<f64>::read_npy(file).unwrap();

        let chroma = chroma_stft(22050, &spectrum, 2048, 12);

        let chroma_stft_file = File::open("data/chroma-stft-normalized-expected.npy").unwrap();
        let expected_chroma = Array2::<f64>::read_npy(chroma_stft_file).unwrap();

        for (expected, actual) in expected_chroma.iter().zip(chroma.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }
    }

    #[test]
    fn test_estimate_tuning() {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let arr = Array2::<f64>::read_npy(file).unwrap();

        let tuning = estimate_tuning(22050, &arr, 2048, 0.01, 12);
        assert!(0.000001 > (-0.09999999999999998 - tuning).abs());
    }

    #[test]
    fn test_pitch_tuning() {
        let file = File::open("data/pitch-tuning.npy").unwrap();
        let pitch = Array1::<f64>::read_npy(file).unwrap();

        assert_eq!(-0.1, pitch_tuning(&pitch, 0.05, 12));
    }

    #[test]
    fn test_pitch_tuning_no_frequencies() {
        let frequencies = arr1(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(0.0, pitch_tuning(&frequencies, 0.05, 12));
    }

    #[test]
    fn test_pip_track() {
        let file = File::open("data/spectrum-chroma.npy").unwrap();
        let spectrum = Array2::<f64>::read_npy(file).unwrap();

        let mags_file = File::open("data/spectrum-chroma-mags.npy").unwrap();
        let expected_mags = Array2::<f64>::read_npy(mags_file).unwrap();

        let pitches_file = File::open("data/spectrum-chroma-pitches.npy").unwrap();
        let expected_pitches = Array2::<f64>::read_npy(pitches_file).unwrap();

        let (pitches, mags) = pip_track(22050, &spectrum, 2048);

        for (expected_pitches, actual_pitches) in expected_pitches.iter().zip(pitches.iter()) {
            assert!(0.00000001 > (expected_pitches - actual_pitches).abs());
        }
        for (expected_mags, actual_mags) in expected_mags.iter().zip(mags.iter()) {
            assert!(0.00000001 > (expected_mags - actual_mags).abs());
        }
    }

    #[test]
    fn test_generate_chroma() {
        let file = File::open("data/chroma-filter.npy").unwrap();
        let expected_filter = Array2::<f64>::read_npy(file).unwrap();

        let filter = chroma_filter(22050, 2048, 12, -0.1);

        for (expected, actual) in expected_filter.iter().zip(filter.iter()) {
            assert!(0.000000001 > (expected - actual).abs());
        }
    }
}
