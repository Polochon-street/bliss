//! Spectral feature extraction module.
//!
//! Contains functions to extract & summarize zero-crossing rate,
//! spectral centroid, spectral flatness and spectral roll-off of
//! a given Song.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use aubio_rs::{bin_to_freq, PVoc, SpecDesc, SpecShape};
use aubio_rs::vec::{CVec};

use super::utils::{geometric_mean, mean, number_crossings};
use super::Descriptor;


pub struct SpectralDesc {
    aubio_obj: SpecDesc,
    phase_vocoder: PVoc,
    // Values before being summarized through f.ex. a mean
    values: Vec<f32>,
    sample_rate: u32,
}

impl SpectralDesc {
    pub const WINDOW_SIZE: usize = 512;
    pub const HOP_SIZE: usize = SpectralDesc::WINDOW_SIZE / 4;

    /**
     * Compute score related to the spectral descriptor.
     *
     * Currently returns the mean of all the chunks' values.
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

/**
 * [Zero-crossing rate](https://en.wikipedia.org/wiki/Zero-crossing_rate)
 * detection object.
 *
 * Zero-crossing rate is mostly used to detect percussive sounds in an audio
 * signal, as well as whether an audio signal contains speech or not.
 * 
 * It is a good metric to differentiate between songs with people speaking clearly,
 * (e.g. slam) and instrumental songs.
 *
 * The value range is between 0 and 1.
 */
#[derive(Default)]
pub struct ZeroCrossingRateDesc {
    values: Vec<u32>,
    number_samples: usize,
}

/**
 * [Spectral centroid](https://en.wikipedia.org/wiki/Spectral_centroid)
 * detection object.
 *
 * Spectral centroid is used to determine the "brightness" of a sound, i.e.
 * how much high frequency there is in an audio signal.
 *
 * It of course depends of the instrument used: a piano-only track that makes
 * use of high frequencies will still score less than a song using a lot of
 * percussive sound, because the piano frequency range is lower.
 *
 * The value range is between 0 and `sample_rate / 2`.
 */
pub struct SpectralCentroidDesc {
    spectral_desc: SpectralDesc,
}

/**
 * Spectral roll-off detection object.
 *
 * Spectral roll-off is the bin frequency number below which a certain
 * percentage of the spectral energy is found, here, 95%.
 *
 * It can be used to distinguish voiced speech (low roll-off) and unvoiced
 * speech (high roll-off). It is also a good indication of the energy
 * repartition of a song.
 *
 * The value range is between 0 and `sample_rate / 2`
 */
// TODO is it really relevant to use?
pub struct SpectralRollOffDesc {
    spectral_desc: SpectralDesc,
}

pub struct SpectralFlatnessDesc {
    spectral_desc: SpectralDesc,
}

impl Descriptor for ZeroCrossingRateDesc {
    fn new(_sample_rate: u32) -> Self {
        ZeroCrossingRateDesc::default()
    }

    /// Count the number of zero-crossings for the current `chunk`.
    fn do_(&mut self, chunk: &[f32]) {
        self.values.push(number_crossings(chunk));
        self.number_samples += chunk.len();
    }

    /// Sum the number of zero-crossings witnessed and divide by
    /// the total number of samples.
    fn get_value(&mut self) -> f32 {
        (self.values.iter().sum::<u32>()) as f32 / self.number_samples as f32
    }
}

impl Descriptor for SpectralCentroidDesc {
    fn new(sample_rate: u32) -> Self {
        SpectralCentroidDesc {
            spectral_desc: SpectralDesc::new(SpecShape::Centroid, sample_rate),
        }
    }

    // TODO make FFT computation common for all spectral descs
    /// Compute FFT and associated spectral centroid for the current chunk.
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

impl Descriptor for SpectralRollOffDesc {
    fn new(sample_rate: u32) -> Self {
        SpectralRollOffDesc {
            spectral_desc: SpectralDesc::new(SpecShape::Rolloff, sample_rate),
        }
    }

    /// Compute FFT and associated spectral roll-off for the current chunk.
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

impl Descriptor for SpectralFlatnessDesc {
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
        let cvec: CVec = fftgrain.as_slice().into();
        let flatness =  geometric_mean(&cvec.norm()) / mean(&cvec.norm());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn test_zcr() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut zcr_desc = ZeroCrossingRateDesc::default();
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            zcr_desc.do_(&chunk);
        }
        assert!(0.001 > (0.075 - zcr_desc.get_value()).abs());
    }

    #[test]
    fn test_spectral_flatness() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut flatness_desc = SpectralFlatnessDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            flatness_desc.do_(&chunk);
        }
        // Spectral flatness value computed here with phase vocoder: 0.111949615
        // Essentia value with spectrum / hann window: 0.11197535695207445
        assert!(0.01 > (0.11 - flatness_desc.get_value()).abs());
    }

    #[test]
    fn test_spectral_roll_off() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut roll_off_desc = SpectralRollOffDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            roll_off_desc.do_(&chunk);
        }
        // Roll-off value computed here with phase vocoder: 2026.7644
        // Essentia value with spectrum / hann window: 1979.632683520047
        assert!(0.01 > (2026.76 - roll_off_desc.get_value()).abs());
    }

    #[test]
    fn test_spectral_centroid() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut centroid_desc = SpectralCentroidDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            centroid_desc.do_(&chunk);
        }
        // Spectral centroid value computed here with phase vocoder: 1354.2273
        // Essentia value with spectrum / hann window: 1351
        assert!(0.01 > (1354.2273 - centroid_desc.get_value()).abs());
    }
}
