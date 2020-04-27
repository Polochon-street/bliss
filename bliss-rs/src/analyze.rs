//! Song analysis module.
//!
//! Use features-extraction functions from other modules
//! e.g. tempo features, spectral features, etc to build an
//! Analysis of a Song
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use crate::timbral::{
    SpectralDesc,
    ZeroCrossingRateDesc,
};
use crate::decode::decode_song;
use crate::misc::LoudnessDesc;
use crate::temporal::BPMDesc;
use crate::{Analysis, Song};

pub fn decode_and_analyze(path: &str) -> Result<Song, String> {
    // TODO error handling here
    let mut song = decode_song(&path)?;

    song.analysis = analyze(&song);
    Ok(song)
}

pub fn analyze(song: &Song) -> Analysis {
    let mut spectral_desc = SpectralDesc::new(song.sample_rate);
    let mut zcr_desc = ZeroCrossingRateDesc::default();
    let mut tempo_desc = BPMDesc::new(song.sample_rate);
    let mut loudness_desc = LoudnessDesc::default();

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

    Analysis {
        tempo: tempo_desc.get_value(),
        spectral_centroid: spectral_desc.get_centroid(),
        zero_crossing_rate: zcr_desc.get_value(),
        spectral_rolloff: spectral_desc.get_rolloff(),
        spectral_flatness: spectral_desc.get_flatness(),
        loudness: loudness_desc.get_value(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_song;

    #[test]
    fn test_analyze() {
        let song = decode_song("data/s16_mono_22_5kHz.flac").unwrap();
        let expected_analysis = Analysis {
            tempo: 141.99,
            spectral_centroid: 1354.22,
            zero_crossing_rate: 0.075,
            spectral_rolloff: 2026.76,
            spectral_flatness: 0.11,
            loudness: -32.79,
        };
        assert!(expected_analysis.approx_eq(&analyze(&song)));
    }
}
