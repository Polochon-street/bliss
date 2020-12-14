extern crate rustfft;
use ndarray::{s, Array, Array1};
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use realfft::{ComplexToReal, RealToComplex};

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
}
