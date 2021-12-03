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
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tauri::Event;
use tempdir::TempDir;
use tokio::sync::RwLock;
use warp::Filter;
// use transcode::Transcoder;

const DEFAULT_PORT: u16 = 21011;

struct DjTool {
    downloader: Downloader,
    // transcoder: Transcoder,
    config: Arc<RwLock<config::Config>>,
    spotify: Arc<RwLock<spotify::AuthCodePkceSpotify>>,
}

impl DjTool {
    pub async fn new(config_dir: &PathBuf) -> Result<Self> {
        let downloader = Downloader::new()?;
        let config = config::Config::load(&config_dir).await?;

        // if config
        let creds = spotify::Credentials::new_pkce("893474f878934ae89fff417e4722e147");
        let oauth = spotify::OAuth {
            redirect_uri: format!("http://localhost:{}/spotify/callback", DEFAULT_PORT),
            scopes: scopes!("playlist-read-private"),
            ..Default::default()
        };
        let client =
            spotify::AuthCodePkceSpotify::new(&config_dir, creds.clone(), oauth.clone()).await?;

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

// #[tokio::main]

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

            // spin up a webserver
            let server = tokio::spawn(async move {
                // error, state
                // code, state
                let library = warp::path("static").and(warp::fs::dir("/home/roman/dev/djtool"));
                let spotify_callback = warp::get()
                    .and(warp::path!("spotify" / "callback"))
                    .and(warp::query::<HashMap<String, String>>())
                    .map(|map: HashMap<String, String>| {
                        // let mut response: Vec<String> = Vec::new();
                        for (key, value) in map.into_iter() {
                            println!("{}={}", key, value);
                        }
                        // for (key, value) in map.into_iter() {
                        //     response.push(format!("{}={}", key, value))
                        // }
                        // todo: set secret of spotify client of config
                        // signal the ui that we got the token
                        let body = r#"
                        <html>
                            <head>
                                <title>HTML with warp!</title>
                            </head>
                            <body>
                                <h1>warp + HTML = &hearts;</h1>
                            </body>
                        </html>
                        "#;
                        warp::reply::html(body)
                    });
                let routes = spotify_callback.or(library);
                warp::serve(routes)
                    // .try_bind_with_graceful_shutdown(([127, 0, 0, 1], DEFAULT_PORT), )
                    .run(([127, 0, 0, 1], DEFAULT_PORT))
                    .await;
            });

            // tool.download_youtube("_Q8ELKOLudE".to_string())
            //     .await
            //     .unwrap();

            // let url = spotify.get_authorize_url(None).unwrap();
            // println!("auth url: {}", url);
            // spotify.load_token(&url).await.unwrap();

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
