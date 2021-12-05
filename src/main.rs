#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tokio;
mod backend;
mod config;
#[cfg(feature = "debug")]
mod debug;
mod download;
mod ffmpeg;
mod ffmpeg_sys;
mod spotify;
mod transcode2;
mod utils;
mod youtube;
mod id3;

use anyhow::Result;
use config::Persist;
use dirs;
use futures_util::pin_mut;
use futures_util::{StreamExt, TryStreamExt};
use reqwest;
use rspotify_model::{Id, PlaylistId, UserId};
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tauri::Event;
use tempdir::TempDir;
use tokio::sync::{Mutex, RwLock};
use warp::{Filter, Rejection, Reply};
use youtube::Youtube;

const DEFAULT_PORT: u16 = 21011;

type SpotifyClient = Arc<spotify::Spotify>;
type YoutubeClient = Arc<youtube::Youtube>;

struct DjTool {
    backends: Youtube,
    config: Arc<RwLock<config::Config>>,
    spotify: SpotifyClient,
}

fn request_user_login(auth_url: reqwest::Url) {
    webbrowser::open(auth_url.as_str());
}

impl DjTool {
    pub async fn new(config_dir: &PathBuf) -> Result<Self> {
        let config = config::Config::open(&config_dir).await?;
        let backends = Youtube::new()?; // config.debug_dir())?;

        // todo: store the credentials in the spotify config
        let creds = spotify::auth::Credentials::pkce("893474f878934ae89fff417e4722e147");
        let oauth = spotify::auth::OAuth {
            redirect_uri: format!("http://localhost:{}/spotify/pkce/callback", DEFAULT_PORT),
            scopes: scopes!("playlist-read-private"),
            ..Default::default()
        };
        let client = spotify::Spotify::pkce(&config_dir, creds.clone(), oauth.clone()).await?;
        println!("created spotify client");

        // authenticate client proactively
        match client.authenticator.reauthenticate().await {
            Err(spotify::error::Error::Auth(spotify::error::AuthError::RequireUserLogin {
                auth_url,
            })) => {
                request_user_login(auth_url);
                // match webbrowser::open(auth_url.as_str()) {
                //     Ok(_) => Ok(()),
                //     Err(err) => Err(err.into()),
                // };
            }
            Err(err) => panic!("{}", err),
            Ok(_) => {}
        };

        Ok(Self {
            backends,
            config: Arc::new(RwLock::new(config)),
            spotify: Arc::new(client),
            // transcoder,
        })
    }

    // async fn download_youtube(&self, video_id: String) -> Result<()> {
    //     let temp_dir = TempDir::new("djtool")?;
    //     // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.aiff");
    //     let downloaded_audio = temp_dir.path().join(&video_id);
    //     let audio = self
    //         .downloader
    //         // .download_audio(video_id, temp_dir.path().to_path_buf())
    //         .download_audio(
    //             video_id,
    //             // PathBuf::from("/home/roman/dev/djtool/Touchpad.webm"),
    //             &downloaded_audio,
    //             // temp_dir.join(video_id),
    //             // PathBuf::from(format!("/home/roman/dev/djtool/Touchpad.webm"),
    //         )
    //         .await?;

    //     // transcode to MP3
    //     // println!("temp dir: {}", temp_dir.path().display());
    //     // println!("audio output: {}", audio.audio_file.display());
    //     // let input_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.webm");
    //     // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.mp3");
    //     let output_file = PathBuf::from("/Users/roman/dev/djtool/Touchpad.mp3");
    //     let res = tokio::task::spawn_blocking(move || {
    //         transcode2::test(audio.audio_file, output_file);
    //         // let mut transcoder = Transcoder::new(audio.audio_file, output_file).unwrap();
    //         // transcoder.start().unwrap();
    //     })
    //     .await?;
    //     Ok(())
    // }
}

fn with_spotify(
    spotify: SpotifyClient,
) -> impl Filter<Extract = (SpotifyClient,), Error = Infallible> + Send + Clone {
    warp::any().map(move || spotify.clone())
}

fn with_youtube(
    youtube: YoutubeClient,
) -> impl Filter<Extract = (YoutubeClient,), Error = Infallible> + Send + Clone {
    warp::any().map(move || youtube.clone())
}

pub async fn spotify_pkce_callback_handler(
    query: spotify::auth::pkce::CallbackQuery,
    sp: SpotifyClient,
) -> std::result::Result<impl Reply, Infallible> {
    let redirect_url = match query.code {
        Some(code) => {
            sp.authenticator
                .handle_user_login_callback(spotify::auth::SpotifyLoginCallback::Pkce {
                    code,
                    state: query.state.unwrap_or(String::new()),
                })
                .await
                .unwrap();
            reqwest::Url::parse("https://spotify.com/").unwrap()
        }
        None => {
            let mut params: HashMap<&str, String> = HashMap::new();
            params.insert("error", query.error.unwrap_or(String::new()));
            reqwest::Url::parse_with_params("https://google.com", params).unwrap()
        }
    };
    println!("redirect to: {}", redirect_url);
    // todo: set secret of spotify client of config
    // signal the ui that we got the token
    let body = [
        r#"<html><head>"#.to_string(),
        format!(
            r#"<meta http-equiv="refresh" content="0; URL={}" />"#,
            redirect_url
        ),
        // r#"<script type="text/javascript">"#.to_string(),
        // r#"window.addEventListener("load", function(){window.close();});"#.to_string(),
        // r#"</script>"#.to_string(),
        r#"<title>djtool</title>"#.to_string(),
        r#"</head></html>"#.to_string(),
    ]
    .join("");
    Ok(warp::reply::html(body))
}

// #[tokio::main]
// async fn main_async() {
//     let debug_dir = dirs::home_dir().unwrap().join(".djtool").join("debug");
//     let mock = Youtube::new(debug_dir).unwrap();
//     let results = mock.search("Touchpad Two Shell".to_string()).await.unwrap();
//     println!("search results: {:?}", results);
// }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ffmpeg::init()?;
    let _ = thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let config_dir = dirs::home_dir().unwrap().join(".djtool");
            println!("config dir: {}", config_dir.display());

            let tool = DjTool::new(&config_dir).await.unwrap();
            let spotify_client = tool.spotify.clone();
            // let backend_client = tool.backend.clone();

            // let results = tool
            //     .backends
            //     .search("Touchpad Two Shell".to_string())
            //     .await
            //     .unwrap();
            // println!("search results: {:?}", results);

            println!("getting lock on the library path");
            let (library_dir, _) = {
                let config = tool.config.read().await;
                (
                    config.library.library_dir.to_owned(),
                    config.debug_dir.to_owned(),
                )
            };

            // spin up a webserver
            let server = tokio::spawn(async move {
                let library = warp::path("static").and(warp::fs::dir(library_dir));

                let spotify_pkce_callback = warp::get()
                    .and(warp::path!("spotify" / "pkce" / "callback"))
                    .and(warp::query::<spotify::auth::pkce::CallbackQuery>())
                    .and(with_spotify(spotify_client.clone()))
                    .and_then(spotify_pkce_callback_handler);

                #[cfg(feature = "debug")]
                let routes = {
                    let debug_spotify_playlists = warp::get()
                        .and(warp::path!("debug" / "spotify" / "playlists"))
                        .and(warp::query::<debug::DebugSpotifyPlaylistsQuery>())
                        .and(with_spotify(spotify_client.clone()))
                        .and_then(debug::debug_spotify_playlists_handler);

                    let youtube = Arc::new(Youtube::new().unwrap());
                    let debug_youtube_search = warp::get()
                        .and(warp::path!("debug" / "youtube" / "search"))
                        .and(warp::query::<debug::DebugYoutubeSearchQuery>())
                        .and(with_youtube(youtube.clone()))
                        .and_then(debug::debug_youtube_search_handler);

                    spotify_pkce_callback
                        .or(library)
                        .or(debug_youtube_search)
                        .or(debug_spotify_playlists)
                };

                #[cfg(not(feature = "debug"))]
                let routes = spotify_pkce_callback.or(library);

                println!("starting server now ...");
                warp::serve(routes)
                    // .try_bind_with_graceful_shutdown(([127, 0, 0, 1], DEFAULT_PORT), )
                    .run(([0, 0, 0, 0], DEFAULT_PORT))
                    .await;
            });

            // tool.download_youtube("_Q8ELKOLudE".to_string())
            //     .await
            //     .unwrap();

            server.await;
        });
    });

    let mut app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|handle, event| {
        match event {
            Event::ExitRequested { api, .. } => {
                println!("exiting");
                // thread::sleep(std::time::Duration::from_secs(10));
                // println!("exiting for real");
                // api.prevent_exit();
            }
            _ => {}
        }
    });

    // unreacheable
    Ok(())
}
