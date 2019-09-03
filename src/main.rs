use plist::Value;
use rspotify::spotify::client::Spotify;
use rspotify::spotify::util::get_token;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::senum::Country;
use clap::{Arg, App};
use std::env;

#[derive(Debug, Clone)]
struct Song {
    artist: String,
    name: String,
    album: String,
    uri: String
}

impl Song {
    fn new(artist: String, name: String, album: String) -> Self {
        Song {
            artist, name, album, uri: Default::default()
        }
    }

    fn as_search_term(&self) -> String {
        format!("{} {} {}", self.name, self.artist, self.album)
    }

//    fn add_uri(&mut self, uri: String) {
//        self.uri = uri;
//    }
}


// Create a .env file in this dir with Spotify Client ID, Client Secret, and "http://localhost:8888/callback" as REDIRECT_URI
fn main() {
    let pkg = env::var("CARGO_PKG_VERSION").unwrap();
    let matches = App::new("Apple Music to Spotify Migrator")
        .version(pkg.as_ref())
        .author("Dustin Knopoff <dustinknopoff@gmail.com>")
        .about("Converts an exported iTunes/Apple Music Playlist xml file to an existing spotify playlist")
        .arg(Arg::with_name("file")
            .short("f")
            .long("file")
            .takes_value(true)
            .help("Itunes/Apple Music Playlist xml file"))
        .arg(Arg::with_name("playlist_id")
            .short("i")
            .long("id")
            .takes_value(true)
            .help("Existing playlist id in spotify"))
        .arg(Arg::with_name("username")
            .short("u")
            .long("user")
            .takes_value(true)
            .help("Your spotify username"))
        .get_matches();

    let myfile = matches.value_of("file").expect("A file is required.");
    let pid = matches.value_of("playlist_id").expect("A playlist id is required.");
    let user = matches.value_of("username").expect("A spotify username is required.");
    let book = Value::from_file(myfile)
        .expect("failed to read book.plist");

    let first = book
        .as_dictionary()
        .and_then(|dict| dict.get("Tracks"))
        .and_then(|tracks| tracks.as_dictionary());

    let mut songs: Vec<Song> = Vec::new();
    if let Some(tracks) = first {
        let kys = tracks.keys();
        for key in kys {
            let track_info = tracks.get(key).and_then(|track| track.as_dictionary());
            if let Some(track) = track_info {
                let artist = track["Artist"].clone().into_string().unwrap();
                let name = track["Name"].clone().into_string().unwrap();
                let album = track["Album"].clone().into_string().unwrap();
                songs.push(Song::new(artist, name, album));
            }
        }
    }
    let mut oauth = SpotifyOAuth::default().scope("playlist-modify-private playlist-modify-public playlist-read-private").build();
    match get_token(&mut oauth) {
        None => println!("auth failed"),
        Some(token_info) => {
            let client_credential = SpotifyClientCredentials::default()
                .token_info(token_info)
                .build();
            let spotify = Spotify::default()
                .client_credentials_manager(client_credential)
                .build();
            let mut track_ids: Vec<String> = Vec::new();
            songs.iter().for_each(|ref mut song| {
                let query = &song.as_search_term();
                let result = spotify.search_track(query, 1, 0, Some(Country::UnitedStates));
                if let Ok(searched) = result {
                    let res = searched.tracks.items.first();
                    if let Some(fulltrack) = res {
                        track_ids.push(fulltrack.uri.clone());
                    }
                }
            });
            for (index, _) in track_ids.iter().enumerate().step_by(75) {
                let max_val = if index + 75 > track_ids.len() { track_ids.len() } else { index + 75 };
                let sliced_ids = &track_ids[index..max_val];
                let playlist_id = pid;
                let user_id = user;
                let result = spotify.user_playlist_add_tracks(user_id, playlist_id, &sliced_ids, None);
                println!("{:?}", result);
            }
        }
    };
}