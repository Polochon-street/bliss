// temporarily pub
pub mod analyze;
pub mod chroma;
pub mod decode;
pub mod misc;
pub mod temporal;
pub mod timbral;
pub mod utils;

pub const CHANNELS: u16 = 1;
pub const SAMPLE_RATE: u32 = 22050;

#[derive(Default, Debug)]
pub struct Song {
    pub sample_array: Vec<f32>,
    pub sample_rate: u32,
    pub path: String,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub track_number: String,
    pub genre: String,
    pub analysis: Analysis,
}

#[derive(Default, Debug, PartialEq)]
pub struct Analysis {
    pub tempo: f32,
    pub spectral_centroid: f32,
    pub zero_crossing_rate: f32,
    pub spectral_rolloff: f32,
    pub spectral_flatness: f32,
    pub loudness: f32,
}

impl Analysis {
    #[allow(dead_code)]
    fn approx_eq(&self, other: &Self) -> bool {
        0.01 > (self.tempo - other.tempo).abs() &&
        0.01 > (self.spectral_centroid - other.spectral_centroid).abs() &&
        0.01 > (self.zero_crossing_rate - other.zero_crossing_rate).abs() &&
        0.01 > (self.spectral_rolloff - other.spectral_rolloff).abs() &&
        0.01 > (self.spectral_flatness - other.spectral_flatness).abs()
    }
}
