//! Song analysis module.
//!
//! Use features-extraction functions from other modules
//! e.g. tempo features, spectral features, etc to build an
//! Analysis of a Song
#[cfg(feature = "aubio-lib")]
extern crate aubio_lib;

use crate::spectral::{
    SpectralCentroidDesc, SpectralDesc, SpectralDescriptor, SpectralFlatnessDesc,
    SpectralRollOffDesc, ZeroCrossingRateDesc,
};
use crate::tempo::TempoDesc;
use crate::{Analysis, Song};

pub fn analyze(song: &Song) -> Analysis {
    let mut centroid_desc = SpectralCentroidDesc::new(song.sample_rate);
    let mut rolloff_desc = SpectralRollOffDesc::new(song.sample_rate);
    let mut flatness_desc = SpectralFlatnessDesc::new(song.sample_rate);
    let mut zcr_desc = ZeroCrossingRateDesc::default();
    let mut tempo_desc = TempoDesc::new(song.sample_rate);

    for i in 1..song.sample_array.len() {
        if (i % SpectralDesc::HOP_SIZE) == 0 {
            let beginning = (i / SpectralDesc::HOP_SIZE - 1) * SpectralDesc::HOP_SIZE;
            let end = i;
            centroid_desc.do_(&song.sample_array[beginning..end]);
            flatness_desc.do_(&song.sample_array[beginning..end]);
            rolloff_desc.do_(&song.sample_array[beginning..end]);
        }

        if (i % ZeroCrossingRateDesc::HOP_SIZE) == 0 {
            let beginning = (i / ZeroCrossingRateDesc::HOP_SIZE - 1) * ZeroCrossingRateDesc::HOP_SIZE;
            let end = i;
            zcr_desc.do_(&song.sample_array[beginning..end]);
        }

        if (i % TempoDesc::HOP_SIZE) == 0 {
            let beginning = (i / TempoDesc::HOP_SIZE - 1) * TempoDesc::HOP_SIZE;
            let end = i;
            tempo_desc.do_(&song.sample_array[beginning..end]);
        }
    }

    Analysis {
        tempo: tempo_desc.get_value(),
        spectral_centroid: centroid_desc.get_value(),
        zero_crossing_rate: zcr_desc.get_value(),
        spectral_rolloff: rolloff_desc.get_value(),
        spectral_flatness: flatness_desc.get_value(),
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
            tempo: 142.38,
            spectral_centroid: 1236.37,
            zero_crossing_rate: 0.075,
            spectral_rolloff: 2026.76,
            spectral_flatness: 12.74,
        };
        assert!(expected_analysis.approx_eq(&analyze(&song)));
    }
}
