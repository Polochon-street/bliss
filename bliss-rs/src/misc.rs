//! Miscellaneous feature extraction module.
//! 
//! Contains various descriptors that don't fit in one of the
//! existing categories.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use aubio_rs::level_lin;

use super::utils::mean;

/**
 * Loudness (in dB) detection object.
 *
 * It indicates how "loud" a recording of a song is. For a given audio signal,
 * this value increases if the amplitude of the signal, and nothing else, is
 * increased.
 *
 * Of course, this makes this result dependent of the recording, meaning
 * the same song would yield different loudness on different recordings. Which
 * is exactly what we want, given that this is not a music theory project, but
 * one that aims at giving the best real-life results.
 *
 * Ranges between -90 dB (~silence) and ~50 dB.
 *
 * (This is technically the sound pressure level of the track, but loudness is
 * way more visual)
 */
#[derive(Default)]
pub struct LoudnessDesc {
    pub values: Vec<f32>,
}

impl LoudnessDesc {
    pub const WINDOW_SIZE: usize = 1024;

    pub fn do_(&mut self, chunk: &[f32]) {
        let level = level_lin(chunk);
        self.values.push(level);
    }

    pub fn get_value(&mut self) -> f32 {
        10.0 * (mean(&self.values)).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn test_loudness() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut loudness_desc = LoudnessDesc::default();
        for chunk in song.sample_array.chunks_exact(LoudnessDesc::WINDOW_SIZE) {
            loudness_desc.do_(&chunk);
        }
        assert!(0.01 > (-32.7931 - loudness_desc.get_value()).abs());
    }
}
