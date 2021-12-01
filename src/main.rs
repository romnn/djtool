// extern crate ffmpeg_next as ffmpeg;
// #![deny(warnings)]

use tokio;
mod download;
// mod transcode;
// mod transcode2;
mod ffmpeg_sys;
mod ffmpeg;
mod utils;

use anyhow::Result;
use download::Downloader;
use std::path::PathBuf;
use tempdir::TempDir;
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
        let audio = self
            .downloader
            // .download_audio(video_id, temp_dir.path().to_path_buf())
            .download_audio(
                video_id,
                PathBuf::from("/home/roman/dev/djtool/Touchpad.webm"),
            )
            .await?;

        // transcode to MP3
        println!("temp dir: {}", temp_dir.path().display());
        println!("audio output: {}", audio.audio_file.display());
        let input_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.webm");
        let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.aiff");
        let res = tokio::task::spawn_blocking(move || {
            // transcode2::test(input_file, output_file);
            // let mut transcoder = Transcoder::new(audio.audio_file, output_file).unwrap();
            // transcoder.start().unwrap();
        })
        .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ffmpeg::init()?;
    // https://www.youtube.com/watch?v=KUyJFHgrrZc
    // https://www.youtube.com/watch?v=_Q8ELKOLudE
    // Hb5ZXUeGPHc
    let tool = DjTool::new()?;
    tool.download_youtube("_Q8ELKOLudE".to_string()).await?;
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
