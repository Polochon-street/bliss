// temporarily pub
// TODO get pub stuff right
// TODO use proper logging system instead of just printlns
pub mod chroma;
pub mod misc;
pub mod song;
pub mod temporal;
pub mod timbral;
pub mod utils;
#[macro_use]
extern crate lazy_static;
extern crate crossbeam;
extern crate num_cpus;
use ndarray::{arr1, arr2, Array1, Array2};

pub const CHANNELS: u16 = 1;
pub const SAMPLE_RATE: u32 = 22050;

lazy_static! {
    /// Covariance matrix for Mahalanobis distance.
    /// See https://lelele.io/thesis.pdf, section 4.
    static ref M: Array2<f32> = arr2(&[
        [
            0.0252749,
            -0.01687417,
            -0.0127546,
            -0.00482922,
            -0.02494876,
            -0.00214683,
            0.06241617,
            -0.03128464,
            -0.0058537
        ],
        [
            -0.01687417,
            0.05556368,
            0.08265444,
            -0.0265552,
            -0.09629444,
            0.00853458,
            -0.03379969,
            0.02132666,
            0.00052228
        ],
        [
            -0.0127546,
            0.08265444,
            0.1666087,
            -0.04609117,
            -0.12301082,
            0.07466383,
            0.01525878,
            -0.0128122,
            -0.00777542
        ],
        [
            -0.00482922,
            -0.0265552,
            -0.04609117,
            0.02578383,
            0.09469197,
            0.01697223,
            -0.00968029,
            0.00318867,
            -0.00242896
        ],
        [
            -0.02494876,
            -0.09629444,
            -0.12301082,
            0.09469197,
            0.42204981,
            0.1233608,
            -0.01607664,
            -0.01726619,
            -0.00726132
        ],
        [
            -0.00214683,
            0.00853458,
            0.07466383,
            0.01697223,
            0.1233608,
            0.18282812,
            0.07852367,
            -0.05310373,
            -0.03115961
        ],
        [
            0.06241617,
            -0.03379969,
            0.01525878,
            -0.00968029,
            -0.01607664,
            0.07852367,
            0.19510575,
            -0.10635691,
            -0.02719779
        ],
        [
            -0.03128464,
            0.02132666,
            -0.0128122,
            0.00318867,
            -0.01726619,
            -0.05310373,
            -0.10635691,
            0.06299881,
            0.01300533
        ],
        [
            -0.0058537,
            0.00052228,
            -0.00777542,
            -0.00242896,
            -0.00726132,
            -0.03115961,
            -0.02719779,
            0.01300533,
            0.00895838
        ]
    ]);
}

#[derive(Default, Debug, PartialEq)]
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
    pub is_major: f32,
    pub fifth: (f32, f32),
}

pub fn bulk_analyse(paths: Vec<String>) -> Vec<Result<Song, String>> {
    let mut songs = Vec::with_capacity(paths.len());
    let num_cpus = num_cpus::get();

    crossbeam::scope(|s| {
        let mut handles = Vec::with_capacity(paths.len() / num_cpus);
        for chunk in paths.chunks(paths.len() / num_cpus) {
            handles.push(s.spawn(move |_| {
                let mut result = Vec::with_capacity(chunk.len());
                for path in chunk {
                    println!("Analyzing path {}", path);
                    let song = Song::new(&path);
                    result.push(song);
                }
                result
            }));
        }

        for handle in handles {
            songs.extend(handle.join().unwrap());
        }
    })
    .unwrap();

    songs
}

impl Analysis {
    #[allow(dead_code)]
    fn approx_eq(&self, other: &Self) -> bool {
        0.01 > (self.tempo - other.tempo).abs()
            && 0.01 > (self.spectral_centroid - other.spectral_centroid).abs()
            && 0.01 > (self.zero_crossing_rate - other.zero_crossing_rate).abs()
            && 0.01 > (self.spectral_rolloff - other.spectral_rolloff).abs()
            && 0.01 > (self.spectral_flatness - other.spectral_flatness).abs()
            && 0.01 > (self.loudness - other.loudness).abs()
            && 0.01 > (self.fifth.0 - other.fifth.0).abs()
            && 0.01 > (self.fifth.1 - other.fifth.1).abs()
            && self.fifth == other.fifth
    }

    pub fn to_vec(&self) -> Vec<f32> {
        vec![
            self.tempo,
            self.spectral_centroid,
            self.zero_crossing_rate,
            self.spectral_rolloff,
            self.spectral_flatness,
            self.loudness,
            self.is_major,
            self.fifth.0,
            self.fifth.1,
        ]
    }

    #[allow(dead_code)]
    pub fn distance(&self, other: &Self) -> f32 {
        let a1: &Array1<f32> = &arr1(&self.to_vec());
        let a2: &Array1<f32> = &arr1(&other.to_vec());

        M.dot(&(a1 - a2)).dot(&(a1 - a2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_to_vec() {
        let analysis = Analysis {
            tempo: 0.37860596,
            spectral_centroid: -0.75483,
            zero_crossing_rate: -0.85036564,
            spectral_rolloff: -0.6326486,
            spectral_flatness: -0.77610075,
            loudness: 0.27126348,
            is_major: -1.,
            fifth: (0., 1.),
        };

        assert_eq!(
            vec![
                0.37860596,
                -0.75483,
                -0.85036564,
                -0.6326486,
                -0.77610075,
                0.27126348,
                -1.,
                0.,
                1.
            ],
            analysis.to_vec(),
        );
    }

    #[test]
    fn test_analysis_distance() {
        let a = Analysis {
            tempo: 0.37860596,
            spectral_centroid: -0.75483,
            zero_crossing_rate: -0.85036564,
            spectral_rolloff: -0.6326486,
            spectral_flatness: -0.77610075,
            loudness: 0.27126348,
            is_major: -1.,
            fifth: (0., 1.),
        };

        let b = Analysis {
            tempo: 0.31255,
            spectral_centroid: 0.15483,
            zero_crossing_rate: -0.15036564,
            spectral_rolloff: -0.0326486,
            spectral_flatness: -0.87610075,
            loudness: -0.27126348,
            is_major: 1.,
            fifth: (0., 1.),
        };
        assert_eq!(a.distance(&b), 0.69275045,)
    }

    #[test]
    fn test_analysis_distance_indiscernible() {
        let a = Analysis {
            tempo: 0.37860596,
            spectral_centroid: -0.75483,
            zero_crossing_rate: -0.85036564,
            spectral_rolloff: -0.6326486,
            spectral_flatness: -0.77610075,
            loudness: 0.27126348,
            is_major: -1.,
            fifth: (0., 1.),
        };

        assert_eq!(a.distance(&a), 0.)
    }

    #[test]
    fn test_bulk_analyse() {
        let results = bulk_analyse(vec![
            String::from("data/s16_mono_22_5kHz.flac"),
            String::from("data/s16_mono_22_5kHz.flac"),
            String::from("nonexistent"),
            String::from("data/s16_stereo_22_5kHz.flac"),
            String::from("nonexistent"),
            String::from("nonexistent"),
            String::from("nonexistent"),
            String::from("nonexistent"),
            String::from("nonexistent"),
            String::from("nonexistent"),
            String::from("nonexistent"),
        ]);
        let mut errored_songs: Vec<String> = results
            .iter()
            .filter_map(|x| x.as_ref().err().cloned())
            .collect();
        errored_songs.sort_by(|a, b| a.cmp(b));

        let mut analysed_songs: Vec<String> = results
            .iter()
            .filter_map(|x| x.as_ref().ok().map(|x| x.path.to_string()))
            .collect();
        analysed_songs.sort_by(|a, b| a.cmp(b));

        assert_eq!(
            vec![
                String::from(
                    "FFmpeg error while opening format: ffmpeg::Error(2: No such file or directory)."
                );
                8
            ],
            errored_songs
        );
        assert_eq!(
            vec![
                String::from("data/s16_mono_22_5kHz.flac"),
                String::from("data/s16_mono_22_5kHz.flac"),
                String::from("data/s16_stereo_22_5kHz.flac"),
            ],
            analysed_songs,
        );
    }
}
