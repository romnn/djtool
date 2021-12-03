#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tokio;
mod download;
// mod transcode;
mod config;
mod ffmpeg;
mod ffmpeg_sys;
mod serialization;
mod spotify;
mod transcode2;
mod utils;

use anyhow::Result;
use config::Persist;
use dirs;
use download::Downloader;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tauri::Event;
use tempdir::TempDir;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};
// use transcode::Transcoder;
use reqwest::Url;

const DEFAULT_PORT: u16 = 21011;

type SpotifyClient = Arc<RwLock<spotify::AuthCodePkceSpotify>>;

struct DjTool {
    downloader: Downloader,
    // transcoder: Transcoder,
    config: Arc<RwLock<config::Config>>,
    spotify: SpotifyClient,
}

impl DjTool {
    pub async fn new(config_dir: &PathBuf) -> Result<Self> {
        let downloader = Downloader::new()?;
        let config = config::Config::open(&config_dir).await?;

        let creds = spotify::Credentials::new_pkce("893474f878934ae89fff417e4722e147");
        let oauth = spotify::OAuth {
            redirect_uri: format!("http://localhost:{}/spotify/callback", DEFAULT_PORT),
            scopes: scopes!("playlist-read-private"),
            ..Default::default()
        };
        println!("creating spotify client");
        let mut client =
            spotify::AuthCodePkceSpotify::new(&config_dir, creds.clone(), oauth.clone()).await?;
        println!("checking spotify client token");
        client.check_token().await?;

        Ok(Self {
            downloader,
            config: Arc::new(RwLock::new(config)),
            spotify: Arc::new(RwLock::new(client)),
            // transcoder,
        })
    }

    async fn download_youtube(&self, video_id: String) -> Result<()> {
        let temp_dir = TempDir::new("djtool")?;
        // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.aiff");
        let downloaded_audio = temp_dir.path().join(&video_id);
        let audio = self
            .downloader
            // .download_audio(video_id, temp_dir.path().to_path_buf())
            .download_audio(
                video_id,
                // PathBuf::from("/home/roman/dev/djtool/Touchpad.webm"),
                &downloaded_audio,
                // temp_dir.join(video_id),
                // PathBuf::from(format!("/home/roman/dev/djtool/Touchpad.webm"),
            )
            .await?;

        // transcode to MP3
        // println!("temp dir: {}", temp_dir.path().display());
        // println!("audio output: {}", audio.audio_file.display());
        // let input_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.webm");
        // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.mp3");
        let output_file = PathBuf::from("/Users/roman/dev/djtool/Touchpad.mp3");
        let res = tokio::task::spawn_blocking(move || {
            transcode2::test(audio.audio_file, output_file);
            // let mut transcoder = Transcoder::new(audio.audio_file, output_file).unwrap();
            // transcoder.start().unwrap();
        })
        .await?;
        Ok(())
    }
}

fn with_spotify(
    sp: SpotifyClient,
) -> impl Filter<Extract = (SpotifyClient,), Error = Infallible> + Clone {
    warp::any().map(move || sp.clone())
}

#[derive(Deserialize)]
pub struct SpotifyCallbackQuery {
    error: Option<String>,
    state: Option<String>,
    code: Option<String>,
}

pub async fn spotify_callback_handler(
    query: SpotifyCallbackQuery,
    sp: SpotifyClient,
) -> std::result::Result<impl Reply, Infallible> {
    let redirect_url = match query.code {
        Some(code) => {
            let mut guard = sp.write().await;
            guard.handle_code_received(&code).await.unwrap();
            Url::parse("https://spotify.com/").unwrap()
        }
        None => {
            let mut params: HashMap<&str, String> = HashMap::new();
            params.insert("error", query.error.unwrap_or(String::new()));
            Url::parse_with_params("https://google.com", params).unwrap()
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
    println!("{}", body);
    Ok(warp::reply::html(body))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ffmpeg::init()?;
    // https://www.youtube.com/watch?v=KUyJFHgrrZc
    // https://www.youtube.com/watch?v=_Q8ELKOLudE
    // Hb5ZXUeGPHc

    let _ = thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let config_dir = dirs::home_dir().unwrap().join(".djtool");
            println!("config dir: {}", config_dir.display());
            let _ = tokio::fs::create_dir_all(&config_dir).await;

            let tool = DjTool::new(&config_dir).await.unwrap();
            let sp_client = tool.spotify.clone();

            // spin up a webserver
            println!("getting lock on the library path");
            let library_path = tool.config.read().await.library.library_path.to_owned();
            println!("got library path");
            let server = tokio::spawn(async move {
                // error, state
                // code, state
                let library = warp::path("static").and(warp::fs::dir(library_path));
                let spotify_callback = warp::get()
                    .and(warp::path!("spotify" / "callback"))
                    .and(warp::query::<SpotifyCallbackQuery>())
                    .and(with_spotify(sp_client))
                    .and_then(spotify_callback_handler);
                // .and_then(|map: HashMap<String, String>| -> std::result::Result<impl Reply, Rejection> async {
                // });
                let routes = spotify_callback.or(library);
                println!("starting server now ...");
                warp::serve(routes)
                    // .try_bind_with_graceful_shutdown(([127, 0, 0, 1], DEFAULT_PORT), )
                    .run(([0, 0, 0, 0], DEFAULT_PORT))
                    .await;
            });

            // tool.download_youtube("_Q8ELKOLudE".to_string())
            //     .await
            //     .unwrap();

            // let history = spotify.current_playback(None, None::<Vec<_>>).await;
            // println!("Response: {:?}", history);
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

    // ui.join().unwrap();
    // println!("exiting");
    // println!("exiting bye");
    Ok(())
}

// fn main() -> Result<(), Box<dyn std::error::Error>> {
//     ffmpeg::init()?;
//     let input_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.webm");
//     let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.aiff");
//     let mut transcoder = Transcoder::new(input_file, output_file)?;
//     transcoder.start()?;
//     Ok(())
// }
