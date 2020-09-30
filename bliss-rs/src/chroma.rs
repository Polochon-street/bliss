#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

extern crate ndarray;
extern crate ndarray_npy;
extern crate ndarray_stats;

use crate::analyze::stft;
use crate::utils::{hz_to_octs, median};
use ndarray::{arr1, s, stack, Array, Array1, Array2, Axis, RemoveAxis};
use ndarray_stats::QuantileExt;

pub struct ChromaDesc {
    sample_rate: u32,
    n_chroma: u32,
    values_chroma: Array2<f64>,
}

impl ChromaDesc {
    pub const WINDOW_SIZE: usize = 8192;
    pub const HOP_SIZE: usize = 2205;

    pub fn new(sample_rate: u32, n_chroma: u32) -> ChromaDesc {
        ChromaDesc {
            sample_rate,
            n_chroma,
            values_chroma: Array2::zeros((n_chroma as usize, 0)),
        }
    }

    pub fn do_(&mut self, signal: &[f32]) {
        let stft = stft(signal, 8192, 2205);
        let tuning = estimate_tuning(
            self.sample_rate as u32,
            &stft,
            ChromaDesc::WINDOW_SIZE,
            0.01,
            12,
        );
        let chroma = chroma_stft(
            self.sample_rate,
            &stft,
            ChromaDesc::WINDOW_SIZE,
            self.n_chroma,
            Some(tuning),
        );
        self.values_chroma = stack![Axis(1), self.values_chroma, chroma];
    }

    // Doesn't make any sense now! Only here for testing
    pub fn get_value(&mut self) -> f64 {
        self.values_chroma.sum()
    }
}

// All the functions below are more than heavily inspired from
// librosa's code: https://github.com/librosa/librosa/blob/main/librosa/feature/spectral.py#L1165
// chroma(22050, n_fft=5, n_chroma=12)
fn chroma_filter(sample_rate: u32, n_fft: usize, n_chroma: u32, tuning: f64) -> Array2<f64> {
    let ctroct = 5.0;
    let octwidth = 2;

    let frequencies = Array::linspace(0., f64::from(sample_rate), (n_fft + 1) as usize);
    let length = frequencies.len();
    let frequencies = frequencies.slice_move(s![1..length - 1]);

    let freq_bins = f64::from(n_chroma) * hz_to_octs(&frequencies, tuning, n_chroma);
    let freq_bins = stack![
        Axis(0),
        arr1(&[freq_bins[0] - 1.5 * f64::from(n_chroma)]),
        freq_bins
    ];

    let binwidth_bins = stack![
        Axis(0),
        (&freq_bins.slice(s![1..]) - &freq_bins.slice(s![..-1])).mapv(|x| if x <= 1. {
            1.
        } else {
            x
        }),
        arr1(&[1.])
    ];

    let mut a: Array2<f64> = Array::zeros((n_chroma as usize, (&freq_bins).len()));
    for (idx, mut row) in a.genrows_mut().into_iter().enumerate() {
        row.fill(idx as f64);
    }
    let d = -a + &freq_bins;
    let n_chroma2 = (f64::from(n_chroma) / 2.0).round() as u32;

    let d = (d + f64::from(n_chroma2) + 10. * f64::from(n_chroma)) % f64::from(n_chroma) - f64::from(n_chroma2) as f64;
    let mut a: Array2<f64> = Array::zeros((n_chroma as usize, binwidth_bins.len()));
    for mut row in a.genrows_mut() {
        row.assign(&binwidth_bins);
    }
    let mut wts = (-0.5 * (2. * d / a).mapv(|x| x.powf(2.))).mapv(f64::exp);
    // Normalize by computing the l2-norm over the columns
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
                * ((&freq_bins / f64::from(n_chroma) - ctroct) / f64::from(octwidth)).mapv(|x| x.powf(2.)))
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

fn pip_track(sample_rate: u32, spectrum: &Array2<f64>, n_fft: usize) -> (Array2<f64>, Array2<f64>) {
    let fmin = 150.0_f64;
    let fmax = 4000.0_f64.min(f64::from(sample_rate) / 2.0);
    let threshold = 0.1;

    let fft_freqs = Array::linspace(0., f64::from(sample_rate) / 2., 1 + n_fft / 2);

    let avg = 0.5 * (&spectrum.slice(s![2.., ..]) - &spectrum.slice(s![..-2, ..]));
    let length = spectrum.len_of(Axis(0));
    let shift = 2. * &spectrum.slice(s![1..length - 1, ..])
        - spectrum.slice(s![2.., ..])
        - spectrum.slice(s![0..length - 2, ..]);

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
        pitches[[i, j]] = (i as f64 + shift[[i, j]]) * f64::from(sample_rate) / n_fft as f64;
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
    let frequencies = f64::from(bins_per_octave) * hz_to_octs(&frequencies, 0.0, 12) % 1.0;

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
    n_fft: usize,
    resolution: f64,
    bins_per_octave: u32,
) -> f64 {
    let (pitch, mag) = pip_track(sample_rate, &spectrum, n_fft);

    let pitches_index = pitch
        .indexed_iter()
        .filter(|(_, item)| **item > 0.)
        .map(|((i, j), _)| (i, j))
        .collect::<Vec<(usize, usize)>>();

    // TODO change that to Array1 stuff when bulk-indexing will be supported
    let threshold = {
        if !pitches_index.is_empty() {
            let mags = pitches_index
                .iter()
                .map(|(i, j)| mag[[*i, *j]])
                .collect::<Vec<f64>>();
            median(&mags)
        }
        else { 0. }
    };

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
    n_fft: usize,
    n_chroma: u32,
    tuning: Option<f64>,
) -> Array2<f64> {
    let tuning = match tuning {
        Some(x) => x,
        None => estimate_tuning(sample_rate, &spectrum, n_fft, 0.01, n_chroma),
    };
    let spectrum = &spectrum.mapv(|x| x.powf(2.));
    let chromafb = chroma_filter(sample_rate, n_fft, n_chroma, tuning);

    let mut raw_chroma = chromafb.dot(spectrum);
    for mut row in raw_chroma.gencolumns_mut() {
        let mut sum = row.mapv(|x| x.powf(2.)).sum().sqrt();
        if sum < f64::MIN_POSITIVE {
            sum = 1.;
        }
        let sum_row = Array::from_elem(row.raw_dim(), sum);
        row.assign(&(row.to_owned() / sum_row));
    }
    raw_chroma
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::analyze::stft;
    use crate::decode::decode_song;
    use ndarray::Array2;
    use ndarray_npy::ReadNpyExt;
    use std::fs::File;

    #[test]
    fn test_chroma_desc() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut chroma_desc = ChromaDesc::new(song.sample_rate, 12);
        chroma_desc.do_(&song.sample_array);
        assert!((263.7979324- chroma_desc.get_value()).abs() < 0.00001);
    }

    #[test]
    fn test_chroma_stft_decode() {
        let signal = decode_song("data/s16_mono_22_5kHz.flac")
            .unwrap()
            .sample_array;
        let stft = stft(&signal, 8192, 2205);

        let file = File::open("data/chroma.npy").unwrap();
        let expected_chroma = Array2::<f64>::read_npy(file).unwrap();

        let chroma = chroma_stft(22050, &stft, 8192, 12, Some(-0.04999999999999999));

        assert!(!chroma.is_empty() && !expected_chroma.is_empty());

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
    fn test_estimate_tuning_decode() {
        let signal = decode_song("data/s16_mono_22_5kHz.flac")
            .unwrap()
            .sample_array;
        let stft = stft(&signal, 8192, 2205);

        let tuning = estimate_tuning(22050, &stft, 8192, 0.01, 12);
        assert!(0.000001 > (-0.04999999999999999 - tuning).abs());
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
    fn test_chroma_filter() {
        let file = File::open("data/chroma-filter.npy").unwrap();
        let expected_filter = Array2::<f64>::read_npy(file).unwrap();

        let filter = chroma_filter(22050, 2048, 12, -0.1);

        for (expected, actual) in expected_filter.iter().zip(filter.iter()) {
            assert!(0.000000001 > (expected - actual).abs());
        }
    }
}
