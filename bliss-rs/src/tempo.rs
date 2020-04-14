//! Temporal feature extraction module.
//!
//! Contains functions to extract & summarize the tempo.
//! For now, there is only a BPM-extraction function.
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use aubio_rs::{OnsetMode, Tempo};


// TODO write proper doc
pub struct TempoDesc {
    aubio_obj: Tempo,
}

impl TempoDesc {
    pub const WINDOW_SIZE: usize = 1024;
    pub const HOP_SIZE: usize = TempoDesc::WINDOW_SIZE / 4;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn test_tempo() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let mut tempo_desc = TempoDesc::new(song.sample_rate);
        for chunk in song.sample_array.chunks(TempoDesc::HOP_SIZE) {
            tempo_desc.do_(&chunk);
        }
        assert!(0.01 > (142.38 - tempo_desc.get_value()).abs());
    }
}
