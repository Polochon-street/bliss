//! Temporal feature extraction module.
//!
//! Contains functions to extract & summarize the temporal aspects
//! of a given Song.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use crate::utils::Normalize;
use aubio_rs::{OnsetMode, Tempo};

/**
 * Beats per minutes ([BPM](https://en.wikipedia.org/wiki/Tempo#Measurement))
 * detection object.
 *
 * It indicates the (subjective) "speed" of a music piece. The higher the BPM,
 * the "quicker" the song will feel.
 *
 * It uses `WPhase`, a phase-deviation onset detection function to perform
 * onset detection; it proved to be the best for finding out the BPM of a panel
 * of songs I had, but it could very well be replaced by something better in the
 * future.
 *
 * Ranges from 0 (theoretically...) to 206 BPM. (Even though aubio apparently
 * has trouble to identify tempo > 190 BPM - did not investigate too much)
 *
 */
pub struct BPMDesc {
    aubio_obj: Tempo,
}

// TODO use the confidence value to discard this descriptor if confidence
// is too low.
impl BPMDesc {
    pub const WINDOW_SIZE: usize = 512;
    pub const HOP_SIZE: usize = BPMDesc::WINDOW_SIZE / 4;

    pub fn new(sample_rate: u32) -> Self {
        BPMDesc {
            aubio_obj: Tempo::new(
                OnsetMode::SpecFlux,
                BPMDesc::WINDOW_SIZE,
                BPMDesc::HOP_SIZE,
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
    // TODO analyse a whole library and check that this is not > 1.
    pub fn get_value(&mut self) -> f32 {
        self.normalize(self.aubio_obj.get_bpm())
    }
}

impl Normalize for BPMDesc {
    // See aubio/src/tempo/beattracking.c:387
    // Should really be 413, needs testing
    const MAX_VALUE: f32 = 206.;
    const MIN_VALUE: f32 = 0.;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Song;

    #[test]
    fn test_tempo_real() {
        let song = Song::decode("data/s16_mono_22_5kHz.flac").unwrap();
        let mut tempo_desc = BPMDesc::new(song.sample_rate);
        for chunk in song.sample_array.unwrap().chunks_exact(BPMDesc::HOP_SIZE) {
            tempo_desc.do_(&chunk);
        }
        assert!(0.01 > (0.378605 - tempo_desc.get_value()).abs());
    }

    #[test]
    fn test_tempo_artificial() {
        let mut tempo_desc = BPMDesc::new(22050);
        // This gives one beat every second, so 60 BPM
        let mut one_chunk = vec![0.; 22000];
        one_chunk.append(&mut vec![1.; 100]);
        let chunks = std::iter::repeat(one_chunk.iter())
            .take(100)
            .flatten()
            .cloned()
            .collect::<Vec<f32>>();
        for chunk in chunks.chunks_exact(BPMDesc::HOP_SIZE) {
            tempo_desc.do_(&chunk);
        }

        // -0.41 is 60 BPM normalized
        assert!(0.01 > (-0.416853 - tempo_desc.get_value()).abs());
    }

    #[test]
    fn test_tempo_boundaries() {
        let mut tempo_desc = BPMDesc::new(10);
        let silence_chunk = vec![0.; 1024];
        tempo_desc.do_(&silence_chunk);
        assert_eq!(-1., tempo_desc.get_value());

        let mut tempo_desc = BPMDesc::new(22050);
        // The highest value I could obtain was with these params, even though
        // apparently the higher bound is 206 BPM, but here I found ~189 BPM.
        let mut one_chunk = vec![0.; 6989];
        one_chunk.append(&mut vec![1.; 100]);
        let chunks = std::iter::repeat(one_chunk.iter())
            .take(500)
            .flatten()
            .cloned()
            .collect::<Vec<f32>>();
        for chunk in chunks.chunks_exact(BPMDesc::HOP_SIZE) {
            tempo_desc.do_(&chunk);
        }
        // 0.83 is 189 BPM normalized
        assert!(0.01 > (0.83015 - tempo_desc.get_value()).abs());
    }
}
