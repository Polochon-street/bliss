//! Temporal feature extraction module.
//!
//! Contains functions to extract & summarize the temporal aspects
//! of a given Song.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

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
 * Ranges from 0 (theoretically...) to 200 BPM.
 * 
 * (Also, if someone knows a way in aubio to get the correct value of 200 BPM
 * for "Through the Fire and Flames", please chip in)
 */
pub struct BPMDesc {
    aubio_obj: Tempo,
}

impl BPMDesc {
    pub const WINDOW_SIZE: usize = 512;
    pub const HOP_SIZE: usize = BPMDesc::WINDOW_SIZE / 4;

    pub fn new(sample_rate: u32) -> Self {
        BPMDesc {
            aubio_obj: Tempo::new(
                OnsetMode::WPhase,
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
    pub fn get_value(&mut self) -> f32 {
        self.aubio_obj.get_bpm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn test_tempo() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut tempo_desc = BPMDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks_exact(BPMDesc::HOP_SIZE) {
            tempo_desc.do_(&chunk);
        }
        assert!(0.01 > (141.993 - tempo_desc.get_value()).abs());
    }
}
