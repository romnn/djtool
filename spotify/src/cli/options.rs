use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
pub struct PerTrackListOptions {
    #[clap(long = "artwork")]
    pub artwork: bool,
    #[clap(long = "preview")]
    pub preview: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct PaginationOptions {
    #[clap(long = "offset")]
    pub offset: u32,
    #[clap(long = "limit")]
    pub limit: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct DownloadOptions {
    #[clap(long = "artwork")]
    pub artwork: bool,
    #[clap(long = "preview")]
    pub preview: bool,
    #[clap(long = "quality")]
    pub quality: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub struct ListOptions {
    #[clap(long = "json", help = "json output file")]
    pub json: Option<PathBuf>,
    #[clap(long = "print", help = "print output in the end")]
    pub print: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct TrackDownloadOptions {
    #[structopt(flatten)]
    download_opts: DownloadOptions,
    #[clap(long = "choose", help = "manually choose the download candidate")]
    pub choose: bool,
    // #[clap(long = "limit", help = "maximum number of candidates to consider")]
    // pub limit: Option<usize>,
}

#[derive(Parser, Debug, Clone)]
pub struct TrackListOptions {
    #[structopt(flatten)]
    list_opts: ListOptions,
}

#[derive(Parser, Debug, Clone)]
pub struct PlaylistDownloadOptions {
    #[structopt(flatten)]
    download_opts: DownloadOptions,
}

#[derive(Parser, Debug, Clone)]
pub struct PlaylistListOptions {
    #[structopt(flatten)]
    list_opts: ListOptions,

    #[clap(long = "limit")]
    pub limit: Option<usize>,
}

#[derive(Parser, Debug, Clone)]
pub enum TrackCommand {
    #[clap(name = "download", about = "")]
    Download(TrackDownloadOptions),
    #[clap(name = "list", about = "")]
    List(TrackListOptions),
}

#[derive(Parser, Debug, Clone)]
pub enum PlaylistCommand {
    #[clap(name = "download", about = "")]
    Download(PlaylistDownloadOptions),
    #[clap(name = "list", about = "")]
    List(PlaylistListOptions),
}

#[derive(Parser, Debug, Clone)]
pub struct CommonTrackOptions {
    #[clap(long = "track-id", alias = "id", env = "TRACK_ID")]
    pub id: Option<String>,
    #[clap(long = "name", env = "TRACK_NAME")]
    pub name: Option<String>,
    #[clap(long = "artist", env = "TRACK_ARTIST")]
    pub artist: Option<String>,
    #[clap(
        long = "source-limit",
        help = "maximum number of source candidates to consider",
        env = "TRACK_SOURCE_LIMIT"
    )]
    pub source_limit: Option<usize>,
    #[clap(
        long = "sink-limit",
        help = "maximum number of sink candidates to consider",
        env = "TRACK_SINK_LIMIT"
    )]
    pub sink_limit: Option<usize>,
}

#[derive(Parser, Debug, Clone)]
pub struct TrackOptions {
    #[structopt(flatten)]
    common: CommonTrackOptions,
    #[clap(subcommand)]
    pub command: TrackCommand,
}

#[derive(Parser, Debug, Clone)]
pub struct PlaylistOptions {
    #[clap(subcommand)]
    pub command: PlaylistCommand,

    #[clap(long = "playlist-id", alias = "id", env = "SPOTIFY_PLAYLIST_ID")]
    pub id: Option<String>,
    #[clap(long = "name", env = "SPOTIFY_PLAYLIST_NAME")]
    pub name: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub enum Command {
    #[clap(name = "track", about = "")]
    Track(TrackOptions),
    #[clap(name = "playlist", about = "")]
    Playlist(PlaylistOptions),
}

#[derive(Parser, Debug, Clone)]
pub struct Options {
    // #[structopt(flatten)]
    // backend_opts: crate::cli::BackendOpts,
    #[clap(subcommand)]
    pub command: Command,

    #[clap(long = "user-id", alias = "user", env = "SPOTIFY_USER_ID")]
    pub user_id: Option<String>,

    #[clap(long = "api-token", env = "SPOTIFY_API_TOKEN")]
    pub api_token: Option<String>,
}
