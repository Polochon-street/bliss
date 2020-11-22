//! Song analysis module.
//!
//! Use features-extraction functions from other modules
//! e.g. tempo features, spectral features, etc to build an
//! Analysis of a Song
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

extern crate ndarray;
extern crate ndarray_npy;

use std::f64::consts::PI;

use aubio_rs::vec::CVec;
use aubio_rs::FFT;
use ndarray::{arr1, s, stack, Array, Array1, Array2, Axis};

use crate::chroma::ChromaDesc;
use crate::decode::decode_song;
use crate::misc::LoudnessDesc;
use crate::temporal::BPMDesc;
use crate::timbral::{SpectralDesc, ZeroCrossingRateDesc};
use crate::{Analysis, Song};

pub fn decode_and_analyze(path: &str) -> Result<Song, String> {
    // TODO error handling here
    let mut song = decode_song(&path)?;

    song.analysis = analyze(&song);
    Ok(song)
}

fn reflect_pad(array: &[f32], pad: usize) -> Vec<f32> {
    let mut prefix = array[1..=pad].iter().rev().copied().collect::<Vec<f32>>();
    let suffix = array[(array.len() - 2) - pad + 1..array.len() - 1]
        .iter()
        .rev()
        .copied()
        .collect::<Vec<f32>>();
    prefix.extend(array);
    prefix.extend(suffix);
    prefix
}

pub fn stft(signal: &[f32], window_length: usize, hop_length: usize) -> Array2<f64> {
    let mut fft = FFT::new(window_length).unwrap();

    let signal = reflect_pad(&signal, window_length / 2);
    let mut stft = Array2::zeros((window_length / 2 + 1, 0));

    // TODO actually have it constant - no need to compute it everytime
    // Periodic, so window_size + 1
    let mut hann_window = Array::zeros(window_length + 1);
    for n in 0..window_length {
        hann_window[[n]] = 0.5 - 0.5 * f64::cos(2. * n as f64 * PI / (window_length as f64));
    }
    hann_window = hann_window.slice_move(s![0..window_length]);
    for i in 1..signal.len() {
        if i >= window_length && (i - window_length) % hop_length == 0 {
            let beginning = i - window_length;
            let end = i;
            let mut fftgrain: Vec<f32> = vec![0.0; window_length + 2];
            // TODO directly apply hann window maybe
            let signal = (arr1(&signal[beginning..end]).mapv(f64::from) * &hann_window)
                .mapv(|x| x as f32)
                .to_vec();
            fft.do_(&signal, fftgrain.as_mut_slice()).unwrap();
            let cvec: CVec = fftgrain.as_slice().into();
            let norm: Array1<f32> = arr1(&cvec.norm());
            stft = stack![Axis(1), stft, norm.insert_axis(Axis(1))];
        }
    }
    stft.mapv(f64::from)
}

pub fn analyze(song: &Song) -> Analysis {
    let mut spectral_desc = SpectralDesc::new(song.sample_rate);
    let mut zcr_desc = ZeroCrossingRateDesc::default();
    let mut tempo_desc = BPMDesc::new(song.sample_rate);
    let mut loudness_desc = LoudnessDesc::default();
    let mut chroma_desc = ChromaDesc::new(song.sample_rate, 12);

    // These descriptors can be streamed
    for i in 1..song.sample_array.len() {
        if (i % SpectralDesc::HOP_SIZE) == 0 {
            let beginning = (i / SpectralDesc::HOP_SIZE - 1) * SpectralDesc::HOP_SIZE;
            let end = i;
            spectral_desc.do_(&song.sample_array[beginning..end]);
            zcr_desc.do_(&song.sample_array[beginning..end]);
        }

        if (i % BPMDesc::HOP_SIZE) == 0 {
            let beginning = (i / BPMDesc::HOP_SIZE - 1) * BPMDesc::HOP_SIZE;
            let end = i;
            tempo_desc.do_(&song.sample_array[beginning..end]);
        }

        // Contiguous windows, so WINDOW_SIZE here
        if (i % LoudnessDesc::WINDOW_SIZE) == 0 {
            let beginning = (i / LoudnessDesc::WINDOW_SIZE - 1) * LoudnessDesc::WINDOW_SIZE;
            let end = i;
            loudness_desc.do_(&song.sample_array[beginning..end]);
        }
    }
    // Non-streaming approach for that one
    chroma_desc.do_(&song.sample_array);
    let (is_major, fifth) = chroma_desc.get_values();

    Analysis {
        tempo: tempo_desc.get_value(),
        spectral_centroid: spectral_desc.get_centroid(),
        zero_crossing_rate: zcr_desc.get_value(),
        spectral_rolloff: spectral_desc.get_rolloff(),
        spectral_flatness: spectral_desc.get_flatness(),
        loudness: loudness_desc.get_value(),
        is_major,
        fifth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;
    use ndarray::Array2;
    use ndarray_npy::ReadNpyExt;
    use std::fs::File;
    use std::f32::consts::PI;

    #[test]
    fn test_analyze() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let expected_analysis = Analysis {
            tempo: 0.37860596,
            spectral_centroid: -0.75483,
            zero_crossing_rate: -0.85036564,
            spectral_rolloff: -0.6326486,
            spectral_flatness: -0.77610075,
            loudness: 0.27126348,
            is_major: -1.,
            fifth: (f32::cos(5. * PI / 3.), f32::sin(5. * PI / 3.)),
        };
        assert!(expected_analysis.approx_eq(&analyze(&song)));
    }

    #[test]
    fn test_compute_stft() {
        let file = File::open("data/librosa-stft.npy").unwrap();
        let expected_stft = Array2::<f32>::read_npy(file).unwrap().mapv(|x| x as f64);

        let song = decode_song("data/piano.flac").unwrap();

        let stft = stft(&song.sample_array, 2048, 512);

        assert!(!stft.is_empty() && !expected_stft.is_empty());
        for (expected, actual) in expected_stft.iter().zip(stft.iter()) {
            assert!(0.0001 > (expected - actual).abs());
        }
    }
}
