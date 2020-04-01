//! Song analysis module.
//!
//! Contains various functions to extract meaningful features from a `Song`,
//! e.g. tempo features, spectral features, etc.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use aubio_rs::{bin_to_freq, silence_detection, OnsetMode, PVoc, SpecDesc, SpecShape, Tempo};

use super::{Analysis, Song};

pub fn analyze_song(song: Song) -> Analysis {
    Analysis {
        tempo: get_tempo(&song),
        spectral_centroid: get_centroid(&song),
    }
}

/**
 * Compute score related to tempo.
 * Right now, basically returns the song's BPM.
 *
 * - `song` Song to compute score from
 */
fn get_tempo(song: &Song) -> f32 {
    const WINDOW_SIZE: usize = 1024;
    const HOP_SIZE: usize = 256;
    let mut tempo =
        Tempo::new(OnsetMode::SpecDiff, WINDOW_SIZE, HOP_SIZE, song.sample_rate).unwrap();
    for chunk in song.sample_array.chunks(HOP_SIZE) {
        tempo.do_result(chunk).unwrap();
    }
    tempo.get_bpm()
}

/**
 * Compute score related to spectral centroid.
 * Returns the mean of computed spectral centroids over the song.
 *
 * - `song` Song to compute score from
 */
fn get_centroid(song: &Song) -> f32 {
    const WINDOW_SIZE: usize = 512;
    let hop_size = WINDOW_SIZE / 4;
    let silence_threshold = -60.;
    let mut centroid = SpecDesc::new(SpecShape::Centroid, WINDOW_SIZE).unwrap();
    let mut phase_vocoder = PVoc::new(WINDOW_SIZE, hop_size).unwrap();

    let mut freqs: Vec<f32> = Vec::with_capacity(song.sample_array.chunks(hop_size).len());
    for chunk in song.sample_array.chunks(hop_size) {
        let mut fftgrain: Vec<f32> = vec![0.0; WINDOW_SIZE + 2];
        // If silence, centroid will be off, so skip instead
        if silence_detection(chunk, silence_threshold) {
            continue;
        }
        phase_vocoder.do_(chunk, fftgrain.as_mut_slice()).unwrap();
        let bin = centroid.do_result(fftgrain.as_slice()).unwrap();
        let freq = bin_to_freq(bin, song.sample_rate as f32, WINDOW_SIZE as f32);
        freqs.push(freq);
    }

    // return mean
    freqs.iter().sum::<f32>() / freqs.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn tempo() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        assert!(0.01 > (142.38 - get_tempo(&song)).abs());
    }

    #[test]
    fn centroid() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        assert!(0.01 > (1236.39 - get_centroid(&song).abs()));
    }
}
