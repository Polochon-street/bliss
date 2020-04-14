//! Spectral feature extraction module.
//!
//! Contains functions to extract & summarize zero-crossing rate,
//! spectral centroid, spectral flatness and spectral roll-off of
//! a given Song.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use aubio_rs::{bin_to_freq, silence_detection, PVoc, SpecDesc, SpecShape};

use super::utils::{mean, number_crossings};

/**
 * [Zero-crossing rate](https://en.wikipedia.org/wiki/Zero-crossing_rate)
 * detection object.
 *
 * Zero-crossing rate is mostly used to detect percussive sounds in an audio
 * signal, as well as whether an audio signal contains speech or not.
 * 
 * It is a good metric to differentiate between songs with people speaking clearly,
 * (e.g. slam) and instrumental songs.
 */
#[derive(Default)]
pub struct ZeroCrossingRateDesc {
    values: Vec<u32>,
    number_samples: usize,
}

impl ZeroCrossingRateDesc {
    pub const HOP_SIZE: usize = 1024;

    /// Count the number of zero-crossings for the current `chunk`.
    pub fn do_(&mut self, chunk: &[f32]) {
        self.values.push(number_crossings(chunk));
        self.number_samples += chunk.len();
    }

    /// Sum the number of zero-crossings witnessed and divide by
    /// the total number of samples.
    pub fn get_value(&mut self) -> f32 {
        (self.values.iter().sum::<u32>()) as f32 / self.number_samples as f32
    }
}

pub struct SpectralDesc {
    aubio_obj: SpecDesc,
    phase_vocoder: PVoc,
    // Values before being summarized through f.ex. a mean
    values: Vec<f32>,
    sample_rate: u32,
}

// TODO change naming
impl SpectralDesc {
    pub const WINDOW_SIZE: usize = 512;
    pub const HOP_SIZE: usize = SpectralDesc::WINDOW_SIZE / 4;

    /**
     * Compute score related to spectral centroid.
     * Returns the mean of computed spectral centroids over the song.
     *
     * - `song` Song to compute score from
     */
    pub fn get_value(&mut self) -> f32 {
        mean(&self.values)
    }

    pub fn new(shape: SpecShape, sample_rate: u32) -> Self {
        SpectralDesc {
            aubio_obj: SpecDesc::new(shape, SpectralDesc::WINDOW_SIZE).unwrap(),
            phase_vocoder: PVoc::new(SpectralDesc::WINDOW_SIZE, SpectralDesc::HOP_SIZE).unwrap(),
            // TODO vec with capacity?
            values: Vec::new(),
            sample_rate,
        }
    }
}

pub trait SpectralDescriptor {
    fn new(sample_rate: u32) -> Self;
    fn do_(&mut self, chunk: &[f32]);
    fn get_value(&mut self) -> f32;
}

pub struct SpectralFlatnessDesc {
    spectral_desc: SpectralDesc,
}

pub struct SpectralRollOffDesc {
    spectral_desc: SpectralDesc,
}

pub struct SpectralCentroidDesc {
    spectral_desc: SpectralDesc,
}

impl SpectralDescriptor for SpectralFlatnessDesc {
    fn new(sample_rate: u32) -> Self {
        SpectralFlatnessDesc {
            spectral_desc: SpectralDesc::new(SpecShape::Kurtosis, sample_rate),
        }
    }

    fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; SpectralDesc::WINDOW_SIZE + 2];

        self.spectral_desc
            .phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let flatness = self
            .spectral_desc
            .aubio_obj
            .do_result(fftgrain.as_slice())
            .unwrap();
        self.spectral_desc.values.push(flatness);
    }

    /**
     * Compute score related to spectral centroid.
     * Returns the mean of computed spectral centroids over the song.
     *
     * - `song` Song to compute score from
     */
    // TODO do we really want the mean there?
    fn get_value(&mut self) -> f32 {
        self.spectral_desc.get_value()
    }
}

impl SpectralDescriptor for SpectralRollOffDesc {
    fn new(sample_rate: u32) -> Self {
        SpectralRollOffDesc {
            spectral_desc: SpectralDesc::new(SpecShape::Rolloff, sample_rate),
        }
    }

    fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; SpectralDesc::WINDOW_SIZE + 2];

        self.spectral_desc
            .phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let bin = self
            .spectral_desc
            .aubio_obj
            .do_result(fftgrain.as_slice())
            .unwrap();
        let freq = bin_to_freq(
            bin,
            self.spectral_desc.sample_rate as f32,
            SpectralDesc::WINDOW_SIZE as f32,
        );
        self.spectral_desc.values.push(freq);
    }

    fn get_value(&mut self) -> f32 {
        self.spectral_desc.get_value()
    }
}

impl SpectralDescriptor for SpectralCentroidDesc {
    fn new(sample_rate: u32) -> Self {
        SpectralCentroidDesc {
            spectral_desc: SpectralDesc::new(SpecShape::Centroid, sample_rate),
        }
    }

    fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; SpectralDesc::WINDOW_SIZE + 2];
        // If silence, centroid will be off, so skip instead
        if silence_detection(chunk, -60.0) {
            return;
        }

        self.spectral_desc
            .phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let bin = self
            .spectral_desc
            .aubio_obj
            .do_result(fftgrain.as_slice())
            .unwrap();
        let freq = bin_to_freq(
            bin,
            self.spectral_desc.sample_rate as f32,
            SpectralDesc::WINDOW_SIZE as f32,
        );
        self.spectral_desc.values.push(freq);
    }

    fn get_value(&mut self) -> f32 {
        self.spectral_desc.get_value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn test_zcr() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut zcr_desc = ZeroCrossingRateDesc::default();
        for chunk in song.sample_array.chunks_exact(ZeroCrossingRateDesc::HOP_SIZE) {
            zcr_desc.do_(&chunk);
        }
        assert!(0.001 > (0.075 - zcr_desc.get_value()).abs());
    }

    #[test]
    fn test_flatness() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut flatness_desc = SpectralFlatnessDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            flatness_desc.do_(&chunk);
        }
        assert!(0.01 > (12.74 - flatness_desc.get_value()).abs());
    }

    #[test]
    fn test_roll_off() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut rolloff_desc = SpectralRollOffDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            rolloff_desc.do_(&chunk);
        }
        println!("{}", rolloff_desc.get_value());
        assert!(0.01 > (2026.76 - rolloff_desc.get_value()).abs());
    }

    #[test]
    fn test_centroid() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut centroid_desc = SpectralCentroidDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            centroid_desc.do_(&chunk);
        }
        assert!(0.01 > (1236.37 - centroid_desc.get_value()).abs());
    }
}
