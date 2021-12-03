#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tokio;
mod download;
// mod transcode;
mod ffmpeg;
mod ffmpeg_sys;
mod serialization;
mod spotify;
mod transcode2;
mod utils;

use anyhow::Result;
use download::Downloader;
use std::path::PathBuf;
use std::thread;
use tempdir::TempDir;
use warp::Filter;
// use transcode::Transcoder;

struct DjTool {
    downloader: Downloader,
    // transcoder: Transcoder,
}

impl DjTool {
    pub fn new() -> Result<Self> {
        let downloader = Downloader::new()?;
        Ok(Self {
            downloader,
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
            // spin up a webserver
            tokio::spawn(async move {
                let examples = warp::path("static").and(warp::fs::dir("./examples/"));
                // let routes = readme.or(examples);
                warp::serve(examples).run(([127, 0, 0, 1], 21011)).await;
            });

            let tool = DjTool::new().unwrap();
            tool.download_youtube("_Q8ELKOLudE".to_string())
                .await
                .unwrap();
        });
    });

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    // });

    // let creds = spotify::Credentials::new_pkce("todo");
    // let oauth = spotify::OAuth {
    //     redirect_uri: "http://localhost:8888/callback".to_string(),
    //     scopes: scopes!("playlist-read-private"),
    //     ..Default::default()
    // };
    // let mut spotify = spotify::AuthCodePkceSpotify::new(creds.clone(), oauth.clone());
    // let url = spotify.get_authorize_url(None).unwrap();
    // println!("auth url: {}", url);
    // spotify.load_token(&url).await.unwrap();

    // let history = spotify.current_playback(None, None::<Vec<_>>).await;
    // println!("Response: {:?}", history);

    // ui.join().unwrap();
    println!("exiting");
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
