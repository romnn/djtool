use crate::download;
use crate::proto;
use crate::sink;
use crate::utils;
use anyhow::Result;
use clap::Parser;
use futures::{Future, Stream};
use futures_util::{StreamExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde_json;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempdir::TempDir;
use tokio::sync::{broadcast, Mutex, RwLock};

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
pub struct TrackOptions {
    #[clap(subcommand)]
    pub command: TrackCommand,
    #[clap(long = "track-id", alias = "id", env = "SPOTIFY_TRACK_ID")]
    pub id: Option<String>,
    #[clap(long = "name", env = "SPOTIFY_TRACK_NAME")]
    pub name: Option<String>,
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

// pub async fn connect_spotify() {
//     let creds = spotify::auth::Credentials::pkce("893474f878934ae89fff417e4722e147");
//     let oauth = spotify::auth::OAuth {
//         redirect_uri: format!(
//             "http://{}:{}/spotify/pkce/callback",
//             self.host.to_string(),
//             self.port
//         ),
//         scopes: scopes!("playlist-read-private"),
//         ..Default::default()
//     };
//     println!("redirect url host: {}", oauth.redirect_uri);
//     if let Err(err) = self.connect_spotify(creds, oauth).await {
//         println!("spotify connect error: {}", err);
//     }
// }
//
// pub trait Progress {
//     fn progress(&self) -> &ProgressBar;
//     fn update(&self) -> ();
// }

// #[derive(Debug)]
// struct BarProgress {
//     total: u32,
//     done: u32,
//     message: String,
// }

#[derive(Debug, Clone)]
struct PlaylistFetchProgress {
    // bar: Arc<ProgressBar>,
}

impl PlaylistFetchProgress {
    pub fn style(bar: &ProgressBar) {
        // let bar = mp.add(ProgressBar::new_spinner());
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.cyan} [{elapsed_precise}] {wide_msg} ({per_sec})");
        // .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})")
        // .progress_chars("#-");
        bar.set_style(style);
        bar.enable_steady_tick(1_000 / 30);
        // Self { bar: Arc::new(bar) }
    }

    // pub fn new(mp: &MultiProgress) -> Self {
    //     let bar = mp.add(ProgressBar::new_spinner());
    //     let style = ProgressStyle::default_spinner()
    //         .template("{spinner:.cyan} [{elapsed_precise}] {wide_msg} ({per_sec})");
    //     // .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})")
    //     // .progress_chars("#-");
    //     bar.set_style(style);
    //     Self { bar: Arc::new(bar) }
    // }
}

#[derive(Debug)]
struct TrackDownloadProgress {
    // bar: ProgressBar,
// bar: ProgressBar,
// total: u64,
// downloaded: u64,
}

impl TrackDownloadProgress {
    pub fn style(bar: &ProgressBar) {
        // let bar = mp.add(ProgressBar::new_spinner());
        let style = ProgressStyle::default_spinner()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})")
            .progress_chars("#-");
        bar.set_style(style);
        bar.enable_steady_tick(1_000 / 30);
        // Self { bar: Arc::new(bar) }
    }

    // pub fn new(mp: &MultiProgress, total: u64) -> Self {
    //     let bar = mp.add(ProgressBar::new(total));
    //     let style = ProgressStyle::default_bar()
    //         // .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
    //         .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})")
    //         .progress_chars("#-");

    //     bar.set_style(style);
    //     Self {
    //         bar,
    //         // total,
    //         // downloaded: 0,
    //     }
    // }

    // pub fn set_downloaded(&mut self, val: u64) {
    //     self.downloaded = val;
    // }

    // pub fn set_total(&mut self, total: u64) {
    //     // self.bar.set_length(total);
    // }
}

// impl Progress for TrackDownloadProgress {
//     fn progress(&self) -> &ProgressBar {
//         &self.bar
//     }

//     fn update(&self) -> () {
//         println!("updating");
//         self.bar.set_position(self.downloaded);
//         self.bar.set_length(self.total);
//         self.bar.tick();
//     }
// }

#[derive(Debug)]
struct Status {
    bar: ProgressBar,
    // message: String,
}

impl Status {
    // pub fn new(bar: ProgressBar) -> Self {
    pub fn new(mp: &MultiProgress) -> Self {
        let bar = mp.add(ProgressBar::new_spinner());
        let sty = ProgressStyle::default_bar();
        bar.set_style(sty.clone());
        Self {
            bar,
            // message: "".to_string(),
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.bar.set_message(msg);
        // self.message = msg;
    }
}

// impl Progress for Status {
//     fn progress(&self) -> &ProgressBar {
//         &self.bar
//     }

//     fn update(&self) -> () {}
// }

#[derive(Debug)]
enum SpotifyProgress {
    Status(Status),
    // Bar(String),
    // Spinner(String),
}

// #[derive(Debug)]
// enum Progress {
//     Bar(BarProgress),
//     Spinner(SpinnerProgress),
// }

// struct ProgressState {
//     // in a hash map
//     // for each bar: current message, total, current, value
//     bars: Arc<RwLock<HashMap<String, ProgressBar>>>,
// }

// #[derive(Clone)]
struct ProgressRenderer {
    mp: MultiProgress,
    status: Arc<RwLock<Option<ProgressBar>>>,
    // bars: Arc<RwLock<HashMap<ProgressItem, ProgressBar>>>,
    // bars: Arc<RwLock<HashMap<String, Box<dyn Progress + Send + 'static>>>>,
}

impl ProgressRenderer {
    pub fn new() -> Self {
        let mp = MultiProgress::new();

        let style = ProgressStyle::default_bar();
        let style = style
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-");

        Self {
            mp,
            status: Arc::new(RwLock::new(None)),
            // bars: Arc::new(RwLock::new(HashMap::new())),
            // Arc::new(m.add(ProgressBar::new_spinner())),
        }
    }

    // pub async fn set_status(&self, msg: String) {
    //     self.status.write().await.set_message(msg);
    // }
    // clear_status

    // pub async fn update(&self, update: &HashMap<ProgressItem, Arc<Mutex<Progress>>>) -> () {
    //     // add or remove status
    //     // remove any stale bars
    //     self.bars.write().await.retain(|k, bar| {
    //         if update.contains_key(k) {
    //             return true;
    //         };
    //         self.mp.remove(bar);
    //         false
    //     });
    //     // add any new bars
    //     // for (&k, progress) in &*update {
    //     for (&k, progress) in update.iter() {
    //         println!("{:?} {:?}", k, progress);
    //         let bar = self.bars.write().await.entry(k).or_insert_with(|| {
    //             let bar = ProgressBar::new_spinner();
    //             let bar = self.mp.add(bar);
    //             let style = ProgressStyle::default_bar();
    //             bar.set_style(style.clone());
    //             bar
    //         });
    //         // update the bar
    //         // bar
    //         // match self.bars.write().await.entry(k) {
    //         //     Occupied(entry) => {
    //         //         // update the bar
    //         //     }
    //         //     Vacant(entry) => {
    //         //         let bar = ProgressBar::new_spinner();
    //         //         let bar = self.mp.add(bar);
    //         //         let style = ProgressStyle::default_bar();
    //         //         bar.set_style(style.clone());
    //         //         entry.insert(progress);
    //         //     }
    //         // }
    //     }
    // }

    pub fn finish(&self) -> Result<()> {
        self.mp.join_and_clear()?;
        Ok(())
    }
}

pub struct CLI<'a> {
    tool: crate::DjTool,
    runtime: &'a tokio::runtime::Runtime,
    shutdown_tx: broadcast::Sender<bool>,
    options: Options,
    // progresses: Arc<RwLock<HashMap<String, Arc<Mutex<Progress>>>>>,
    // progresses: Arc<RwLock<HashMap<String, Box<dyn Progress + Sync + Send + 'static>>>>,
}

impl<'a> CLI<'a> {
    pub fn parse(
        runtime: &'a tokio::runtime::Runtime,
        mut shutdown_tx: broadcast::Sender<bool>,
        options: Options,
    ) -> () {
        // let renderer = Arc::new(ProgressRenderer::new());
        // let progresses = Arc::new(RwLock::new(
        //     // HashMap::<ProgressItem, Arc<Mutex<Progress>>>::new(),
        //     HashMap::new(),
        // ));
        let shutdown_tx_clone = shutdown_tx.clone();
        let tool = runtime.block_on(async move {
            let tool = crate::DjTool::persistent(None::<PathBuf>).await.unwrap();

            let tool_clone = tool.clone();
            tokio::task::spawn(async move {
                tool_clone.serve(shutdown_tx_clone).await;
                std::process::exit(0);
            });

            let creds = super::auth::Credentials::pkce("893474f878934ae89fff417e4722e147");
            let oauth = super::auth::OAuth {
                redirect_uri: format!(
                    "http://{}:{}/spotify/pkce/callback",
                    tool.host.to_string(),
                    tool.port
                ),
                scopes: crate::scopes!("playlist-read-private"),
                ..Default::default()
            };
            tool.connect_spotify(creds, oauth).await.unwrap();
            println!("connected");
            tool
        });

        // let mp = MultiProgress::new();
        let cli = Self {
            tool,
            runtime,
            shutdown_tx,
            options,
            // renderer,
            // mp,
            // progresses,
        };
        cli.handle().unwrap();
    }

    pub fn track_download(
        &self,
        track_opts: TrackOptions,
        dl_opts: TrackDownloadOptions,
    ) -> Result<()> {
        let mp = Arc::new(MultiProgress::new());
        let mp_clone = mp.clone();
        let tool = self.tool.clone();
        let spfy_opts = self.options.clone();

        // let pb = TrackDownloadProgress::new(&mp, 100);
        // pb.bar.tick();
        // let bar = Arc::new(mp.add(ProgressBar::new_spinner()));
        // TrackDownloadProgress::style(&bar);
        // bar.tick();

        let user_id = spfy_opts.user_id.unwrap();
        // let track = track.ok_or(anyhow::anyhow!("no track found"))?;

        let handle = self.runtime.spawn(async move {
            let res: Result<(), anyhow::Error> = async {
                // find the spotify track
                let sources = tool.sources.read().await;
                let sinks = tool.sinks.read().await;
                let source = &sources[&proto::djtool::Service::Spotify];
                // todo: make this configurable
                let sink = &sinks[&proto::djtool::Service::Youtube];

                let mut track = None;
                if let Some(track_id) = track_opts.id {
                    track = source.track_by_id(&track_id).await?;
                } else if let Some(track_name) = track_opts.name {
                    // source.track_by_name(track_name);
                }

                let track = track.ok_or(anyhow::anyhow!("no track found"))?;
                // println!("track: {:?}", track);

                // let title = track.name.to_owned();
                // let artist = track.artist.to_owned();
                let filename = format!("{} - {}", track.name, track.artist);
                // let filename_clone = filename.clone();
                // let filename = "test".to_string();
                // println!("filename: {}", filename);

                // let sinks_lock = sinks_lock.clone();

                let filename = utils::sanitize_filename(&filename);
                let temp_dir = TempDir::new(&filename)?;
                // youtube by default
                // let track = &sinks[&proto::djtool::Service::Youtube];
                // let bar = Arc::new(mp_clone.add(ProgressBar::new_spinner()));
                // TrackDownloadProgress::style(&bar);
                // bar.tick();

                let candidates = sink
                    .candidates(
                        &track,
                        Box::new(|progress: sink::QueryProgress| {}),
                        Some(10),
                    )
                    .await?;
                println!("found {} candidates", candidates.len());

                let download_track = candidates.first().ok_or(anyhow::anyhow!("no download"))?;

                let downloaded = sink
                    .download(
                        &download_track,
                        &temp_dir.path().to_path_buf().join("audio"),
                        None,
                        Box::new(|progress: download::DownloadProgress| {
                            println!(
                                "downloaded: {} {:?}",
                                progress.downloaded,
                                progress.total.unwrap()
                            );
                            io::stdout().flush().unwrap();
                            io::stderr().flush().unwrap();
                        }),
                    )
                    .await?;
                println!("download done");
                // transcode
                // let library_dir = {
                //     let config = self.config.read().await;
                //     config.as_ref().map(|c| c.library.library_dir.to_owned())
                // }
                // .ok_or(anyhow::anyhow!("no library"))?;
                Ok::<(), anyhow::Error>(())
            }
            .await;
            // bar.finish();
            res

            // {
            //     // print error
            //     eprintln!("error: {}", err);
            //     // pb.bar.finish();
            //     // most importantly, clean up the progress
            // };
        });

        // mp.join_and_clear()?;
        self.runtime.block_on(async move { handle.await? })
        // let playlist_stream = tool.all_playlists_stream(&user_id, &sources.deref());
        // let playlist_stream = match opts.limit {
        //     Some(limit) => playlist_stream.take(limit).into_inner(),
        //     None => playlist_stream,
        // };

        // let playlists = Arc::new(RwLock::new(Vec::<super::proto::djtool::Playlist>::new()));
        // let total = Arc::new(AtomicUsize::new(0));
        // let pb = TrackDownloadProgress::new(&mp);
        // pb.bar.tick();

        // let status = SpotifyProgress::Status(Status::new(mp.add(ProgressBar::new_spinner())));
        // let mut status = Status::new(mp.add(ProgressBar::new_spinner()));
        // let mut status = Status::new(&mp);
        // let mut downloaded = TrackDownloadProgress::new(&mp, 100);
        // ProgressBar::new_spinner());
        // mp.add(status.bar);
        // let status = Arc::new(mp.add(ProgressBar::new_spinner()));
        // let sty = ProgressStyle::default_bar();
        // status.set_style(sty.clone());
        // let test = "some track";
        // status.set_message(format!("downloading #{}", test));
        // mp.join_and_clear();
        // mp.join();
        // Ok(())
    }

    pub fn playlist_list(
        &self,
        plist_opts: PlaylistOptions,
        list_opts: PlaylistListOptions,
        concurrency: Option<usize>,
    ) -> Result<()> {
        let mp = Arc::new(MultiProgress::new());
        let tool = self.tool.clone();
        let spfy_opts = self.options.clone();

        let user_id = spfy_opts.user_id.unwrap();
        let total = Arc::new(AtomicUsize::new(0));
        let bar = Arc::new(mp.add(ProgressBar::new_spinner()));
        PlaylistFetchProgress::style(&bar);
        bar.tick();

        let handle = self.runtime.spawn(async move {
            let res: Result<(), anyhow::Error> = async {
                let sources = tool.sources.read().await;
                let source = &sources[&proto::djtool::Service::Spotify];

                if let Some(playlist_id) = plist_opts.id {
                    // get the single playlist and its tracks

                    // println!("tracks for playlist");
                    let tracks = Arc::new(RwLock::new(Vec::<super::proto::djtool::Track>::new()));
                    let playlist = source.playlist_by_id(&playlist_id).await?;
                    let playlist = playlist.ok_or(anyhow::anyhow!("no playlist found"))?;
                    let tracks_stream = source.user_playlist_tracks_stream(playlist)?;
                    tracks_stream
                        .filter_map(|track: Result<proto::djtool::Track>| {
                            async {
                                match track {
                                    Ok(track) => Some(track),
                                    Err(err) => {
                                        eprintln!("track error: {}", err);
                                        // {
                                        //     let mut fp = tracks_failed.lock().await;
                                        //     *fp += 1;
                                        // };
                                        None
                                    }
                                }
                            }
                        })
                        .for_each_concurrent(
                            concurrency,
                            |(track): (super::proto::djtool::Track)| {
                                let total = total.clone();
                                let bar_clone = bar.clone();
                                let tracks = tracks.clone();
                                async move {
                                    total.fetch_add(1, Ordering::SeqCst);
                                    bar_clone.println(format!(
                                        "{}: {} ({})",
                                        track
                                            .id
                                            .as_ref()
                                            .map(|id| id.to_string())
                                            .unwrap_or("".to_string()),
                                        &track.name,
                                        crate::cli::human_duration(chrono::Duration::seconds(
                                            track.duration_secs as i64
                                        ))
                                    ));
                                    bar_clone.set_message(format!(
                                        "Track: {} ({} done)",
                                        &track.name,
                                        total.load(Ordering::Relaxed)
                                    ));
                                    tracks.write().await.push(track);
                                }
                            },
                        )
                        .await;

                    let tracks = tracks.read().await;
                    let result = serde_json::to_string_pretty(&tracks.deref()).unwrap();
                    if list_opts.list_opts.print {
                        let pretty = serde_json::to_string_pretty(&tracks.deref());
                        pretty.map(|res| println!("{}", res));
                    }
                    if let Some(out) = list_opts.list_opts.json {
                        OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(out)
                            .map(|file| serde_json::to_writer_pretty(file, &tracks.deref()));
                    }
                } else {
                    // get all playlists of the user
                    let playlists =
                        Arc::new(RwLock::new(Vec::<super::proto::djtool::Playlist>::new()));
                    let playlist_stream = tool.all_playlists_stream(&user_id, &sources.deref());
                    let playlist_stream = match list_opts.limit {
                        Some(limit) => playlist_stream.take(limit).into_inner(),
                        None => playlist_stream,
                    };

                    playlist_stream
                        .for_each_concurrent(
                            concurrency,
                            |(_, playlist): (_, super::proto::djtool::Playlist)| {
                                let total = total.clone();
                                let bar_clone = bar.clone();
                                let playlists = playlists.clone();
                                async move {
                                    total.fetch_add(1, Ordering::SeqCst);
                                    bar_clone.println(format!(
                                        "{}: {} ({} tracks)",
                                        playlist
                                            .id
                                            .as_ref()
                                            .map(|id| id.to_string())
                                            .unwrap_or("".to_string()),
                                        &playlist.name,
                                        &playlist.total
                                    ));
                                    bar_clone.set_message(format!(
                                        "Playlist: {} ({} done)",
                                        &playlist.name,
                                        total.load(Ordering::Relaxed)
                                    ));
                                    playlists.write().await.push(playlist);
                                }
                            },
                        )
                        .await;

                    let playlists = playlists.read().await;
                    let result = serde_json::to_string_pretty(&playlists.deref()).unwrap();
                    if list_opts.list_opts.print {
                        let pretty = serde_json::to_string_pretty(&playlists.deref());
                        pretty.map(|res| println!("{}", res));
                    }
                    if let Some(out) = list_opts.list_opts.json {
                        OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(out)
                            .map(|file| serde_json::to_writer_pretty(file, &playlists.deref()));
                    }
                }
                Ok::<(), anyhow::Error>(())
            }
            .await;
            // let res = work.await
            // {
            //     eprintln!("sorry error: {}", err);
            // }
            // io::stdout().flush().unwrap();
            // io::stderr().flush().unwrap();
            bar.finish();
            res
            // tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        });
        // println!("{}", bar_clone.is_finished());
        // println!("waiting ...");
        // mp.join()?;
        mp.join_and_clear()?;
        // let res: Result<_> = self.runtime.block_on(async move {
        // if let Err(err) =
        self.runtime.block_on(async move { handle.await? })
        // eprintln!("sorry error: {}", err);
        // }
        // // println!("result: {:?}", res);
        // Ok(())
    }

    pub fn handle(&self) -> Result<()> {
        match &self.options.command {
            Command::Track(track_opts) => {
                println!("track options: {:?}", track_opts);
                match &track_opts.command {
                    TrackCommand::List(list_opts) => {
                        println!("track list options: {:?}", list_opts);
                        // self.track_list(list_opts.to_owned())
                        Ok(())
                        // self.runtime.spawn(async move {})
                    }
                    TrackCommand::Download(download_opts) => {
                        println!("track download options: {:?}", download_opts);
                        self.track_download(track_opts.to_owned(), download_opts.to_owned())
                        // self.runtime.spawn(async move {})
                    }
                }
            }
            Command::Playlist(pl_opts) => {
                println!("playlist options: {:?}", pl_opts);
                match &pl_opts.command {
                    PlaylistCommand::List(list_opts) => {
                        println!("playlist list options: {:?}", list_opts);
                        self.playlist_list(pl_opts.to_owned(), list_opts.to_owned(), Some(8))
                    }
                    PlaylistCommand::Download(dl_opts) => {
                        println!("playlist download options: {:?}", dl_opts);
                        Ok(())
                        // self.runtime.spawn(async move {})
                    }
                }
            }
        }
    }
}

// pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
// .unwrap()
// .with_key("eta", |state| format!("{:.1}s", state.eta().as_secs_f64()))
// .progress_chars("#>-"));
// let progresses = Arc::new(m.add(ProgressBar::new_spinner())),
// let progress = Progress::new();

// let m = MultiProgress::new();

// let status = Arc::new(m.add(ProgressBar::new_spinner()));
// let sty = ProgressStyle::default_bar();
// status.set_style(sty.clone());

// let bars = Vec::<ProgressBar>::new();
// for bar in bars {
//     m.add(ProgressBar::new(128))
// }
// let pb = m.add(ProgressBar::new(128));
// pb.set_style(style.clone());

// let pb2 = m.add(ProgressBar::new(128));
// let pb2 = m.insert_after(&pb, ProgressBar::new(128));
// pb2.set_style(style.clone());

// let pb3 = m.insert_after(&pb2, ProgressBar::new(1024));
// let pb3 = m.add(ProgressBar::new(1024));
// pb3.set_style(style);
// let (sender, receiver) = sync_channel(1);
// return Ok(());

// let h1 = thread::spawn(move || {
//     for i in 0..128 {
//         pb.set_message(format!("item #{}", i + 1));
//         pb.inc(1);
//         thread::sleep(Duration::from_millis(15));
//     }
//     pb.finish_with_message("done");
// });

// let h2 = thread::spawn(move || {
//     for _ in 0..3 {
//         pb2.set_position(0);
//         for i in 0..128 {
//             pb2.set_message(format!("item #{}", i + 1));
//             pb2.inc(1);
//             thread::sleep(Duration::from_millis(8));
//         }
//     }
//     pb2.finish_with_message("done");
// });

// let h3 = thread::spawn(move || {
//     for i in 0..1024 {
//         status.set_message(format!("testtt #{}", i + 1));
//         pb3.set_message(format!("item #{}", i + 1));
//         pb3.inc(1);
//         thread::sleep(Duration::from_millis(2));
//     }
//     pb3.finish_with_message("done");
// });

// m.join_and_clear()?;
// let _ = h1.join();
// let _ = h2.join();
// let _ = h3.join();
// m.clear().unwrap();
