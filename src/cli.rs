use clap::Parser;
use std::path::PathBuf;
// use djtool::{DjTool, SPLASH_LOGO};

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
pub struct SpotifyOpts {
    // #[clap(short = 'd', long = "device")]
// pub device: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct YoutubeOpts {
    // #[clap(short = 'd', long = "device")]
// pub device: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub enum Command {
    // #[clap(name = "start", about = "start the server")]
    // Start(StartOpts),
    #[cfg(feature = "spotify")]
    #[clap(name = "spotify", about = "spotify commands")]
    Spotify(SpotifyOpts),
    #[cfg(feature = "youtube")]
    #[clap(name = "youtube", about = "youtube commands")]
    Youtube(YoutubeOpts),
}

#[derive(Parser, Debug, Clone)]
#[clap(version = "1.0", author = "romnn <contact@romnn.com>")]
pub struct Opts {
    // #[cfg(feature = "record")]
    // #[clap(short = 'i', long = "input-device")]
    // pub input_device: Option<String>,

    // #[cfg(feature = "record")]
    // #[clap(short = 'o', long = "output-device")]
    // pub output_device: Option<String>,

    // #[cfg(feature = "record")]
    // #[clap(long = "latency", default_value = "5")]
    // pub latency: u32,

    // #[cfg(use_jack)]
    // #[clap(long = "jack", about = "use jack audio backend")]
    // pub use_jack: bool,

    // #[cfg(feature = "portaudio")]
    // #[clap(long = "portaudio", about = "use portaudio audio backend")]
    // pub use_portaudio: bool,
    #[clap(short = 'p', long = "port", default_value = "21011")]
    pub port: u16,
    #[clap(long = "host")]
    pub host: Option<String>,
    #[clap(long = "cache")]
    pub cache_dir: Option<PathBuf>,
    #[clap(long = "library")]
    pub library_dir: Option<PathBuf>,
    // #[clap(short = 'q', long = "quality")]
    // pub quality: Option<Quality>,
    #[clap(long = "api")]
    pub api: bool,
    #[clap(long = "headless")]
    pub headless: bool,

    #[clap(subcommand)]
    pub subcommand: Option<Command>,
}
