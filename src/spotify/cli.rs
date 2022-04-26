use anyhow::Result;
use clap::Parser;
use futures::{Future, Stream};
use futures_util::{StreamExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde_json;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
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
pub struct TrackDownloadOptions {
    #[structopt(flatten)]
    download_opts: DownloadOptions,
}

#[derive(Parser, Debug, Clone)]
pub struct TrackListOptions {}

#[derive(Parser, Debug, Clone)]
pub struct PlaylistDownloadOptions {
    #[structopt(flatten)]
    download_opts: DownloadOptions,
}

#[derive(Parser, Debug, Clone)]
pub struct PlaylistListOptions {}

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
}

#[derive(Parser, Debug, Clone)]
pub struct PlaylistOptions {
    #[clap(subcommand)]
    pub command: PlaylistCommand,

    #[clap(long = "id")]
    pub id: Option<String>,
    #[clap(long = "name")]
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

    #[clap(long = "user-id")]
    pub user_id: Option<String>,

    #[clap(long = "api-token")]
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
    bar: Arc<ProgressBar>,
}

impl PlaylistFetchProgress {
    pub fn new(mp: &MultiProgress) -> Self {
        let bar = mp.add(ProgressBar::new_spinner());
        let style = ProgressStyle::default_spinner()
            .template("{spinner:.cyan} [{elapsed_precise}] {wide_msg} ({per_sec})");
        // .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})")
        // .progress_chars("#-");
        bar.set_style(style);
        Self { bar: Arc::new(bar) }
    }
}

#[derive(Debug)]
struct TrackDownloadProgress {
    bar: ProgressBar,
    // bar: ProgressBar,
    // total: u64,
    // downloaded: u64,
}

impl TrackDownloadProgress {
    pub fn new(mp: &MultiProgress, total: u64) -> Self {
        let bar = mp.add(ProgressBar::new(total));
        let style = ProgressStyle::default_bar()
            // .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {msg} ({bytes_per_sec}, {eta})")
            .progress_chars("#-");

        bar.set_style(style);
        Self {
            bar,
            // total,
            // downloaded: 0,
        }
    }

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

// pub fn playlist_list(
//     runtime: tokio::runtime::Runtime,
//     mut shutdown_tx: broadcast::Sender<bool>,
//     options: Options,
// ) -> Result<()> {

pub struct CLI {
    tool: crate::DjTool,
    runtime: tokio::runtime::Runtime,
    shutdown_tx: broadcast::Sender<bool>,
    options: Options,
    // mp: MultiProgress,
    // renderer: Arc<ProgressRenderer>,
    // progresses: Arc<RwLock<HashMap<String, Arc<Mutex<Progress>>>>>,
    // progresses: Arc<RwLock<HashMap<String, Box<dyn Progress + Sync + Send + 'static>>>>,
}

impl CLI {
    pub fn parse(
        runtime: tokio::runtime::Runtime,
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

    pub fn track_download(&self, mp: &MultiProgress) -> tokio::task::JoinHandle<()> {
        let tool = self.tool.clone();
        let options = self.options.clone();
        // let renderer = self.renderer.clone();
        // let progresses = self.progresses.clone();
        // let mp = MultiProgress::new();

        // let status = SpotifyProgress::Status(Status::new(mp.add(ProgressBar::new_spinner())));
        // let mut status = Status::new(mp.add(ProgressBar::new_spinner()));
        let mut status = Status::new(&mp);
        let mut downloaded = TrackDownloadProgress::new(&mp, 100);
        // ProgressBar::new_spinner());
        // mp.add(status.bar);
        // let status = Arc::new(mp.add(ProgressBar::new_spinner()));
        // let sty = ProgressStyle::default_bar();
        // status.set_style(sty.clone());
        // let test = "some track";
        // status.set_message(format!("downloading #{}", test));

        status.set_status("test".to_string());
        for i in 0..100 {
            // println!("downloaded {}", i);
            // downloaded.set_downloaded(i);
            downloaded.bar.tick();
            status.bar.tick();
            thread::sleep(Duration::from_secs(1));
        }

        self.runtime.spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            // status.finish();
            // mp.join_and_clear().expect("finish progress");
        })
    }

    // pub fn playlist_list(&self) -> tokio::task::JoinHandle<()> {
    pub fn playlist_list(&self, concurrency: Option<usize>, limit: Option<usize>) -> Result<()> {
        let mp = Arc::new(MultiProgress::new());
        let tool = self.tool.clone();
        let options = self.options.clone();
        // let renderer = self.renderer.clone();
        // let progresses = self.progresses.clone();
        let pb = PlaylistFetchProgress::new(&mp);
        pb.bar.tick();

        self.runtime.spawn(async move {
            // let tool = crate::DjTool::persistent(None::<PathBuf>).await.unwrap();

            // let tool_clone = tool.clone();
            // let test = tokio::task::spawn(async move {
            //     tool_clone.serve(self.shutdown_tx).await;
            //     std::process::exit(0);
            // });

            let tool_clone = tool.clone();
            // tokio::task::spawn(async move {

            // let spotify_client =
            //     crate::spotify::Spotify::pkce(&tool_clone.data_dir.as_ref().unwrap(), creds, oauth)
            //         .await
            //         .unwrap();

            // tool_clone.sources.write().await.insert(
            //     crate::proto::djtool::Service::Spotify,
            //     Arc::new(Box::new(spotify_client)),
            // );

            // println!("connected");
            let sources = tool_clone.sources.read().await;
            // let client = &sources[&crate::proto::djtool::Service::Spotify];
            // println!("reauthenticate");
            // match client.reauthenticate().await {
            //     Ok(Some(auth_url)) => {
            //         tool_clone.request_user_login(auth_url).await.unwrap();
            //     }
            //     Err(err) => panic!("{}", err),
            //     _ => {}
            // };
            // return ();

            let user_id = options.user_id.unwrap();
            let playlist_stream = tool_clone.all_playlists_stream(&user_id, &sources.deref());
            // if let Some(limit) = limit {
            //     let playlist_stream = playlist_stream.take(limit).into_inner();
            // }
            // let playlist_stream = match limit {
            //     Some(limit) => ,
            //     None => playlist_stream,
            // };
            let playlists = Arc::new(RwLock::new(Vec::<super::proto::djtool::Playlist>::new()));

            let total = Arc::new(AtomicUsize::new(0));
            // let total = Arc::new(AtomicUsize::new(0));
            // tokio::task::spawn_blocking(move || {
            //     status.set_message("hello test");
            // });

            // let item = ProgressItem::Status("status".to_string());
            // let status = Arc::new(Mutex::new(Progress::Spinner(SpinnerProgress {
            //     message: "test".to_string(),
            // })));
            // progresses.write().await.insert(item, status.clone());
            // renderer.set_status("test".to_string());

            // pb.inc(1);
            playlist_stream
                .for_each_concurrent(
                    concurrency,
                    |(_, playlist): (_, super::proto::djtool::Playlist)| {
                        let playlists = playlists.clone();
                        // let renderer = renderer.clone();
                        // let progresses = progresses.clone();
                        // let status = status.clone();
                        let total = total.clone();
                        let pb_clone = pb.clone();
                        async move {
                            // new entry

                            // let bar = renderer.mp.add(ProgressBar::new(100));
                            // bar.inc(1);
                            // let style = ProgressStyle::default_bar();
                            // bar.set_style(style.clone());
                            // // bar.enable_steady_tick(100);
                            // let id = playlist.id.as_ref().unwrap().id.clone();
                            // renderer.bars.write().await.insert(id.clone(), bar);
                            // let bar = &renderer.bars.read().await[&id];
                            pb_clone
                                .bar
                                .set_message(format!("Playlist: {}", playlist.name));
                            // pb_clone.bar.inc(1);
                            total.fetch_add(1, Ordering::SeqCst);
                            {
                                // let mut s = status.lock().await;
                                // if let Progress::Spinner(ref mut p) = s.deref_mut() {
                                //     p.message = "test 2".to_string();
                                // }
                            }
                            // renderer.update(progresses.read().await.deref()).await;
                            // if !progresses.read().await.contains_key(&item) {
                            //     // insert
                            // }

                            // let test = progresses.read().await;
                            // match test.get_mut(&item) {
                            //     Some(p) => {
                            //         if let Progress::Spinner(pp) = p.lock().await.deref() {
                            //             pp.message = "test 2".to_string();
                            //         }
                            //     }
                            //     // Some(_) => {},
                            //     None => {
                            //         progresses.write().await.insert(
                            //             item,
                            //             Mutex::new(Progress::Spinner(SpinnerProgress {
                            //                 message: "test".to_string(),
                            //             })),
                            //         );
                            //     } // *x = "b";
                            // };

                            // println!("{}", total.load(Ordering::Relaxed));
                            // tokio::task::spawn_blocking(move || {
                            // pb.inc(1);
                            // status.set_message(format!(
                            //     "playlist {}/{}",
                            //     total.load(Ordering::Relaxed),
                            //     total.load(Ordering::Relaxed)
                            // ));
                            // });

                            // simulate some processing time
                            // tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

                            playlists.write().await.push(playlist);
                            {
                                // let mut s = status.lock().await;
                                // if let Progress::Spinner(ref mut p) = s.deref_mut() {
                                //     p.message = "test 2".to_string();
                                // }
                            }
                            // renderer.update(progresses.deref());

                            // bar.finish_with_message("done");
                            // renderer.mp.remove(&bar);
                            // renderer.bars.write().await.remove(&id);
                        }
                    },
                )
                .await;

            let test = playlists.read().await;
            let result = serde_json::to_string_pretty(&test.deref()).unwrap();
            // println!("{}", result);
            // println!("done");
            pb.bar.finish();
            // tokio::task::spawn_blocking(move || {
            //     pb.finish_with_message("done");
            //     status.finish_with_message("done");
            // });
            // });
            // println!("done 2");
        });
        mp.join();
        Ok(())
    }

    pub fn handle(&self) -> Result<()> {
        let task = match &self.options.command {
            Command::Track(options) => {
                println!("track options: {:?}", options);
                match &options.command {
                    TrackCommand::List(list) => {
                        println!("track list options: {:?}", list);
                        // self.track_list()
                        // self.runtime.spawn(async move {})
                    }
                    TrackCommand::Download(download) => {
                        println!("track download options: {:?}", download);
                        // self.track_download(&mp)
                        // self.runtime.spawn(async move {})
                    }
                }
            }
            Command::Playlist(options) => {
                println!("playlist options: {:?}", options);
                match &options.command {
                    PlaylistCommand::List(list) => {
                        println!("playlist list options: {:?}", list);
                        self.playlist_list(Some(8), None);
                    }
                    PlaylistCommand::Download(download) => {
                        println!("playlist download options: {:?}", download);
                        // self.runtime.spawn(async move {})
                    }
                }
            }
        };
        // println!("waiting for the task to complete");
        // self.runtime.block_on(async move {
        //     task.await;
        // });
        return Ok(());

        println!("waiting for the task to complete");
        // let (tx, rx) = std::sync::mpsc::channel::<Box<dyn Progress + Sync + Send + 'static>>();
        // let (tx, rx) = std::sync::mpsc::channel::<Option<u64>>();

        // let status = mp.add(ProgressBar::new_spinner());
        // let pb = Arc::new(mp.add(ProgressBar::new(128)));
        // let pb = Arc::new(mp.add(ProgressBar::new(20)));
        // let pb = Arc::new(ProgressBar::new(128));
        // let mut pb = Box::new(TrackDownloadProgress::new(&mp, 20));
        // let mut pb = TrackDownloadProgress::new(&mp, 20);
        // pb.bar.tick();

        // let ui = thread::spawn(move || {
        //     // sender.send(expensive_computation()).unwrap();
        //     println!("start");
        //     for progress in rx {
        //         pb.inc(progress);
        //         // progress.update();
        //         // progress.inc(1);
        //     }
        //     println!("done");
        // });

        // let send = tx.clone();
        self.runtime.spawn(async move {
            // let h1 = thread::spawn(move || {
            // pb.bar.set_message(format!("download #{}", 12));
            for i in 0..20 {
                // println!("inc {}", i);
                // pb.inc(1);
                // pb.bar.inc(i);
                // tx.send(pb.clone()).unwrap();
                // tx.send(Some(1)).unwrap();
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                // thread::sleep(Duration::from_millis(15));
            }
            // pb.bar.finish();
            // tx.send(None).unwrap();
            // });
            // m.join_and_clear()?;
            // task.await;
            // pb.finish_with_message("done");
            // status.finish_with_message("done");
            // let test = tokio::task::spawn_blocking(move || mp.join_and_clear().unwrap());
            // test.await;
        });
        // mp.join_and_clear()?;
        // test.join();

        let ui = thread::spawn(move || {
            // println!("start");
            // for progress in rx {
            //     match progress {
            //         Some(p) => {
            //             pb.inc(p);
            //             pb.tick();
            //         }
            //         None => pb.finish(),
            //     }
            //     // progress.update();
            //     // progress.inc(1);
            // }
        });
        // mp.join();
        // mp.join_and_clear();
        // pb.finish();
        println!("done");

        ui.join();
        println!("waiting for progress bars to complete");
        // self.renderer.finish()?;
        println!("end");
        Ok(())
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
