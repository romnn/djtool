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
pub fn human_duration(d: chrono::Duration) -> String {
    // let mut owned_string: String = "hello ".to_owned();
    // let mut res = String::new();
    if d.num_hours() > 0 {
        // res.push_str(format!("{}:{:02}:{02}", d.num_hours(), d.num_minutes(), d.num_seconds()))
        return format!(
            "{}:{:02}:{02}",
            d.num_hours(),
            d.num_minutes().rem_euclid(60),
            d.num_seconds().rem_euclid(60),
        );
    }
    // if d.num_minutes() > 0 {
    //     return format!(
    //         "{}:{:02}",
    //         d.num_minutes().rem_euclid(60),
    //         d.num_seconds().rem_euclid(60)
    //     );
    //     // res.push_str(format!("{}:", d.num_hours()))
    // }
    format!(
        "{:02}:{:02}",
        d.num_minutes().rem_euclid(60),
        d.num_seconds().rem_euclid(60)
    )
    // {:04}
    // res.push_str(fmt.Sprintf("%01dh);
    // mins = d.num_minutes();
    // mins = d.num_minutes();
    // d = d.Round(time.Minute)
    // h := d / time.Hour
    // d -= h * time.Hour
    // m := d / time.Minute
    // return fmt.Sprintf("%02d:%02d", h, m)
    // res
}

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
//
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
