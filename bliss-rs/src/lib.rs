// temporarily pub
pub mod analyze;
pub mod decode;

pub const CHANNELS: u16 = 1;
pub const SAMPLE_RATE: u32 = 22050;

#[derive(Default)]
pub struct Song {
    pub sample_array: Vec<f32>,
    pub sample_rate: u32,
    pub file_path: String,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub track_number: String,
    pub genre: String,
}

#[derive(Default)]
pub struct Analysis {
    pub tempo: f32,
    pub spectral_centroid: f32,
}
