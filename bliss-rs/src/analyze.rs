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
use crate::tempo::TempoDesc;
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
    let mut tempo_desc = TempoDesc::new(song.sample_rate);

    for i in 1..song.sample_array.len() {
        if (i % SpectralDesc::HOP_SIZE) == 0 {
            let beginning = (i / SpectralDesc::HOP_SIZE - 1) * SpectralDesc::HOP_SIZE;
            let end = i;
            spectral_desc.do_(&song.sample_array[beginning..end]);
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
        spectral_centroid: spectral_desc.get_centroid(),
        zero_crossing_rate: zcr_desc.get_value(),
        spectral_rolloff: spectral_desc.get_rolloff(),
        spectral_flatness: spectral_desc.get_flatness(),
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
            spectral_centroid: 1354.22,
            zero_crossing_rate: 0.075,
            spectral_rolloff: 2026.76,
            spectral_flatness: 0.11,
        };
        assert!(expected_analysis.approx_eq(&analyze(&song)));
    }
}
