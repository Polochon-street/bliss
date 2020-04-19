//! Timbral feature extraction module.
//!
//! Contains functions to extract & summarize the zero-crossing rate,
//! spectral centroid, spectral flatness and spectral roll-off of
//! a given Song.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use aubio_rs::vec::CVec;
use aubio_rs::{bin_to_freq, PVoc, SpecDesc, SpecShape};

use super::utils::{geometric_mean, mean, number_crossings};

/**
 * General object holding all the spectral descriptor.
 *
 * Holds 3 spectral descriptors together. It would be better conceptually
 * to have 3 different spectral descriptor objects, but this avoids re-computing
 * the same FFT three times.
 *
 * Current spectral descriptors are spectral centroid, spectral rolloff and
 * spectral flatness (see `values_object` for a further description of the
 * object.
 *
 * All descriptors are currently summarized by their mean only.
 */
pub struct SpectralDesc {
    phase_vocoder: PVoc,
    sample_rate: u32,

    centroid_aubio_desc: SpecDesc,
    rolloff_aubio_desc: SpecDesc,
    values_centroid: Vec<f32>,
    values_rolloff: Vec<f32>,
    values_flatness: Vec<f32>,
}

impl SpectralDesc {
    pub const WINDOW_SIZE: usize = 512;
    pub const HOP_SIZE: usize = SpectralDesc::WINDOW_SIZE / 4;

    /**
     * Compute score related to the
     * [spectral centroid](https://en.wikipedia.org/wiki/Spectral_centroid) values,
     * obtained after repeatedly calling `do_` on all of the song's chunks.
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
    pub fn get_centroid(&mut self) -> f32 {
        mean(&self.values_centroid)
    }

    /**
     * Compute score related to the spectral roll-off values, obtained
     * after repeatedly calling `do_` on all of the song's chunks.
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
    pub fn get_rolloff(&mut self) -> f32 {
        mean(&self.values_rolloff)
    }

    /**
     * Compute score related to the
     * [spectral flatness](https://en.wikipedia.org/wiki/Spectral_flatness) values,
     * obtained after repeatedly calling `do_` on all of the song's chunks.
     *
     * Spectral flatness is the ratio between the geometric mean of the spectrum
     * and its arithmetic mean.
     *
     * It is used to distinguish between tone-like and noise-like signals.
     * Tone-like audio is f.ex. a piano key, something that has one or more
     * specific frequencies, while (white) noise has an equal distribution
     * of intensity among all frequencies.
     *
     * The value range is between 0 and 1, since the geometric mean is always less
     * than the arithmetic mean.
     */
    pub fn get_flatness(&mut self) -> f32 {
        mean(&self.values_flatness)
    }

    pub fn new(sample_rate: u32) -> Self {
        SpectralDesc {
            centroid_aubio_desc: SpecDesc::new(SpecShape::Centroid, SpectralDesc::WINDOW_SIZE)
                .unwrap(),
            rolloff_aubio_desc: SpecDesc::new(SpecShape::Rolloff, SpectralDesc::WINDOW_SIZE)
                .unwrap(),
            phase_vocoder: PVoc::new(SpectralDesc::WINDOW_SIZE, SpectralDesc::HOP_SIZE).unwrap(),
            // TODO vec with capacity?
            values_centroid: Vec::new(),
            values_rolloff: Vec::new(),
            values_flatness: Vec::new(),
            sample_rate,
        }
    }

    /**
     * Compute all the descriptors' value for the given chunk.
     *
     * After using this on all the song's chunks, you can call
     * `get_centroid`, `get_flatness` and `get_rolloff` to get the respective
     * descriptors' values.
     */
    pub fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; SpectralDesc::WINDOW_SIZE + 2];
        self.phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();

        let bin = self
            .centroid_aubio_desc
            .do_result(fftgrain.as_slice())
            .unwrap();
        let freq = bin_to_freq(
            bin,
            self.sample_rate as f32,
            SpectralDesc::WINDOW_SIZE as f32,
        );
        self.values_centroid.push(freq);

        let bin = self
            .rolloff_aubio_desc
            .do_result(fftgrain.as_slice())
            .unwrap();
        let freq = bin_to_freq(
            bin,
            self.sample_rate as f32,
            SpectralDesc::WINDOW_SIZE as f32,
        );
        self.values_rolloff.push(freq);

        let cvec: CVec = fftgrain.as_slice().into();
        let geo_mean = geometric_mean(&cvec.norm());
        if geo_mean == 0.0 {
            self.values_flatness.push(0.0);
            return;
        }
        let flatness = geo_mean / mean(&cvec.norm());
        self.values_flatness.push(flatness);
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

impl ZeroCrossingRateDesc {
    pub fn new(_sample_rate: u32) -> Self {
        ZeroCrossingRateDesc::default()
    }

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
        let mut spectral_desc = SpectralDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            spectral_desc.do_(&chunk);
        }
        // Spectral flatness value computed here with phase vocoder: 0.111949615
        // Essentia value with spectrum / hann window: 0.11197535695207445
        assert!(0.01 > (0.11 - spectral_desc.get_flatness()).abs());
    }

    #[test]
    fn test_spectral_roll_off() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut spectral_desc = SpectralDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            spectral_desc.do_(&chunk);
        }
        // Roll-off value computed here with phase vocoder: 2026.7644
        // Essentia value with spectrum / hann window: 1979.632683520047
        assert!(0.01 > (2026.76 - spectral_desc.get_rolloff()).abs());
    }

    #[test]
    fn test_spectral_centroid() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut spectral_desc = SpectralDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(SpectralDesc::HOP_SIZE) {
            spectral_desc.do_(&chunk);
        }
        // Spectral centroid value computed here with phase vocoder: 1354.2273
        // Essentia value with spectrum / hann window: 1351
        assert!(0.01 > (1354.2273 - spectral_desc.get_centroid()).abs());
    }
}
