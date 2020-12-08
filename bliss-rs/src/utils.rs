extern crate rustfft;
use ndarray::{arr1, s, Array, Array1};
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use rustfft::FFTplanner;

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

pub fn hz_to_octs(frequencies: &Array1<f64>, tuning: f64, bins_per_octave: u32) -> Array1<f64> {
    let a440 = 440.0 * (2_f64.powf(tuning / f64::from(bins_per_octave)) as f64);

    (frequencies / (a440 / 16.)).mapv(f64::log2)
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

pub fn convolve(input: &Array1<f64>, kernel: &Array1<f64>) -> Array1<f64> {
    let common_length = input.len() + kernel.len() - 1;
    let input = input.mapv(|x| Complex::new(x, 0.));
    let kernel = kernel.mapv(|x| Complex::new(x, 0.));
    let mut padded_input = Array::from_elem(common_length, Complex::new(0., 0.));
    padded_input.slice_mut(s![..input.len()]).assign(&input);
    let mut padded_kernel = Array::zeros(common_length);
    padded_kernel.slice_mut(s![..kernel.len()]).assign(&kernel);

    let mut input_fft: Vec<Complex<f64>> = vec![Complex::zero(); common_length];
    let mut kernel_fft: Vec<Complex<f64>> = vec![Complex::zero(); common_length];

    let mut planner = FFTplanner::new(false);
    let fft = planner.plan_fft(common_length);
    fft.process(&mut padded_input.to_vec(), &mut input_fft);
    fft.process(&mut padded_kernel.to_vec(), &mut kernel_fft);

    let mut multiplication = input_fft
        .iter()
        .zip(kernel_fft)
        .map(|(x, y)| x * y)
        .collect::<Vec<Complex<f64>>>();

    let mut planner = FFTplanner::new(true);
    let mut output: Vec<Complex<f64>> = vec![Complex::zero(); common_length];
    let fft = planner.plan_fft(common_length);
    fft.process(&mut multiplication, &mut output);

    let output = arr1(
        &output
            .iter()
            .map(|x| x.re / output.len() as f64)
            .collect::<Vec<f64>>(),
    );
    output.slice_move(s![
        (common_length - input.len()) / 2..(common_length - input.len()) / 2 + input.len()
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::{arr1, Array};
    use ndarray_npy::ReadNpyExt;
    use std::fs::File;

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
    fn test_hz_to_octs() {
        let frequencies = arr1(&[32., 64., 128., 256.]);
        let expected = arr1(&[0.16864029, 1.16864029, 2.16864029, 3.16864029]);

        let octs = hz_to_octs(&frequencies, 0.5, 10);
        octs.iter()
            .zip(expected.iter())
            .for_each(|(x, y)| assert!(0.0001 > (x - y).abs()));
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
}
