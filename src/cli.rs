use clap::Parser;
use std::path::PathBuf;
// use djtool::spotify;
// use crate::spotify;
// #[cfg(feature = "spotify")]
// use crate::spotify;
// use super::spotify;
// use djtool::{DjTool, SPLASH_LOGO};
// use djtool;
// use djtool::spotify;

// #[derive(Debug, Clone)]
// pub struct Config {
// }

// djtool cli
// start: port, host, cache dir, library dir, bitrate, api, headless, spotify-auth stuff
// youtube: list, rank, info
// spotify: download, info, rank

// #[derive(Parser, Debug, Clone)]
// pub struct StartOpts {

// #[clap(short = 'f', long = "play-file")]
// pub play_file: Option<String>,
// #[clap(long = "max-sessions")]
// pub max_sessions: Option<usize>,
// #[clap(long = "max-viewers")]
// pub max_viewers: Option<usize>,
// #[clap(long = "max-controllers")]
// pub max_controllers: Option<usize>,
// #[clap(long = "keepalive-sec", default_value = "30")]
// pub max_keepalive_sec: u64,
// #[cfg(feature = "record")]
// #[clap(long = "no-sound")]
// pub no_sound: bool,
// }

#[derive(Parser, Debug, Clone)]
pub enum BackendCommand {
    // #[clap(name = "start", about = "start the server")]
    // Start(StartOpts),
    // #[clap(name = "spotify", about = "spotify commands")]
    // Spotify(SpotifyOpts),
    // #[cfg(feature = "youtube")]
    // #[clap(name = "youtube", about = "youtube commands")]
    // Youtube(YoutubeOpts),
}


#[derive(Parser, Debug, Clone)]
pub struct BackendOpts {
    #[clap(subcommand)]
    pub command: Option<BackendCommand>,
    // #[clap(subcommand)]
    // pub sink_command: Option<SinkCommand>,
}

#[derive(Parser, Debug, Clone)]
pub struct YoutubeOpts {
    // #[clap(short = 'd', long = "device")]
    // pub device: Option<String>,
}
