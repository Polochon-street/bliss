extern crate rustfft;
use ndarray::{s, arr1, Array, Array1, Array2};
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use realfft::{ComplexToReal, RealToComplex};
use super::CHANNELS;
extern crate ffmpeg4 as ffmpeg;
use ffmpeg::util;
use ffmpeg::util::format::sample::{Sample, Type};
use std::f32::consts::PI;

// Until https://github.com/rust-ndarray/ndarray/issues/446 is solved
pub const TEMPLATES_MAJMIN: [f64; 12 * 24] = [
    1., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0.,
    0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0.,
    0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1.,
    0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0.,
    1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0.,
    0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 1., 0.,
    0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., 1.,
    1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0.,
    0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0.,
    0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0., 0.,
    0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1., 0.,
    0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1.,
];

pub fn reflect_pad(array: &[f32], pad: usize) -> Vec<f32> {
    let prefix = array[1..=pad].iter().rev().copied().collect::<Vec<f32>>();
    let suffix = array[(array.len() - 2) - pad + 1..array.len() - 1]
        .iter()
        .rev()
        .copied()
        .collect::<Vec<f32>>();
    let mut output = Vec::with_capacity(prefix.len() + array.len() + suffix.len());

    output.extend(prefix);
    output.extend(array);
    output.extend(suffix);
    output
}

pub fn stft(signal: &[f32], window_length: usize, hop_length: usize) -> Array2<f64> {
    // Take advantage of raw-major order to have contiguous window for the
    // `assign`, reversing the axes to have the expected shape at the end only.
    let mut stft = Array2::zeros((
        (signal.len() as f32 / hop_length as f32).ceil() as usize,
        window_length / 2 + 1,
    ));
    let signal = reflect_pad(&signal, window_length / 2);

    // TODO actually have it constant - no need to compute it everytime
    // Periodic, so window_size + 1
    let mut hann_window = Array::zeros(window_length + 1);
    for n in 0..window_length {
        hann_window[[n]] = 0.5 - 0.5 * f32::cos(2. * n as f32 * PI / (window_length as f32));
    }
    hann_window = hann_window.slice_move(s![0..window_length]);
    let mut output_window = Array::from_elem(window_length / 2 + 1, Complex::zero());
    let mut r2c = RealToComplex::<f32>::new(window_length).unwrap();

    for (window, mut stft_col) in signal
        .windows(window_length)
        .step_by(hop_length)
        .zip(stft.genrows_mut())
    {
        let mut signal = arr1(&window) * &hann_window;
        r2c.process(
            signal.as_slice_mut().unwrap(),
            output_window.as_slice_mut().unwrap(),
        )
        .unwrap();
        stft_col.assign(
            &output_window
                .slice(s![..window_length / 2 + 1])
                .mapv(|x| (x.re * x.re + x.im * x.im).sqrt() as f64),
        );
    }
    stft.permuted_axes((1, 0))
}

pub fn push_to_sample_array(frame: ffmpeg::frame::Audio, sample_array: &mut Vec<f32>) {
    if frame.samples() == 0 {
        return
    }
    // Account for the padding
    let actual_size = util::format::sample::Buffer::size(
        Sample::F32(Type::Packed),
        CHANNELS,
        frame.samples(),
        false,
    );
    let f32_frame: Vec<f32> = frame.data(0)[..actual_size]
        .chunks_exact(4)
        .map(|x| {
            let mut a: [u8; 4] = [0; 4];
            a.copy_from_slice(x);
            f32::from_le_bytes(a)
        })
        .collect();
    sample_array.extend_from_slice(&f32_frame);
}

pub fn mean<T: Clone + Into<f32>>(input: &[T]) -> f32 {
    input.iter().map(|x| x.clone().into() as f32).sum::<f32>() / input.len() as f32
}

pub trait Normalize {
    const MAX_VALUE: f32;
    const MIN_VALUE: f32;

    fn normalize(&self, value: f32) -> f32 {
        2. * (value - Self::MIN_VALUE) / (Self::MAX_VALUE - Self::MIN_VALUE) - 1.
    }
}

// Essentia algorithm
// https://github.com/MTG/essentia/blob/master/src/algorithms/temporal/zerocrossingrate.cpp
pub fn number_crossings(input: &[f32]) -> u32 {
    let mut crossings = 0;
    let mut val = input[0];

    if val.abs() < 0. {
        val = 0.
    };
    let mut was_positive = val > 0.;
    let mut is_positive: bool;

    for sample in input {
        val = *sample;
        if val.abs() <= 0.0 {
            val = 0.0
        };
        is_positive = val > 0.;
        if was_positive != is_positive {
            crossings += 1;
            was_positive = is_positive;
        }
    }

    crossings
}

pub fn geometric_mean(input: &[f32]) -> f32 {
    let mut mean = 0.0;
    for &sample in input {
        if sample == 0.0 {
            return 0.0;
        }
        mean += sample.ln();
    }
    mean /= input.len() as f32;
    mean.exp()
}

pub fn hz_to_octs_inplace(
    frequencies: &mut Array1<f64>,
    tuning: f64,
    bins_per_octave: u32,
) -> &mut Array1<f64> {
    let a440 = 440.0 * (2_f64.powf(tuning / f64::from(bins_per_octave)) as f64);

    *frequencies /= a440 / 16.;
    frequencies.mapv_inplace(f64::log2);
    frequencies
}

// TODO try to make this FFT real only
// TODO have less buffers - technically, we probably only need two
pub fn convolve(input: &Array1<f64>, kernel: &Array1<f64>) -> Array1<f64> {
    let mut common_length = input.len() + kernel.len();
    if (common_length % 2) != 0 {
        common_length -= 1;
    }
    let mut padded_input = Array::zeros(common_length);
    padded_input.slice_mut(s![..input.len()]).assign(&input);
    let mut padded_kernel = Array::zeros(common_length);
    padded_kernel.slice_mut(s![..kernel.len()]).assign(&kernel);

    let mut input_fft = Array::from_elem(common_length / 2 + 1, Complex::zero());
    let mut kernel_fft = Array::from_elem(common_length / 2 + 1, Complex::zero());

    let mut r2c = RealToComplex::<f64>::new(common_length).unwrap();
    r2c.process(padded_input.as_slice_mut().unwrap(), input_fft.as_slice_mut().unwrap()).unwrap();
    r2c.process(padded_kernel.as_slice_mut().unwrap(), kernel_fft.as_slice_mut().unwrap()).unwrap();

    let mut multiplication = input_fft * kernel_fft;

    let mut c2r = ComplexToReal::<f64>::new(common_length).unwrap();
    let mut output = Array::zeros(common_length);
    c2r.process(multiplication.as_slice_mut().unwrap(), output.as_slice_mut().unwrap()).unwrap();

    let output_length = output.len() as f64;
    let output = output.slice_move(s![
        (kernel.len() - 1) / 2..(kernel.len() - 1) / 2 + input.len()
    ]);
    output / (output_length / 2.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::{arr1, Array};
    use ndarray_npy::ReadNpyExt;
    use std::fs::File;
    use crate::Song;
    use ndarray::Array2;


    #[test]
    fn test_convolve() {
        let file = File::open("data/convolve.npy").unwrap();
        let expected_convolve = Array1::<f64>::read_npy(file).unwrap();
        let input: Array1<f64> = Array::range(0., 1000., 0.5);
        let kernel: Array1<f64> = Array::ones(100);

        let output = convolve(&input, &kernel);
        for (expected, actual) in expected_convolve.iter().zip(output.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }

        let input: Array1<f64> = Array::range(0., 1000., 0.5);
        let file = File::open("data/convolve_odd.npy").unwrap();
        let expected_convolve = Array1::<f64>::read_npy(file).unwrap();
        let kernel: Array1<f64> = Array::ones(99);

        let output = convolve(&input, &kernel);
        for (expected, actual) in expected_convolve.iter().zip(output.iter()) {
            assert!(0.0000001 > (expected - actual).abs());
        }
    }

    #[test]
    fn test_mean() {
        let numbers = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        assert_eq!(2.0, mean(&numbers));
    }

    #[test]
    fn test_geometric_mean() {
        let numbers = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        assert_eq!(0.0, geometric_mean(&numbers));

        let numbers = vec![4.0, 1.0, 0.03125];
        assert_eq!(0.5, geometric_mean(&numbers));
    }

    #[test]
    fn test_hz_to_octs_inplace() {
        let mut frequencies = arr1(&[32., 64., 128., 256.]);
        let expected = arr1(&[0.16864029, 1.16864029, 2.16864029, 3.16864029]);

        hz_to_octs_inplace(&mut frequencies, 0.5, 10)
            .iter()
            .zip(expected.iter())
            .for_each(|(x, y)| assert!(0.0001 > (x - y).abs()));
    }

    #[test]
    fn test_compute_stft() {
        let file = File::open("data/librosa-stft.npy").unwrap();
        let expected_stft = Array2::<f32>::read_npy(file).unwrap().mapv(|x| x as f64);

        let song = Song::decode("data/piano.flac").unwrap();

        let stft = stft(&song.sample_array, 2048, 512);

        assert!(!stft.is_empty() && !expected_stft.is_empty());
        for (expected, actual) in expected_stft.iter().zip(stft.iter()) {
            assert!(0.0001 > (expected - actual).abs());
        }
    }

    #[test]
    fn test_reflect_pad() {
        let array = Array::range(0., 100000., 1.);

        let output = reflect_pad(array.as_slice().unwrap(), 3);
        assert_eq!(&output[..4], &[3.0, 2.0, 1.0, 0.]);
        assert_eq!(&output[3..100003], array.to_vec());
        assert_eq!(&output[100003..100006], &[99998.0, 99997.0, 99996.0]);
    }

}
