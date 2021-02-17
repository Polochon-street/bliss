//! Module containing the Library trait, useful to get started to implement
//! a plug-in for an audio player.
use crate::Song;
use ndarray::{arr1, Array, Array1};
use ndarray_stats::QuantileExt;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

/// Library trait to make creating plug-ins for existing audio players easier.
pub trait Library {
    /// Return the absolute path of all the songs in an
    /// audio player's music library.
    fn get_songs_paths(&self) -> Vec<String>;
    /// Store an analyzed Song object in some (cold) storage, e.g.
    /// a database, a file...
    fn store_song(&mut self, song: Song);
    /// Log and / or store that an error happened while trying to decode and
    /// analyze a song.
    fn store_error_song(&mut self, song_path: String, error: String);
    /// Retrieve a list of all the stored Songs.
    ///
    /// This should work only after having run `analyze_library` at least
    /// once.
    fn get_stored_songs(&self) -> Vec<Song>;

    /// Return a list of songs that are similar to ``first_song``.
    ///
    /// # Arguments
    ///
    /// * `first_song` - The song the playlist will be built from.
    /// * `playlist_length` - The playlist length. If there are not enough
    /// songs in the library, it will be truncated to the size of the library.
    ///
    /// # Returns
    ///
    /// A vector of `playlist_length` Songs, including `first_song`, that you
    /// most likely want to plug in your audio player by using something like
    /// `ret.map(|song| song.path.to_owned()).collect::<Vec<String>>()`.
    fn playlist_from_song(&self, first_song: Song, playlist_length: usize) -> Vec<Song> {
        let analysis_current_song = arr1(&first_song.analysis.to_vec());
        let mut songs = self.get_stored_songs();
        // Get rid of the song we want to build a playlist from
        songs.retain(|x| *x != first_song);
        let mut array: Array1<f32> = Array1::zeros(songs.len());
        let m = Array::eye(first_song.analysis.len());
        // TODO make that better
        for (i, song) in songs.iter().enumerate() {
            array[[i]] = (arr1(&song.analysis) - &analysis_current_song)
                .dot(&m)
                .dot(&(arr1(&song.analysis) - &analysis_current_song));
        }

        let mut playlist: Vec<Song> = vec![first_song];
        let playlist_length = (playlist_length - 1).min(array.len());
        // TODO also make that better
        for _ in 0..playlist_length {
            let min_index = array.argmin().unwrap();
            let mut vec = array.to_vec();
            vec.remove(min_index);
            array = arr1(&vec);
            playlist.push(songs[min_index].to_owned());
            songs.remove(min_index);
        }
        playlist
    }

    /// Analyzes a song library, using `get_songs_paths`, `store_song` and
    /// `store_error_song` implementations.
    fn analyze_library(&mut self) -> Result<(), String> {
        let paths = self.get_songs_paths();
        let num_cpus = num_cpus::get();

        let (tx, rx): (
            Sender<(String, Result<Song, String>)>,
            Receiver<(String, Result<Song, String>)>,
        ) = mpsc::channel();
        let mut handles = Vec::new();
        let mut chunk_length = paths.len() / num_cpus;
        if chunk_length == 0 {
            chunk_length = paths.len();
        }

        for chunk in paths.chunks(chunk_length) {
            let tx_thread = tx.clone();
            let owned_chunk = chunk.to_owned();
            let child = thread::spawn(move || {
                for path in owned_chunk {
                    println!("Analyzing path {}", path);
                    let song = Song::new(&path);
                    tx_thread.send((path.to_string(), song)).unwrap();
                }
                drop(tx_thread);
            });
            handles.push(child);
        }
        drop(tx);

        for (path, song) in rx.iter() {
            match song {
                Ok(song) => self.store_song(song),
                Err(e) => self.store_error_song(path.to_string(), e),
            }
        }

        for child in handles {
            child.join().unwrap();
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Default)]
    struct TestLibrary {
        internal_storage: Vec<Song>,
        failed_files: Vec<(String, String)>,
    }

    impl Library for TestLibrary {
        fn get_songs_paths(&self) -> Vec<String> {
            vec![
                String::from("./data/white_noise.flac"),
                String::from("./data/s16_mono_22_5kHz.flac"),
                String::from("not-existing.foo"),
                String::from("definitely-not-existing.foo"),
            ]
        }

        fn store_song(&mut self, song: Song) {
            self.internal_storage.push(song);
        }

        fn store_error_song(&mut self, song_path: String, error: String) {
            self.failed_files.push((song_path, error));
        }

        fn get_stored_songs(&self) -> Vec<Song> {
            self.internal_storage.to_owned()
        }
    }

    #[test]
    fn test_analyze_library() {
        let mut test_library = TestLibrary {
            internal_storage: vec![],
            failed_files: vec![],
        };
        test_library.analyze_library().unwrap();

        let mut failed_files = test_library
            .failed_files
            .iter()
            .map(|x| x.0.to_owned())
            .collect::<Vec<String>>();
        failed_files.sort();

        assert_eq!(
            failed_files,
            vec![
                String::from("definitely-not-existing.foo"),
                String::from("not-existing.foo"),
            ],
        );

        let mut songs = test_library
            .internal_storage
            .iter()
            .map(|x| x.path.to_owned())
            .collect::<Vec<String>>();
        songs.sort();

        assert_eq!(
            songs,
            vec![
                String::from("./data/s16_mono_22_5kHz.flac"),
                String::from("./data/white_noise.flac"),
            ],
        );

        test_library
            .internal_storage
            .iter()
            .for_each(|x| assert!(x.analysis.len() > 0));
    }

    #[test]
    fn test_playlist_from_song() {
        let mut test_library = TestLibrary::default();
        let first_song = Song {
            path: String::from("path-to-first"),
            analysis: vec![0., 0., 0.],
            ..Default::default()
        };

        let second_song = Song {
            path: String::from("path-to-second"),
            analysis: vec![0.1, 0., 0.],
            ..Default::default()
        };

        let third_song = Song {
            path: String::from("path-to-third"),
            analysis: vec![10., 11., 10.],
            ..Default::default()
        };

        test_library.internal_storage = vec![
            first_song.to_owned(),
            second_song.to_owned(),
            third_song.to_owned(),
        ];
        assert_eq!(
            vec![first_song.to_owned(), second_song],
            test_library.playlist_from_song(first_song, 2)
        );
    }

    #[test]
    fn test_playlist_from_song_too_little_songs() {
        let mut test_library = TestLibrary::default();
        let first_song = Song {
            path: String::from("path-to-first"),
            analysis: vec![0., 0., 0.],
            ..Default::default()
        };

        let second_song = Song {
            path: String::from("path-to-second"),
            analysis: vec![0.1, 0., 0.],
            ..Default::default()
        };

        let third_song = Song {
            path: String::from("path-to-third"),
            analysis: vec![10., 11., 10.],
            ..Default::default()
        };

        test_library.internal_storage = vec![
            first_song.to_owned(),
            second_song.to_owned(),
            third_song.to_owned(),
        ];
        assert_eq!(
            vec![first_song.to_owned(), second_song, third_song],
            test_library.playlist_from_song(first_song, 200)
        );
    }
}
