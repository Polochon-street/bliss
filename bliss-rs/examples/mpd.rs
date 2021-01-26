//! Example of how a plugin for an audio player could look like.
//!
//! The handles the analysis of an [MPD](https://www.musicpd.org/) song
//! library, storing songs in an SQLite local database file in
//! ~/.local/share/bliss-rs/songs.db
//!
//! Playlists can then subsequently be made from the current song using
//! --playlist.
use bliss_rs::library::Library;
use bliss_rs::Song;
use dirs::data_local_dir;
use mpd::search::{Query, Term};
#[cfg(not(test))]
use mpd::Client;
use rusqlite::{params, Connection, Error, Row};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct MPDLibrary {
    pub mpd_base_path: String,
    pub sqlite_conn: Arc<Mutex<Connection>>,
}

impl MPDLibrary {
    fn row_closure(row: &Row) -> Result<(String, f32), Error> {
        let path = row.get(0)?;
        let feature = row.get(1)?;
        Ok((path, feature))
    }

    #[cfg(not(test))]
    fn get_mpd_conn() -> Client {
        // TODO add support for MPD_HOST or sth
        Client::connect("127.0.0.1:6600").unwrap()
    }

    fn get_database_folder() -> PathBuf {
        match env::var("XDG_DATA_HOME") {
            // TODO check that path is valid
            Ok(path) => Path::new(&path).join("bliss-rs"),
            Err(_) => data_local_dir().unwrap().join("bliss-rs"),
        }
    }

    fn current_song(&self) -> Song {
        let mut mpd_conn = Self::get_mpd_conn();
        let song = mpd_conn.currentsong().unwrap().unwrap();
        let sql_conn = self.sqlite_conn.lock().unwrap();

        let path = String::from(
            Path::new(&self.mpd_base_path)
                .join(Path::new(&song.file))
                .to_str()
                .unwrap(),
        );
        // TODO handle case where the song is not in the library
        let mut stmt = sql_conn
            .prepare(
                "
                  select
                      song.path, feature from feature
                      inner join song on song.id = feature.song_id
                      where song.path = ? and analyzed = true
                      order by song.path, feature.feature_index
                ",
            )
            .unwrap();
        let results = stmt
            .query_map(params![&path], MPDLibrary::row_closure)
            .unwrap();

        let mut current_song = Song::default();
        current_song.path = path.to_string();
        let mut analysis = vec![];
        for result in results {
            analysis.push(result.unwrap().1);
        }
        current_song.analysis = analysis;
        current_song
    }

    //TODO add Result
    fn new(mpd_base_path: String) -> Self {
        let db_folder = Self::get_database_folder();
        create_dir_all(&db_folder).unwrap();
        let db_path = db_folder.join(Path::new("songs.db"));
        let db_path = db_path.to_str().unwrap();
        let sqlite_conn = Connection::open(db_path).unwrap();
        sqlite_conn
            .execute(
                "
            create table if not exists song (
                id integer primary key,
                path text not null unique,
                artist text,
                title text,
                album text,
                track_number text,
                genre text,
                stamp timestamp default current_timestamp,
                analyzed boolean default false
            );
            ",
                [],
            )
            .unwrap();
        sqlite_conn
            .execute("pragma foreign_keys = on;", [])
            .unwrap();
        sqlite_conn
            .execute(
                "
            create table if not exists feature (
                id integer primary key,
                song_id integer not null,
                feature real not null,
                feature_index integer not null,
                unique(id, feature_index),
                foreign key(song_id) references song(id)
            )
            ",
                [],
            )
            .unwrap();
        MPDLibrary {
            mpd_base_path,
            sqlite_conn: Arc::new(Mutex::new(sqlite_conn)),
        }
    }

    // TODO print stuff, like how much stuff there's to update, etc
    fn update(&mut self) {
        let stored_songs = self
            .get_stored_songs()
            .iter()
            .map(|x| x.path.to_owned())
            .collect::<HashSet<String>>();
        let mpd_songs = self
            .get_songs_paths()
            .into_iter()
            .collect::<HashSet<String>>();
        let to_analyze = mpd_songs
            .difference(&stored_songs)
            .cloned()
            .collect::<Vec<String>>();
        self.analyze_paths(to_analyze).unwrap();
    }

    fn full_rescan(&mut self) -> Result<(), String> {
        let sqlite_conn = self.sqlite_conn.lock().unwrap();
        sqlite_conn.execute("delete from feature", []).unwrap();
        sqlite_conn.execute("delete from song", []).unwrap();
        drop(sqlite_conn);
        self.analyze_library()
    }
}

impl Library for MPDLibrary {
    fn get_stored_songs(&self) -> Vec<Song> {
        let sqlite_conn = self.sqlite_conn.lock().unwrap();
        let mut stmt = sqlite_conn
            .prepare(
                "
                select
                    song.path, feature from feature
                    inner join song on song.id = feature.song_id
                    where song.analyzed = true order by path;
                ",
            )
            .unwrap();
        let results = stmt.query_map([], MPDLibrary::row_closure).unwrap();

        let mut songs_hashmap = HashMap::new();
        for result in results {
            let result = result.unwrap();
            let song_entry = songs_hashmap.entry(result.0).or_insert(vec![]);
            song_entry.push(result.1);
        }
        let songs: Vec<Song> = songs_hashmap
            .into_iter()
            .map(|(path, analysis)| Song {
                analysis,
                path,
                ..Default::default()
            })
            .collect();
        songs
    }

    fn get_songs_paths(&self) -> Vec<String> {
        let mut mpd_conn = Self::get_mpd_conn();
        mpd_conn
            .list(&Term::File, &Query::default())
            .unwrap()
            .iter()
            .map(|x| {
                String::from(
                    Path::new(&self.mpd_base_path)
                        .join(Path::new(x))
                        .to_str()
                        .unwrap(),
                )
            })
            .collect::<Vec<String>>()
    }

    fn store_song(&mut self, song: Song) {
        let sqlite_conn = self.sqlite_conn.lock().unwrap();
        sqlite_conn
            .execute(
                "
            insert into song (
                path, artist, title, album,
                track_number, genre, analyzed
            )
            values (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7
            )
            ",
                params![
                    song.path,
                    song.artist,
                    song.title,
                    song.album,
                    song.track_number,
                    song.genre,
                    true,
                ],
            )
            .unwrap();
        let last_song_id = sqlite_conn.last_insert_rowid();
        for (index, feature) in song.analysis.iter().enumerate() {
            sqlite_conn
                .execute(
                    "
                insert into feature (song_id, feature, feature_index)
                values (?1, ?2, ?3)
                ",
                    params![last_song_id, feature, index],
                )
                .unwrap();
        }
    }

    fn store_error_song(&mut self, song_path: String, _: String) {
        self.sqlite_conn
            .lock()
            .unwrap()
            .execute(
                "
            insert into song(path) values (?1)
            ",
                [song_path],
            )
            .unwrap();
    }
}

// TODO maybe store the MPD base path persistently somewhere
fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let (command, base_path) = match args.len() {
        3 => (&args[1], &args[2]),
        _ => {
            println!("Usage: ./{} <command> <MPD base path>", args[0]);
            return Err(String::from(""));
        }
    };

    let mut library = MPDLibrary::new(base_path.to_string());
    if command == "rescan" {
        library.full_rescan().unwrap();
    } else if command == "playlist" {
        let playlist = library.playlist_from_song(library.current_song(), 20);
        println!(
            "{:?}",
            playlist
                .iter()
                .map(|x| x.path.to_string())
                .collect::<Vec<String>>()
        );
    } else if command == "update" {
        library.update();
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use mpd::error::Result;
    use mpd::song::Song as MPDSong;
    use std::env;
    use tempdir::TempDir;

    // TODO smuggle results to list / currentsong here
    pub struct MockMPDClient {}

    impl MockMPDClient {
        pub fn connect(address: &str) -> Result<Self> {
            assert_eq!(address, "127.0.0.1:6600");
            Ok(Self {})
        }

        pub fn currentsong(&mut self) -> Result<Option<MPDSong>> {
            let song = MPDSong {
                file: String::from("path/first_song.flac"),
                name: Some(String::from("Coucou")),
                ..Default::default()
            };
            Ok(Some(song))
        }

        pub fn list(&mut self, term: &Term, _: &Query) -> Result<Vec<String>> {
            assert!(matches!(term, Term::File));
            Ok(vec![
                String::from("./data/s16_mono_22_5kHz.flac"),
                String::from("./data/s16_stereo_22_5kHz.flac"),
                String::from("foo"),
            ])
        }
    }

    impl MPDLibrary {
        pub fn get_mpd_conn() -> MockMPDClient {
            MockMPDClient::connect("127.0.0.1:6600").unwrap()
        }
    }

    #[test]
    fn test_full_rescan() {
        let temp_directory = TempDir::new("test-analyze").unwrap();
        env::set_var("XDG_DATA_HOME", temp_directory.path().to_str().unwrap());

        let mut library = MPDLibrary::new(String::from(""));
        let sqlite_conn = library.sqlite_conn.lock().unwrap();
        sqlite_conn
            .execute(
                "
            insert into song (id, path, analyzed) values
                (1,'./data/s16_mono_22_5kHz.flac', true)
            ",
                [],
            )
            .unwrap();

        sqlite_conn
            .execute(
                "
            insert into feature (song_id, feature, feature_index) values
                (1, 0., 1),
                (1, 0., 2),
                (1, 0., 3)
            ",
                [],
            )
            .unwrap();
        drop(sqlite_conn);

        library.full_rescan().unwrap();

        let sqlite_conn = library.sqlite_conn.lock().unwrap();
        let mut stmt = sqlite_conn
            .prepare("select path, analyzed from song order by path")
            .unwrap();
        let expected_songs = stmt
            .query_map([], |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())))
            .unwrap()
            .map(|x| {
                let x = x.unwrap();
                (x.0, x.1)
            })
            .collect::<Vec<(String, bool)>>();

        assert_eq!(
            expected_songs,
            vec![
                (String::from("./data/s16_mono_22_5kHz.flac"), true),
                (String::from("./data/s16_stereo_22_5kHz.flac"), true),
                (String::from("foo"), false),
            ],
        );

        let mut stmt = sqlite_conn
            .prepare("select count(*) from feature group by song_id")
            .unwrap();
        let expected_feature_count = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<u32>>();
        for feature_count in expected_feature_count {
            assert!(feature_count > 1);
        }
    }

    #[test]
    fn test_playlist() {
        let temp_directory = TempDir::new("test-playlist").unwrap();
        env::set_var("XDG_DATA_HOME", temp_directory.path().to_str().unwrap());

        let library = MPDLibrary::new(String::from(""));

        let sqlite_conn = library.sqlite_conn.lock().unwrap();
        sqlite_conn
            .execute(
                "
            insert into song (id, path, analyzed) values
                (1,'path/first_song.flac', true),
                (2,'path/second_song.flac', true),
                (3,'path/last_song.flac', true),
                (4,'path/unanalyzed.flac', false)
            ",
                [],
            )
            .unwrap();

        sqlite_conn
            .execute(
                "
            insert into feature (song_id, feature, feature_index) values
                (1, 0., 1),
                (1, 0., 2),
                (1, 0., 3),
                (2, 0.1, 1),
                (2, 0.1, 2),
                (2, 0.1, 3),
                (3, 10., 1),
                (3, 10., 2),
                (3, 10., 3)
            ",
                [],
            )
            .unwrap();
        drop(sqlite_conn);
        let playlist = library
            .playlist_from_song(library.current_song(), 20)
            .iter()
            .map(|x| x.path.to_owned())
            .collect::<Vec<String>>();

        assert_eq!(
            playlist,
            vec![
                String::from("path/first_song.flac"),
                String::from("path/second_song.flac"),
                String::from("path/last_song.flac"),
            ],
        );
    }

    #[test]
    fn test_update() {
        let temp_directory = TempDir::new("test-playlist").unwrap();
        env::set_var("XDG_DATA_HOME", temp_directory.path().to_str().unwrap());

        let mut library = MPDLibrary::new(String::from(""));

        let sqlite_conn = library.sqlite_conn.lock().unwrap();
        sqlite_conn
            .execute(
                "
            insert into song (id, path, analyzed) values
                (1, './data/s16_mono_22_5kHz.flac', true)
            ",
                [],
            )
            .unwrap();

        sqlite_conn
            .execute(
                "
            insert into feature (song_id, feature, feature_index) values
                (1, 0., 1),
                (1, 0., 2),
                (1, 0., 3)
            ",
                [],
            )
            .unwrap();
        drop(sqlite_conn);

        library.update();

        let sqlite_conn = library.sqlite_conn.lock().unwrap();
        let mut stmt = sqlite_conn
            .prepare("select path, analyzed from song order by path")
            .unwrap();
        let expected_songs = stmt
            .query_map([], |row| Ok((row.get(0).unwrap(), row.get(1).unwrap())))
            .unwrap()
            .map(|x| {
                let x = x.unwrap();
                (x.0, x.1)
            })
            .collect::<Vec<(String, bool)>>();

        assert_eq!(
            expected_songs,
            vec![
                (String::from("./data/s16_mono_22_5kHz.flac"), true),
                (String::from("./data/s16_stereo_22_5kHz.flac"), true),
                (String::from("foo"), false),
            ],
        );

        let mut stmt = sqlite_conn
            .prepare("select count(*) from feature group by song_id")
            .unwrap();
        let expected_feature_count = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<u32>>();
        for feature_count in expected_feature_count {
            assert!(feature_count > 1);
        }
    }
}
