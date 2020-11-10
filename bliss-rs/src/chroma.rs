//! Chroma feature extraction module.
//!
//! Contains functions to compute the chromagram of a song, and
//! then from this chromagram extract the song's tone and mode
//! (minor / major).
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use crate::analyze::stft;
use crate::utils::{convolve, hz_to_octs, median, TEMPLATES_MAJMIN};
use ndarray::{arr1, arr2, s, stack, Array, Array1, Array2, Axis, RemoveAxis, Zip};
use ndarray_stats::QuantileExt;
use std::f32::consts::PI;

const CHORD_LABELS: [&str; 24] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B", "Cm", "C#m", "Dm", "D#m",
    "Em", "Fm", "F#m", "Gm", "G#m", "Am", "A#m", "Bm",
];
// Contains the sequence of fifths: CHORD_LABELS[0] = C, CHORD_LABELS[7] = G, etc.
const PERFECT_FIFTH_INDICES: [u8; 12] = [0, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10, 5];
#[allow(dead_code)]
const SCALE_LABELS_ABSOLUTE: [&str; 12] = [
    "0", "1#", "2#", "3#", "4#", "5#", "6#", "5b", "4b", "3b", "2b", "1b",
];
// In order https://en.wikipedia.org/wiki/Circle_of_fifths#/media/File:Circle_of_fifths_deluxe_4.svg
// 0 = C / A, 1# = G / E etc
const CIRCLE_FIFTHS: [(&str, &str); 12] = [
    ("C", "A"),
    ("G", "E"),
    ("D", "B"),
    ("A", "F#"),
    ("E", "C#"),
    ("B", "G#"),
    ("F#", "D#"),
    ("C#", "A#"),
    ("G#", "F"),
    ("D#", "C"),
    ("A#", "G"),
    ("F", "D"),
];

/**
 * General object holding the chroma descriptor.
 *
 * Current chroma descriptors are the tone and the mode, see the [circle of
 * fifths](https://en.wikipedia.org/wiki/Circle_of_fifths#/media/File:Circle_of_fifths_deluxe_4.svg).
 *
 * Contrary to the other descriptors that can be used with streaming
 * without consequences, this one performs better if the full song is used at
 * once.
 */
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

    // Here, we want to call `do_()` on the whole song as much as possible - streaming
    // would be rather bad
    /**
     * Compute and store the chroma of a signal.
     *
     * Passing a full song here once instead of streaming smaller parts of the
     * song will greatly improve accuracy.
     */
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

    /**
     * Get the song's mode (minor / major) and its tone.
     *
     * The song's tone is made of the projection of
     * https://en.wikipedia.org/wiki/Circle_of_fifths#/media/File:Circle_of_fifths_deluxe_4.svg
     * into a trigonometric circle: for example 1# is at pi/3, #2 pi/6, etc.
     * While it may not make a lot of sense conceptually, it's a good way to
     * convert the tone in a set of usable / comparable features.
     */
    pub fn get_values(&mut self) -> (bool, (f32, f32)) {
        chroma_fifth_is_major(&self.values_chroma)
    }
}

// Functions below are Rust versions of python notebooks by AudioLabs Erlang
// (https://www.audiolabs-erlangen.de/resources/MIR/FMP/C0/C0.html)
fn chroma_fifth_is_major(chroma: &Array2<f64>) -> (bool, (f32, f32)) {
    // Values here are in the same order as SCALE_LABELS_ABSOLUTES
    let scale_values: [(f32, f32); 12] = [
        (f32::cos(PI / 2.), f32::sin(PI / 2.)),
        (f32::cos(PI / 3.), f32::sin(PI / 3.)),
        (f32::cos(PI / 6.), f32::sin(PI / 6.)),
        (f32::cos(0.), f32::sin(0.)),
        (f32::cos(11. * PI / 6.), f32::sin(11. * PI / 6.)),
        (f32::cos(5. * PI / 3.), f32::sin(5. * PI / 3.)),
        (f32::cos(3. * PI / 2.), f32::sin(3. * PI / 2.)),
        (f32::cos(4. * PI / 3.), f32::sin(4. * PI / 3.)),
        (f32::cos(7. * PI / 6.), f32::sin(7. * PI / 6.)),
        (f32::cos(PI), f32::sin(PI)),
        (f32::cos(5. * PI / 6.), f32::sin(5. * PI / 6.)),
        (f32::cos(2. * PI / 3.), f32::sin(2. * PI / 3.)),
    ];

    let templates_majmin = Array::from_shape_vec((12, 24), TEMPLATES_MAJMIN.to_vec()).unwrap();

    let chroma_filtered = smooth_downsample_feature_sequence(chroma, 15, 10);
    let chroma_filtered = normalize_feature_sequence(&chroma_filtered);
    let f_analysis_prefilt = analysis_template_match(&chroma_filtered, &templates_majmin, true);
    let mut f_analysis_max_prefilt = Array::zeros((24, f_analysis_prefilt.dim().1));
    for (i, column) in f_analysis_prefilt.gencolumns().into_iter().enumerate() {
        let index = column.argmax().unwrap();
        f_analysis_max_prefilt[[index, i]] = 1.;
    }
    let summed = f_analysis_max_prefilt.sum_axis(Axis(1));

    let chroma_filtered = smooth_downsample_feature_sequence(chroma, 45, 15);
    let chroma_filtered = normalize_feature_sequence(&chroma_filtered);
    let chroma_sorted = sort_by_fifths(&chroma_filtered, -1);
    let template_diatonic = arr2(&[[1., 3., 2., 1., 2., 3., 1., 0., 0., 0., 0., 0.]])
        .t()
        .to_owned();
    let templates_scale = generate_template_matrix(&template_diatonic);
    let f_analysis = analysis_template_match(&chroma_sorted, &templates_scale, false);
    let f_analysis_norm = normalize_feature_sequence(&f_analysis);
    let f_analysis_exp = (f_analysis_norm * 70.).mapv(f64::exp);
    let f_analysis_rescaled = (&f_analysis_exp / &f_analysis_exp.sum_axis(Axis(0))).to_owned();
    // should this really be a mean?
    let index = f_analysis_rescaled
        .mean_axis(Axis(1))
        .unwrap()
        .argmax()
        .unwrap();
    let major_chord = CIRCLE_FIFTHS[index].0;
    let major_chord_index = CHORD_LABELS.iter().position(|&x| x == major_chord).unwrap();
    let minor_chord = format!("{}m", CIRCLE_FIFTHS[index].1);
    let minor_chord_index = CHORD_LABELS.iter().position(|&x| x == minor_chord).unwrap();

    let minor = summed[minor_chord_index];
    let major = summed[major_chord_index];
    let mode = scale_values[index];
    (major > minor, mode)
}

fn generate_template_matrix(templates: &Array2<f64>) -> Array2<f64> {
    let mut output = Array2::zeros((12, 12 * templates.dim().1));

    for shift in 0..12 as isize {
        let mut uninit: Vec<f64> = Vec::with_capacity((&templates).len());
        unsafe {
            uninit.set_len(templates.len());
        }
        let mut rolled = Array::from(uninit).into_shape(templates.dim()).unwrap();
        if shift != 0 {
            rolled
                .slice_mut(s![shift.., ..])
                .assign(&templates.slice(s![..-shift, ..]));
            rolled
                .slice_mut(s![..shift, ..])
                .assign(&templates.slice(s![-shift.., ..]));
        } else {
            rolled = templates.to_owned();
        }
        output
            .column_mut(shift as usize)
            .assign(&rolled.index_axis(Axis(1), 0));
        // TODO ugly hack; fixme
        if templates.dim().1 > 1 {
            output
                .column_mut(shift as usize + 12)
                .assign(&rolled.index_axis(Axis(1), 1));
        }
    }

    output
}

fn sort_by_fifths(feature: &Array2<f64>, offset: isize) -> Array2<f64> {
    let mut output = Array2::zeros((0, feature.dim().1));
    for index in PERFECT_FIFTH_INDICES.iter() {
        output = stack![
            Axis(0),
            output,
            feature
                .index_axis(Axis(0), *index as usize)
                .insert_axis(Axis(0))
        ];
    }

    // np.roll again TODO make a proper function
    // np.roll(array, -offset)
    let mut uninit: Vec<f64> = Vec::with_capacity((&output).len());
    unsafe {
        uninit.set_len(output.len());
    }
    let mut b = Array::from(uninit).into_shape(output.dim()).unwrap();
    b.slice_mut(s![-offset.., ..])
        .assign(&output.slice(s![..offset, ..]));
    b.slice_mut(s![..-offset, ..])
        .assign(&output.slice(s![offset.., ..]));

    b
}

fn smooth_downsample_feature_sequence(
    feature: &Array2<f64>,
    filter_length: u32,
    down_sampling: u32,
) -> Array2<f64> {
    let filter_kernel = Array::ones(filter_length as usize);
    let mut output = Array2::zeros((
        0,
        (feature.dim().1 as f64 / down_sampling as f64).ceil() as usize,
    ));
    for row in feature.genrows() {
        let smoothed = convolve(&row.to_owned(), &filter_kernel);
        let smoothed: Array2<f64> = smoothed
            .to_vec()
            .into_iter()
            .step_by(down_sampling as usize)
            .collect::<Array1<f64>>()
            .insert_axis(Axis(0));
        output = stack![Axis(0), output, smoothed];
    }
    output / filter_length as f64
}

fn normalize_feature_sequence(feature: &Array2<f64>) -> Array2<f64> {
    let mut normalized_sequence = Array::zeros(feature.raw_dim());
    Zip::from(feature.gencolumns())
        .and(normalized_sequence.gencolumns_mut())
        .apply(|col, mut norm_col| {
            let mut sum = (&col * &col).sum().sqrt();
            if sum < 0.0001 {
                sum = 1.;
            }
            norm_col.assign(&(&col / sum));
        });

    normalized_sequence
}

fn analysis_template_match(
    chroma: &Array2<f64>,
    templates: &Array2<f64>,
    normalize: bool,
) -> Array2<f64> {
    if chroma.shape()[0] != 12 || templates.shape()[0] != 12 {
        panic!("Wrong size for input");
    }

    let chroma_normalized = normalize_feature_sequence(chroma);
    let templates_normalized = normalize_feature_sequence(templates);

    let f_analysis = &templates_normalized.t().dot(&chroma_normalized);
    if normalize {
        normalize_feature_sequence(&f_analysis)
    } else {
        f_analysis.to_owned()
    }
}

// All the functions below are more than heavily inspired from
// librosa"s code: https://github.com/librosa/librosa/blob/main/librosa/feature/spectral.py#L1165
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

    let d = (d + f64::from(n_chroma2) + 10. * f64::from(n_chroma)) % f64::from(n_chroma)
        - f64::from(n_chroma2) as f64;
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
                * ((&freq_bins / f64::from(n_chroma) - ctroct) / f64::from(octwidth))
                    .mapv(|x| x.powf(2.)))
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
        } else {
            0.
        }
    };

    let pitch = pitches_index
        .iter()
        .filter(|(i, j)| mag[[*i, *j]] >= threshold)
        .map(|(i, j)| pitch[[*i, *j]])
        .collect::<Array1<f64>>();

    pitch_tuning(&pitch, resolution, bins_per_octave)
}

fn chroma_stft(
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
    use ndarray::{arr2, Array2};
    use ndarray_npy::ReadNpyExt;
    use std::fs::File;

    #[test]
    fn test_fifth_is_major() {
        let file = File::open("data/chroma.npy").unwrap();
        let chroma = Array2::<f64>::read_npy(file).unwrap();

        let fifth_is_major = chroma_fifth_is_major(&chroma);
        assert_eq!(
            fifth_is_major,
            (false, (f32::cos(5. * PI / 3.), f32::sin(5. * PI / 3.)))
        );
    }

    #[test]
    fn test_generate_template_matrix() {
        let templates = arr2(&[
            [1., 1.],
            [0., 0.],
            [0., 0.],
            [0., 1.],
            [1., 0.],
            [0., 0.],
            [0., 0.],
            [1., 1.],
            [0., 0.],
            [0., 0.],
            [0., 0.],
            [0., 0.],
        ]);

        let expected_template = [
            1., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1.,
            0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.,
            0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.,
            0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1.,
            0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1.,
            0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.,
            0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.,
            0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 1.,
            0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0., 0., 1., 0.,
            0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0.,
            0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0.,
            0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0.,
            0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0.,
            0., 1.,
        ];
        let expected_template =
            Array::from_shape_vec((12, 24), expected_template.to_vec()).unwrap();
        let template_matrix = generate_template_matrix(&templates);
        assert_eq!(template_matrix, expected_template);
    }

    #[test]
    fn test_sort_by_fifths() {
        let file = File::open("data/filtered_features.npy").unwrap();
        let features = Array2::<f64>::read_npy(file).unwrap();
        let file = File::open("data/sorted_features.npy").unwrap();
        let expected_sorted = Array2::<f64>::read_npy(file).unwrap();

        let sorted = sort_by_fifths(&features, -1);
        for (expected, actual) in expected_sorted.iter().zip(sorted.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }
    }

    #[test]
    fn test_smooth_downsample_feature_sequence() {
        let file = File::open("data/chroma.npy").unwrap();
        let chroma = Array2::<f64>::read_npy(file).unwrap();
        let file = File::open("data/downsampled.npy").unwrap();
        let expected_downsampled = Array2::<f64>::read_npy(file).unwrap();

        let downsampled = smooth_downsample_feature_sequence(&chroma, 45, 15);
        for (expected, actual) in expected_downsampled.iter().zip(downsampled.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }
    }

    #[test]
    fn test_analysis_template_match() {
        let file = File::open("data/f_analysis_normalized.npy").unwrap();
        let expected_analysis = Array2::<f64>::read_npy(file).unwrap();

        let file = File::open("data/chroma.npy").unwrap();
        let chroma = Array2::<f64>::read_npy(file).unwrap();

        let templates = Array::from_shape_vec((12, 24), TEMPLATES_MAJMIN.to_vec()).unwrap();
        let analysis = analysis_template_match(&chroma, &templates, true);

        for (expected, actual) in expected_analysis.iter().zip(analysis.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }

        let analysis = analysis_template_match(&chroma, &templates, false);
        let file = File::open("data/f_analysis.npy").unwrap();
        let expected_analysis = Array2::<f64>::read_npy(file).unwrap();
        for (expected, actual) in expected_analysis.iter().zip(analysis.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }
    }

    #[test]
    fn test_normalize_feature_sequence() {
        let array = arr2(&[[0.1, 0.3, 0.4], [1.1, 0.53, 1.01]]);
        let expected_array = arr2(&[
            [0.09053575, 0.49259822, 0.36821425],
            [0.99589321, 0.87025686, 0.92974097],
        ]);

        let normalized_array = normalize_feature_sequence(&array);

        assert!(!array.is_empty() && !expected_array.is_empty());

        for (expected, actual) in normalized_array.iter().zip(expected_array.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }
    }

    #[test]
    fn test_chroma_desc() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut chroma_desc = ChromaDesc::new(song.sample_rate, 12);
        chroma_desc.do_(&song.sample_array);
        assert_eq!(
            chroma_desc.get_values(),
            (false, (f32::cos(5. * PI / 3.), f32::sin(5. * PI / 3.)))
        );
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
