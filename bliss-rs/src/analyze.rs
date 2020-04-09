//! Song analysis module.
//!
//! Contains various functions to extract meaningful features from a `Song`,
//! e.g. tempo features, spectral features, etc.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use aubio_rs::{bin_to_freq, silence_detection, OnsetMode, PVoc, SpecDesc, SpecShape, Tempo};

use super::utils::{mean, number_crossings};

struct ZeroCrossingRateDesc {
    values: Vec<u32>,
    number_samples: usize,
}

impl ZeroCrossingRateDesc {
    const HOP_SIZE: usize = 1024;

    pub fn new() -> Self {
        ZeroCrossingRateDesc {
            values: Vec::new(),
            number_samples: 0,
        }
    }

    pub fn do_(&mut self, chunk: &[f32]) {
        self.values.push(number_crossings(chunk));
        self.number_samples += chunk.len();
    }

    pub fn get_value(&mut self) -> f32 {
        (self.values.iter().sum::<u32>()) as f32 / self.number_samples as f32
    }
}

// TODO write proper doc
struct TempoDesc {
    aubio_obj: Tempo,
}

impl TempoDesc {
    const WINDOW_SIZE: usize = 1024;
    const HOP_SIZE: usize = TempoDesc::WINDOW_SIZE / 4;

    pub fn new(sample_rate: u32) -> Self {
        TempoDesc {
            aubio_obj: Tempo::new(
                OnsetMode::SpecDiff,
                TempoDesc::WINDOW_SIZE,
                TempoDesc::HOP_SIZE,
                sample_rate,
            )
            .unwrap(),
        }
    }

    pub fn do_(&mut self, chunk: &[f32]) {
        self.aubio_obj.do_result(chunk).unwrap();
    }

    /**
     * Compute score related to tempo.
     * Right now, basically returns the song's BPM.
     *
     * - `song` Song to compute score from
     */
    pub fn get_value(&mut self) -> f32 {
        self.aubio_obj.get_bpm()
    }
}

struct SpectralFlatnessDesc {
    aubio_obj: SpecDesc,
    phase_vocoder: PVoc,
    // Values before being summarized through f.ex. a mean
    values: Vec<f32>,
    sample_rate: u32,
}

impl SpectralFlatnessDesc{
    const WINDOW_SIZE: usize = 512;
    const HOP_SIZE: usize = SpectralFlatnessDesc::WINDOW_SIZE / 4;

    pub fn new(sample_rate: u32) -> Self {
        SpectralFlatnessDesc {
            aubio_obj: SpecDesc::new(SpecShape::Kurtosis, SpectralFlatnessDesc::WINDOW_SIZE)
                .unwrap(),
            phase_vocoder: PVoc::new(
                SpectralFlatnessDesc::WINDOW_SIZE,
                SpectralFlatnessDesc::HOP_SIZE,
            )
            .unwrap(),
            values: Vec::new(),
            sample_rate,
        }
    }

    pub fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; SpectralFlatnessDesc::WINDOW_SIZE + 2];

        self.phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let flatness = self.aubio_obj.do_result(fftgrain.as_slice()).unwrap();
        self.values.push(flatness);
    }

    /**
     * Compute score related to spectral centroid.
     * Returns the mean of computed spectral centroids over the song.
     *
     * - `song` Song to compute score from
     */
    // TODO do we really want the mean there?
    pub fn get_value(&mut self) -> f32 {
        mean(&self.values)
    }
}

struct SpectralRollOffDesc {
    aubio_obj: SpecDesc,
    phase_vocoder: PVoc,
    // Values before being summarized through f.ex. a mean
    values: Vec<f32>,
    sample_rate: u32,
}

struct SpectralCentroidDesc {
    aubio_obj: SpecDesc,
    phase_vocoder: PVoc,
    // Values before being summarized through f.ex. a mean
    values: Vec<f32>,
    sample_rate: u32,
}

impl SpectralRollOffDesc {
    const WINDOW_SIZE: usize = 512;
    const HOP_SIZE: usize = SpectralRollOffDesc::WINDOW_SIZE / 4;

    pub fn new(sample_rate: u32) -> Self {
        SpectralRollOffDesc {
            aubio_obj: SpecDesc::new(SpecShape::Rolloff, SpectralRollOffDesc::WINDOW_SIZE)
                .unwrap(),
            phase_vocoder: PVoc::new(
                SpectralRollOffDesc::WINDOW_SIZE,
                SpectralRollOffDesc::HOP_SIZE,
            )
            .unwrap(),
            values: Vec::new(),
            sample_rate,
        }
    }

    pub fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; SpectralRollOffDesc::WINDOW_SIZE + 2];

        self.phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let bin = self.aubio_obj.do_result(fftgrain.as_slice()).unwrap();
        let freq = bin_to_freq(
            bin,
            self.sample_rate as f32,
            SpectralRollOffDesc::WINDOW_SIZE as f32,
        );
        self.values.push(freq);
    }

    /**
     * Compute score related to spectral centroid.
     * Returns the mean of computed spectral centroids over the song.
     *
     * - `song` Song to compute score from
     */
    pub fn get_value(&mut self) -> f32 {
        mean(&self.values)
    }
}

impl SpectralCentroidDesc {
    const WINDOW_SIZE: usize = 512;
    const HOP_SIZE: usize = SpectralCentroidDesc::WINDOW_SIZE / 4;

    pub fn new(sample_rate: u32) -> Self {
        SpectralCentroidDesc {
            aubio_obj: SpecDesc::new(SpecShape::Centroid, SpectralCentroidDesc::WINDOW_SIZE)
                .unwrap(),
            phase_vocoder: PVoc::new(
                SpectralCentroidDesc::WINDOW_SIZE,
                SpectralCentroidDesc::HOP_SIZE,
            )
            .unwrap(),
            // TODO vec with capacity?
            values: Vec::new(),
            sample_rate,
        }
    }

    pub fn do_(&mut self, chunk: &[f32]) {
        let mut fftgrain: Vec<f32> = vec![0.0; SpectralCentroidDesc::WINDOW_SIZE + 2];
        // If silence, centroid will be off, so skip instead
        if silence_detection(chunk, -60.0) {
            return;
        }

        self.phase_vocoder
            .do_(chunk, fftgrain.as_mut_slice())
            .unwrap();
        let bin = self.aubio_obj.do_result(fftgrain.as_slice()).unwrap();
        let freq = bin_to_freq(
            bin,
            self.sample_rate as f32,
            SpectralCentroidDesc::WINDOW_SIZE as f32,
        );
        self.values.push(freq);
    }

    /**
     * Compute score related to spectral centroid.
     * Returns the mean of computed spectral centroids over the song.
     *
     * - `song` Song to compute score from
     */
    pub fn get_value(&mut self) -> f32 {
        mean(&self.values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn test_zcr() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut zcr_desc = ZeroCrossingRateDesc::new();
        for chunk in song.sample_array.chunks(ZeroCrossingRateDesc::HOP_SIZE) {
            zcr_desc.do_(&chunk);
        }
        assert!(0.001 > (0.075 - zcr_desc.get_value()).abs());
    }

    #[test]
    fn test_tempo() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut tempo_desc = TempoDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks(TempoDesc::HOP_SIZE) {
            tempo_desc.do_(&chunk);
        }
        assert!(0.01 > (142.38 - tempo_desc.get_value()).abs());
    }

    #[test]
    fn test_flatness() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut flatness_desc = SpectralFlatnessDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks(SpectralFlatnessDesc::HOP_SIZE) {
            flatness_desc.do_(&chunk);
        }
        assert!(0.01 > (12.74 - flatness_desc.get_value()).abs());
    }

    #[test]
    fn test_roll_off() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut roll_off_desc = SpectralRollOffDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks(SpectralRollOffDesc::HOP_SIZE) {
            roll_off_desc.do_(&chunk);
        }
        assert!(0.01 > (2026.69 - roll_off_desc.get_value()).abs());
    }

    #[test]
    fn test_centroid() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut centroid_desc = SpectralCentroidDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks(SpectralCentroidDesc::HOP_SIZE) {
            centroid_desc.do_(&chunk);
        }
        assert!(0.01 > (1236.39 - centroid_desc.get_value()).abs());
    }
}
