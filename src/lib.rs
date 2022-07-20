#![allow(warnings)]

use tokio;
pub mod config;
#[cfg(feature = "debug")]
mod debug;
pub mod download;
pub mod ffmpeg;
mod ffmpeg_sys;
pub mod id3;
pub mod matching;
pub mod proto;
// pub mod error;
pub mod cli;
pub mod sink;
pub mod source;
pub mod spotify;
pub mod transcode;
pub mod utils;
pub mod youtube;

use anyhow::Result;
use config::Persist;
use dirs;
use futures::{Future, Stream};
use futures_util::pin_mut;
use futures_util::stream;
use futures_util::{StreamExt, TryStreamExt};
use reqwest;
use serde::Deserialize;
use spotify::auth::{Credentials, OAuth};
use spotify::model::{Id, PlaylistId, UserId};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::thread;
use tauri::Event;
use tempdir::TempDir;
// use std::sync::{mpsc as smpsc};
use tokio::sync::{broadcast, mpsc, oneshot, watch, Mutex, RwLock, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server as TonicServer, Code, Request, Response, Status};
use warp::{Filter, Rejection, Reply};
use youtube::Youtube;

pub const SPLASH_LOGO: &str = r"

 ___/ / (_) /____  ___  / /
/ _  / / / __/ _ \/ _ \/ / 
\_,_/_/ /\__/\___/\___/_/  
   |___/                   
";

pub type SpotifyClient = Arc<spotify::Spotify>;
pub type YoutubeClient = Arc<youtube::Youtube>;

pub type Source = Arc<Box<dyn source::Source + Send + Sync>>;
pub type Sink = Arc<Box<dyn sink::Sink + Send + Sync>>;

// pub type TrackStream<'a> = Pin<Box<dyn Stream<Item = Result<proto::djtool::Track>> + 'a + Send>>;
pub type TrackStream<'a> = Pin<Box<dyn Stream<Item = proto::djtool::Track> + 'a + Send>>;

#[derive(Clone)]
// pub struct DjTool<'a> {
pub struct DjTool {
    sources: Arc<RwLock<HashMap<proto::djtool::Service, Source>>>,
    sinks: Arc<RwLock<HashMap<proto::djtool::Service, Sink>>>,
    transcoder: Arc<Box<dyn transcode::Transcoder + Sync + Send>>,
    data_dir: Option<PathBuf>,
    config: Arc<RwLock<Option<config::Config>>>,
    login_done: Arc<(broadcast::Sender<bool>, broadcast::Receiver<bool>)>,
    host: IpAddr,
    port: u16,
}

// impl<'_> DjTool<'_> {
impl DjTool {
    // host: Option<IpAddr::V4>, port: Option<u16>,
    // watch::Receiver<bool>
    pub async fn serve(
        &self,
        // shutdown_rx: impl Future<Output = ()> + Send + Clone + 'static,
        mut shutdown_tx: broadcast::Sender<bool>,
    ) -> Result<()> {
        let grpc_addr = SocketAddr::new(self.host, 21022);
        let http_addr = SocketAddr::new(self.host, self.port);

        println!("grpc listening at {}", grpc_addr);
        println!("frontend served at {}", http_addr);

        let djtool_grpc_service = proto::djtool::dj_tool_server::DjToolServer::new(self.clone());
        let djtool_grpc_service = tonic_web::config()
            // .allow_origins(vec!["localhost", "127.0.0.1"])
            .enable(djtool_grpc_service);

        let grpc_server = TonicServer::builder()
            .accept_http1(true)
            .max_concurrent_streams(128)
            .add_service(djtool_grpc_service);

        let library_dir = {
            let config = self.config.read().await;
            config.as_ref().map(|c| c.library.library_dir.to_owned())
        };
        let http_tool_clone = self.clone();
        let mut shutdown_rx = shutdown_tx.subscribe();

        let http_server = tokio::spawn(async move {
            let library = warp::path("library").and(warp::fs::dir(library_dir.unwrap()));

            let http_tool = http_tool_clone.clone();
            let spotify_pkce_callback = warp::get()
                .and(warp::path!("spotify" / "pkce" / "callback"))
                .and(warp::query::<spotify::auth::pkce::CallbackQuery>())
                .and(warp::any().map(move || http_tool.clone()))
                .and_then(spotify_pkce_callback_handler);

            #[cfg(feature = "debug")]
            let routes = {
                let http_tool = http_tool_clone.clone();
                let debug_spotify_playlists = warp::get()
                    .and(warp::path!("debug" / "spotify" / "playlists"))
                    .and(warp::query::<debug::DebugSpotifyPlaylistsQuery>())
                    .and(warp::any().map(move || http_tool.clone()))
                    .and_then(debug::debug_spotify_playlists_handler);

                let http_tool = http_tool_clone.clone();
                let debug_youtube_search = warp::get()
                    .and(warp::path!("debug" / "youtube" / "search"))
                    .and(warp::query::<debug::DebugYoutubeSearchQuery>())
                    .and(warp::any().map(move || http_tool.clone()))
                    .and_then(debug::debug_youtube_search_handler);

                spotify_pkce_callback
                    .or(library)
                    .or(debug_youtube_search)
                    .or(debug_spotify_playlists)
            };

            #[cfg(not(feature = "debug"))]
            let routes = spotify_pkce_callback.or(library);

            println!("starting server now ...");
            let (_, server) = warp::serve(routes)
                .try_bind_with_graceful_shutdown(http_addr, async move {
                    shutdown_rx
                        // .clone()
                        // .changed()
                        .recv()
                        .await
                        .expect("failed to shutdown");
                })
                .expect("failed to bind");
            server.await;
        });

        let mut shutdown_rx = shutdown_tx.subscribe();
        grpc_server
            .serve_with_shutdown(grpc_addr, async move {
                shutdown_rx
                    // .subscribe()
                    // .clone()
                    // .changed()
                    .recv()
                    .await
                    .expect("failed to shutdown");
            })
            .await?;
        Ok(())
    }
}

#[tonic::async_trait]
impl proto::djtool::dj_tool_server::DjTool for DjTool {
    type SyncStream = Pin<
        Box<
            dyn Stream<Item = Result<proto::djtool::SyncProgressUpdate, Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn sync(
        &self,
        request: Request<proto::djtool::SyncRequest>,
    ) -> Result<Response<Self::SyncStream>, Status> {
        let (stream_tx, stream_rx) = mpsc::channel(1);
        let pinned_stream = Box::pin(ReceiverStream::new(stream_rx));
        let response: Response<Self::SyncStream> = Response::new(pinned_stream);
        Ok(response)
    }
}

impl Default for DjTool {
    fn default() -> Self {
        let mut sinks: HashMap<proto::djtool::Service, Sink> = HashMap::new();
        // todo: make this a better interface
        sinks.insert(
            proto::djtool::Service::Youtube,
            Arc::new(Box::new(Youtube::new())),
        );
        let transcoder = transcode::FFmpegTranscoder::default();
        Self {
            data_dir: None,
            transcoder: Arc::new(Box::new(transcoder)),
            // sources: Arc::new(RwLock::new(Arc::new(HashMap::new()))),
            // transcoder: Arc::new(RwLock::new(HashMap::new())),
            sources: Arc::new(RwLock::new(HashMap::new())),
            sinks: Arc::new(RwLock::new(sinks)),
            config: Arc::new(RwLock::new(None)),
            login_done: Arc::new(broadcast::channel(16)),
            host: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            port: 21011,
        }
    }
}

impl DjTool {
    pub async fn request_user_login(
        &self,
        auth_url: reqwest::Url,
    ) -> Result<broadcast::Receiver<bool>> {
        // pub async fn request_user_login(&self, auth_url: reqwest::Url) -> Result<()> {
        // todo: figure out how to notify the ui to show the link too if it cannot be opened
        // let (tx, rx) = oneshot::channel();
        // let (tx, rx): (Sender<i32>, Receiver<i32>) = mpsc::channel();
        // let (tx, rx) = mpsc::channel(1);
        webbrowser::open(auth_url.as_str())?;

        let mut done = self.login_done.0.subscribe();
        done.recv().await?;

        Ok(done)
        // Ok(rx.subscribe())
    }

    pub async fn is_connected(&self, source: proto::djtool::Service) -> bool {
        let sources = self.sources.read().await;
        sources.contains_key(&source)
    }

    pub async fn connect_spotify(&self, creds: Credentials, oauth: OAuth) -> Result<()> {
        let spotify_client =
            spotify::Spotify::pkce(&self.data_dir.as_ref().unwrap(), creds, oauth).await?;

        self.sources.write().await.insert(
            proto::djtool::Service::Spotify,
            Arc::new(Box::new(spotify_client)),
        );

        let spotify_client = &self.sources.read().await[&proto::djtool::Service::Spotify];
        match spotify_client.reauthenticate().await {
            Ok(Some(auth_url)) => {
                // blocks until user logs in
                self.request_user_login(auth_url).await?;
            }
            Err(err) => panic!("{}", err),
            _ => {}
        };

        // match spotify_client.authenticator.reauthenticate().await {
        //     Err(spotify::error::Error::Auth(spotify::error::AuthError::RequireUserLogin {
        //         auth_url,
        //     })) => {
        //         // blocks until user logs in
        //         self.request_user_login(auth_url).await?;
        //     }
        //     Err(err) => panic!("{}", err),
        //     Ok(_) => {}
        // };
        // println!("connected with spotify");
        Ok(())
    }

    pub async fn connect_sources(&self) {
        // todo: store the credentials in the spotify config
        let creds = spotify::auth::Credentials::pkce("893474f878934ae89fff417e4722e147");
        let oauth = spotify::auth::OAuth {
            redirect_uri: format!(
                "http://{}:{}/spotify/pkce/callback",
                self.host.to_string(),
                self.port
            ),
            scopes: scopes!("playlist-read-private"),
            ..Default::default()
        };
        println!("redirect url host: {}", oauth.redirect_uri);
        if let Err(err) = self.connect_spotify(creds, oauth).await {
            println!("spotify connect error: {}", err);
        }
    }

    pub fn ephemeral() -> Self {
        Self::default()
    }

    pub async fn persistent(
        data_dir: Option<impl AsRef<Path> + Sync + Send + Clone>,
    ) -> Result<Self> {
        let data_dir = data_dir
            .map(|d| d.as_ref().to_path_buf())
            .or(dirs::home_dir().map(|d| d.join(".djtool")))
            .ok_or(anyhow::anyhow!("no data dir available"))?;

        let config = config::Config::open(&data_dir).await?;
        Ok(Self {
            data_dir: Some(data_dir.to_owned()),
            config: Arc::new(RwLock::new(Some(config))),
            ..Default::default()
        })
    }

    // pub async fn track_stream<'a>(&'a self) -> TrackStream<'a> {
    // pub async fn track_stream<'b>(&'b self) -> Pin<Box<dyn Stream<Item = proto::djtool::Track> + 'b + Send>> {

    pub fn all_playlists_stream<'a>(
        &'a self,
        user_id: &'a String,
        sources: &'a HashMap<proto::djtool::Service, Source>,
        // ) -> Pin<Box<dyn Stream<Item = (&proto::djtool::Service, &Source)> + Send + 'a>> {
        // ) -> Pin<Box<dyn Stream<Item = Result<proto::djtool::Track>> + Send + 'a>> {
    ) -> Pin<Box<dyn Stream<Item = (proto::djtool::Service, proto::djtool::Playlist)> + Send + 'a>>
    {
        // Box::pin(stream::iter(&[1u32, 2u32, 3u32]))
        let sources_stream = stream::iter(sources.into_iter());
        let playlist_stream = sources_stream
            // .filter_map(move |source_id: &proto::djtool::Service| {
            .filter_map(
                move |(service, source): (&proto::djtool::Service, &Source)| {
                    // let sources_clone = sources.clone();
                    // let playlists_failed = playlists_failed.clone();
                    // let source: &Source = &sources_lock[source_id];
                    // let source: &'a Source = &sources_lock[source_id];
                    // let source: &'b Source = &sources_lock[source_id];
                    // let source = Box::new(sources_lock_clone[source_id].clone());
                    // let source = &sources_lock[source_id];
                    // let test = sources_lock.clone();
                    // let source: &Source = &test[source_id];
                    async move {
                        // let sources_lock = self.sources.read().await;
                        // match sources_clone {
                        //     Some(src) => {
                        //         if (src.is_empty() || src.contains(&source_id)) {
                        //             return None;
                        //         };
                        //     }
                        //     None => {
                        //         return None;
                        //     }
                        // };
                        // if !(sources_clone.is_some() || sources_clone.is_empty() || sources_clone.contains(&source_id)) {
                        //     return None;
                        // }
                        let playlist_stream = {
                            // let source = &sources_lock[source_id];
                            // sources_lock[source_id].user_playlists_stream(user_id)
                            source.user_playlists_stream(user_id)
                        };

                        match playlist_stream {
                            Ok(playlist_stream) => Some(playlist_stream),
                            Err(err) => {
                                println!("playlist create stream error: {}", err);
                                // let mut fp = playlists_failed.lock().await;
                                // *fp += 1;
                                None
                            }
                        }
                    }
                },
            )
            .flat_map(|playlist_stream| playlist_stream)
            .filter_map(|playlist: Result<proto::djtool::Playlist>| {
                // let playlists_failed = playlists_failed.clone();
                let source_id = playlist
                    .as_ref()
                    .ok()
                    .and_then(|pl| pl.id.as_ref())
                    .map(|id| id.source)
                    .and_then(proto::djtool::Service::from_i32);

                async move {
                    match playlist {
                        Ok(pl) => source_id.map(|id| (id, pl)),
                        Err(err) => {
                            println!("playlist error: {}", err);
                            // let mut fp = playlists_failed.lock().await;
                            // *fp += 1;
                            None
                        }
                    }
                }
            });
        // .filter_map(
        //     move |(source_id, playlist): (proto::djtool::Service, proto::djtool::Playlist)| {
        //         // let playlists_failed = playlists_failed.clone();
        //         // let playlists_succeeded = playlists_succeeded.clone();
        //         // let source: &'a Source = &sources_lock[&source_id];
        //         // let source: &Source = &sources_lock[&source_id];
        //         // let test = source_id.clone();
        //         // let source = &(sources_lock[&test]);
        //         // let source = &sources_lock[&source_id];
        //         async move {
        //             let sources_lock = self.sources.read().await;
        //             let source = &sources_lock[&source_id];
        //             let tracks_stream = source.user_playlist_tracks_stream(playlist);
        //             match tracks_stream {
        //                 Ok(track_stream) => {
        //                     // let mut f = playlists_succeeded.lock().await;
        //                     // *f += 1;
        //                     Some(track_stream)
        //                 }
        //                 Err(err) => {
        //                     println!("track stream error: {}", err);
        //                     // {
        //                     //     let mut f = playlists_failed.lock().await;
        //                     //     *f += 1;
        //                     // };
        //                     None
        //                 }
        //             }
        //         }
        //     },
        // );
        // let playlist_stream = playlist_stream.take(100);
        // let playlist_stream = playlist_stream.flat_map(|track_stream| track_stream);
        Box::pin(playlist_stream)
    }

    pub fn track_stream<'a>(
        &'a self,
        sources: &'a HashMap<proto::djtool::Service, Source>,
        playlist_stream: Pin<
            Box<dyn Stream<Item = (proto::djtool::Service, proto::djtool::Playlist)> + Send + 'a>,
        >,
        // sources: &'a HashMap<proto::djtool::Service, Source>,
        // ) -> Pin<Box<dyn Stream<Item = (&proto::djtool::Service, &Source)> + Send + 'a>> {
        // ) -> Pin<Box<dyn Stream<Item = Result<proto::djtool::Track>> + Send + 'a>> {
    ) -> Pin<Box<dyn Stream<Item = (proto::djtool::Track)> + Send + 'a>> {
        let track_stream = playlist_stream
            .filter_map(
                move |(source_id, playlist): (proto::djtool::Service, proto::djtool::Playlist)| {
                    // let playlists_failed = playlists_failed.clone();
                    // let playlists_succeeded = playlists_succeeded.clone();
                    // let source: &'a Source = &sources_lock[&source_id];
                    // let source: &Source = &sources_lock[&source_id];
                    // let test = source_id.clone();
                    // let source = &(sources_lock[&test]);
                    // let source = &sources_lock[&source_id];
                    async move {
                        // let source = &sources_lock[&source_id];
                        let source = &sources[&source_id];
                        let tracks_stream = source.user_playlist_tracks_stream(playlist);
                        match tracks_stream {
                            Ok(track_stream) => {
                                // let mut f = playlists_succeeded.lock().await;
                                // *f += 1;
                                Some(track_stream)
                            }
                            Err(err) => {
                                println!("track stream error: {}", err);
                                // {
                                //     let mut f = playlists_failed.lock().await;
                                //     *f += 1;
                                // };
                                None
                            }
                        }
                        // None
                    }
                },
            )
            // let track_stream = track_stream .take(100);
            .flat_map(|track_stream| track_stream)
            // let track
            .filter_map(|track: Result<proto::djtool::Track>| {
                // let tracks_failed = tracks_failed.clone();
                async move {
                    match track {
                        Ok(track) => Some(track),
                        Err(err) => {
                            println!("track error: {}", err);
                            // {
                            //     let mut fp = tracks_failed.lock().await;
                            //     *fp += 1;
                            // };
                            None
                        }
                    }
                }
            });
        Box::pin(track_stream)
    }

    // pub async fn track_stream(
    //     &self,
    //     sources: Option<HashSet<proto::djtool::Service>>,
    //     playlists: Option<HashMap<proto::djtool::Service, String>>,
    // ) -> Pin<Box<dyn Stream<Item = proto::djtool::Track> + Send + '_>> {
    //     // let sources = Arc::new(HashSet::from([proto::djtool::Service::Spotify]));
    //     // let playlists = Arc::new(HashSet::from([(
    //     //     proto::djtool::Service::Soundcloud,
    //     //     String::from("test"),
    //     // )]));

    //     // let sources_lock = self.sources.read().await;
    //     // let sources_lock = Arc::new(self.sources.read().await);
    //     // let sources_lock = &(self.sources.read().await);
    //     // let sinks_lock = self.sinks.read().await;
    //     // println!("locked sources and sinks");

    //     // let sources_lock_clone: HashMap<_,_> = sources_lock.clone();
    //     // let sources_clone = sources.clone();
    //     // let sources_lock_clone = sources_lock.clone();
    //     // let test = ;
    //     // let test = Arc::new(self.sources.read().await.keys());
    //     let test = self.sources.read().await;
    //     // .keys();
    //     // let sources_stream = stream::iter(HashSet::from([proto::djtool::Service::Spotify]).keys());
    //     let sources_stream = stream::iter(test.keys());
    //     // let mut sources_stream = tokio_stream::iter(test.keys());
    //     // tokio::pin!(sources_stream);

    //     let playlist_stream = sources_stream
    //         .filter_map(move |source_id: &proto::djtool::Service| {
    //             let sources_clone = sources.clone();
    //             // let playlists_failed = playlists_failed.clone();
    //             // let source: &Source = &sources_lock[source_id];
    //             // let source: &'a Source = &sources_lock[source_id];
    //             // let source: &'b Source = &sources_lock[source_id];
    //             // let source = Box::new(sources_lock_clone[source_id].clone());
    //             // let source = &sources_lock[source_id];
    //             // let test = sources_lock.clone();
    //             // let source: &Source = &test[source_id];
    //             async move {
    //                 let sources_lock = self.sources.read().await;
    //                 match sources_clone {
    //                     Some(src) => {
    //                         if (src.is_empty() || src.contains(&source_id)) {
    //                             return None;
    //                         };
    //                     }
    //                     None => {
    //                         return None;
    //                     }
    //                 };
    //                 // if !(sources_clone.is_some() || sources_clone.is_empty() || sources_clone.contains(&source_id)) {
    //                 //     return None;
    //                 // }
    //                 let user_id = env::var("SPOTIFY_USER_ID").unwrap();
    //                 let playlist_stream = {
    //                     // let source = &sources_lock[source_id];
    //                     sources_lock[source_id].user_playlists_stream(user_id)
    //                 };

    //                 match playlist_stream {
    //                     Ok(playlist_stream) => Some(playlist_stream),
    //                     Err(err) => {
    //                         println!("playlist create stream error: {}", err);
    //                         // let mut fp = playlists_failed.lock().await;
    //                         // *fp += 1;
    //                         None
    //                     }
    //                 }
    //             }
    //         })
    //         .flat_map(|playlist_stream| playlist_stream)
    //         .filter_map(|playlist: Result<proto::djtool::Playlist>| {
    //             // let playlists_failed = playlists_failed.clone();
    //             let source_id = playlist
    //                 .as_ref()
    //                 .ok()
    //                 .and_then(|pl| pl.id.as_ref())
    //                 .map(|id| id.source)
    //                 .and_then(proto::djtool::Service::from_i32);

    //             async move {
    //                 match playlist {
    //                     Ok(pl) => source_id.map(|id| (id, pl)),
    //                     Err(err) => {
    //                         println!("playlist error: {}", err);
    //                         // let mut fp = playlists_failed.lock().await;
    //                         // *fp += 1;
    //                         None
    //                     }
    //                 }
    //             }
    //         })
    //         .filter_map(
    //             move |(source_id, playlist): (proto::djtool::Service, proto::djtool::Playlist)| {
    //                 // let playlists_failed = playlists_failed.clone();
    //                 // let playlists_succeeded = playlists_succeeded.clone();
    //                 // let source: &'a Source = &sources_lock[&source_id];
    //                 // let source: &Source = &sources_lock[&source_id];
    //                 // let test = source_id.clone();
    //                 // let source = &(sources_lock[&test]);
    //                 // let source = &sources_lock[&source_id];
    //                 async move {
    //                     let sources_lock = self.sources.read().await;
    //                     let source = &sources_lock[&source_id];
    //                     let tracks_stream = source.user_playlist_tracks_stream(playlist);
    //                     match tracks_stream {
    //                         Ok(track_stream) => {
    //                             // let mut f = playlists_succeeded.lock().await;
    //                             // *f += 1;
    //                             Some(track_stream)
    //                         }
    //                         Err(err) => {
    //                             println!("track stream error: {}", err);
    //                             // {
    //                             //     let mut f = playlists_failed.lock().await;
    //                             //     *f += 1;
    //                             // };
    //                             None
    //                         }
    //                     }
    //                 }
    //             },
    //         );
    //     let playlist_stream = playlist_stream.take(100);
    //     let track_stream = playlist_stream
    //         .flat_map(|track_stream| track_stream)
    //         .filter_map(|track: Result<proto::djtool::Track>| {
    //             // let tracks_failed = tracks_failed.clone();
    //             async move {
    //                 match track {
    //                     Ok(track) => Some(track),
    //                     Err(err) => {
    //                         println!("track error: {}", err);
    //                         // {
    //                         //     let mut fp = tracks_failed.lock().await;
    //                         //     *fp += 1;
    //                         // };
    //                         None
    //                     }
    //                 }
    //             }
    //         });
    //     let track_stream = track_stream.take(100);
    //     // Ok(Box::pin(tracks))
    //     Box::pin(track_stream)
    //     // track_stream
    // }

    pub async fn sync_library(&self) -> Result<()> {
        println!("starting sync");

        // let sources = Arc::new(HashSet::from([proto::djtool::Service::Spotify]));
        // let playlists = Arc::new(HashSet::from([(
        //     proto::djtool::Service::Soundcloud,
        //     String::from("test"),
        // )]));

        // create lots of nested hash maps here that will store the final data
        // let playlists_failed = Arc::new(Mutex::new(0u64));
        // let playlists_succeeded = Arc::new(Mutex::new(0u64));

        // let tracks_failed = Arc::new(Mutex::new(0u64));
        // let tracks_succeeded = Arc::new(Mutex::new(0u64));
        // let tracks_in_progress = Arc::new(Mutex::new(0u64));

        let user_id = env::var("SPOTIFY_USER_ID").unwrap();
        let sources_locked = self.sources.read().await;
        // let track_stream = self.track_stream(None, None).await;
        let playlist_stream = self.all_playlists_stream(
            &user_id,
            // sources: &'a HashMap<proto::djtool::Service, Source>,
            // &self.sources.clone(),
            // Arc::as_ptr(&self.sources.clone()),
            // self.sources.clone().deref().deref(),
            &sources_locked.deref(),
        );
        let target_playlists = vec![proto::djtool::PlaylistId {
            id: "6pxZ9k6K50R1Q1R3ptcy3T".to_string(),
            source: proto::djtool::Service::Spotify as i32,
        }];
        // let playlist_stream = stream::iter(target_playlists)
        //     .buffer_unordered(5)
        //     .map(
        //     self.playlists_stream(
        //             );
        // .flat_map(|track_stream| track_stream)
        //
        // let sources_lock = self.sources.read().await;
        let track_stream = self.track_stream(&sources_locked.deref(), playlist_stream);
        // let track_stream = playlist_stream
        //     .filter_map(
        //         move |(source_id, playlist): (proto::djtool::Service, proto::djtool::Playlist)| {
        //             // let playlists_failed = playlists_failed.clone();
        //             // let playlists_succeeded = playlists_succeeded.clone();
        //             // let source: &'a Source = &sources_lock[&source_id];
        //             // let source: &Source = &sources_lock[&source_id];
        //             // let test = source_id.clone();
        //             // let source = &(sources_lock[&test]);
        //             // let source = &sources_lock[&source_id];
        //             async move {
        //                 // let source = &sources_lock[&source_id];
        //                 // let source = &sources_locked[&source_id];
        //                 // let tracks_stream = source.user_playlist_tracks_stream(playlist);
        //                 // match tracks_stream {
        //                 //     Ok(track_stream) => {
        //                 //         // let mut f = playlists_succeeded.lock().await;
        //                 //         // *f += 1;
        //                 //         Some(track_stream)
        //                 //     }
        //                 //     Err(err) => {
        //                 //         println!("track stream error: {}", err);
        //                 //         // {
        //                 //         //     let mut f = playlists_failed.lock().await;
        //                 //         //     *f += 1;
        //                 //         // };
        //                 //         None
        //                 //     }
        //                 // }
        //                 None
        //             }
        //         },
        //     )
        //     // let track_stream = track_stream .take(100);
        //     .flat_map(|track_stream| track_stream)
        //     // let track
        //     .filter_map(|track: Result<proto::djtool::Track>| {
        //         // let tracks_failed = tracks_failed.clone();
        //         async move {
        //             match track {
        //                 Ok(track) => Some(track),
        //                 Err(err) => {
        //                     println!("track error: {}", err);
        //                     // {
        //                     //     let mut fp = tracks_failed.lock().await;
        //                     //     *fp += 1;
        //                     // };
        //                     None
        //                 }
        //             }
        //         }
        //     });
        let track_stream = track_stream.take(100);

        let sources_lock = self.sources.read().await;
        let sinks_lock = self.sinks.read().await;
        println!("locked sources and sinks");

        let process_track = track_stream
            .for_each_concurrent(Some(8), |track: proto::djtool::Track| {
                let sinks_lock = sinks_lock.clone();
                let transcoder = self.transcoder.clone();
                // let tracks_succeeded = tracks_succeeded.clone();
                // let tracks_in_progress = tracks_in_progress.clone();
                // let tracks_failed = tracks_failed.clone();
                async move {
                    let title = track.name.to_owned();
                    let artist = track.artist.to_owned();
                    let filename = format!("{} - {}", title, artist);
                    let filename_clone = filename.clone();

                    // let tracks_succeeded_clone = tracks_succeeded.clone();
                    // let tracks_in_progress_clone = tracks_in_progress.clone();
                    // let tracks_failed_clone = tracks_failed.clone();

                    let res = async move {
                        // {
                        //     let mut p = tracks_in_progress_clone.lock().await;
                        //     *p += 1;
                        // };
                        // println!("{}", filename);

                        let filename = utils::sanitize_filename(&filename);
                        let temp_dir = TempDir::new(&filename)?;
                        // todo: load the preferred sink for the video if available, otherwise use

                        // youtube by default
                        let sink = &sinks_lock[&proto::djtool::Service::Youtube];
                        let downloaded = sink
                            .download(
                                &track,
                                &temp_dir.path().to_path_buf().join("audio"),
                                None,
                                // None,
                                Some(Box::new(|progress: download::DownloadProgress| {})),
                            )
                            .await?;
                        // println!("downloaded to {}", downloaded.output_path.display());

                        // transcode
                        let library_dir = {
                            let config = self.config.read().await;
                            config.as_ref().map(|c| c.library.library_dir.to_owned())
                        }
                        .ok_or(anyhow::anyhow!("no library"))?;

                        let temp_dir_transcode = TempDir::new(&filename)?;
                        let mut transcoded_path = temp_dir_transcode.path().join(&filename);
                        transcoded_path.set_extension("mp3");
                        let mut output_path = library_dir.join(&filename);
                        output_path.set_extension("mp3");

                        // println!("transcoding to {}", transcoded_path.display());
                        let transcoded_path_clone = transcoded_path.to_owned();
                        let res = tokio::task::spawn_blocking(move || {
                            transcoder.transcode_blocking(
                                &downloaded.output_path,
                                &transcoded_path_clone,
                                Some(&transcode::TranscoderOptions::mp3()),
                                &mut Box::new(|progress: transcode::TranscodeProgress| {
                                    println!("{}", progress.frame);
                                }),
                            );
                            Ok::<(), anyhow::Error>(())
                        })
                        .await?;

                        // download the artwork
                        if let Some(artwork) = track.artwork {
                            let artwork_path = async {
                                let dest = temp_dir.path().join("artwork.jpg");
                                let mut download =
                                    download::Download::new(&artwork.url, &dest).await?;
                                download.start().await?;
                                Ok::<PathBuf, anyhow::Error>(dest)
                            }
                            .await;
                            let transcoded_path_clone = transcoded_path.clone();
                            let test = tokio::task::spawn_blocking(move || {
                                artwork_path.and_then(|path| {
                                    id3::embed_image(&transcoded_path_clone, &path)
                                })
                                // match artwork_path {
                                //     Ok(path) => {
                                //     },
                                //     Err(err) => println!("artwork download error: {:?}", err)
                                // }
                                //
                            })
                            .await;
                            println!("artwork: {:?}", test);
                        }

                        std::fs::rename(&transcoded_path, &output_path)?;

                        // move result
                        // transcoded_path.rename(output_path)?;

                        // {
                        //     let mut s = tracks_succeeded.lock().await;
                        //     *s += 1;
                        //     let mut p = tracks_in_progress.lock().await;
                        //     *p -= 1;
                        // };
                        Ok::<(), anyhow::Error>(())
                    }
                    .await;

                    match res {
                        Ok(_) => {
                            // let mut s = tracks_succeeded.lock().await;
                            // *s += 1;
                        }
                        Err(_) => {
                            // {
                            //     let mut s = tracks_failed.lock().await;
                            //     *s += 1;
                            // }
                        }
                    };
                    // {
                    //     let mut p = tracks_in_progress.lock().await;
                    //     *p -= 1;
                    // }
                    println!("track: {} result: {:?}", filename_clone, res);
                }
            })
            .await;

        // println!("playlists failed: {:?}", playlists_failed.lock().await);
        // println!(
        //     "playlists succeeded: {:?}",
        //     playlists_succeeded.lock().await
        // );
        // println!("tracks failed: {:?}", tracks_failed.lock().await);
        // println!("tracks succeeded: {:?}", tracks_succeeded.lock().await);
        println!("sync completed");
        Ok(())

        // stream!(
        // let audio = self
        //     .downloader
        //     // .download_audio(video_id, temp_dir.path().to_path_buf())
        //     .download_audio(
        //         video_id,
        //         // PathBuf::from("/home/roman/dev/djtool/Touchpad.webm"),
        //         &downloaded_audio,
        //         // temp_dir.join(video_id),
        //         // PathBuf::from(format!("/home/roman/dev/djtool/Touchpad.webm"),
        //     )
        //     .await?;

        // transcode to MP3
        // println!("temp dir: {}", temp_dir.path().display());
        // println!("audio output: {}", audio.audio_file.display());
        // let input_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.webm");
        // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.mp3");
        // // let output_file = PathBuf::from("/Users/roman/dev/djtool/Touchpad.mp3");
        // let transcoder = self.transcoder.clone();
        // let res = tokio::task::spawn_blocking(move || {
        //     transcoder.transcode_blocking(
        //         // &audio.audio_file,
        //         &input_file,
        //         &output_file,
        //         None,
        //         &Box::new(|progress: transcode::TranscodeProgress| {
        //             println!("{}", progress.frame);
        //         }),
        //     );
        //     // let mut transcoder = Transcoder::new(audio.audio_file, output_file).unwrap();
        //     // transcoder.start().unwrap();
        // })
        // .await?;
        // Ok(())
    }
}

pub async fn spotify_pkce_callback_handler(
    query: spotify::auth::pkce::CallbackQuery,
    tool: DjTool,
) -> std::result::Result<impl Reply, Infallible> {
    let redirect_url = match query.code {
        Some(code) => {
            let sources = tool.sources.read().await;
            let spotify = &sources[&proto::djtool::Service::Spotify];
            let spotify_login_callback = proto::djtool::SpotifyUserLoginCallback {
                method: Some(proto::djtool::spotify_user_login_callback::Method::Pkce(
                    proto::djtool::SpotifyUserLoginCallbackPkce {
                        code,
                        state: query.state.unwrap_or(String::new()),
                    },
                )),
            };

            spotify
                .handle_user_login_callback(proto::djtool::UserLoginCallback {
                    login: Some(proto::djtool::user_login_callback::Login::SpotifyLogin(
                        spotify_login_callback,
                    )),
                })
                .await
                .unwrap();
            reqwest::Url::parse("https://spotify.com/").unwrap()
        }
        None => {
            let mut params: HashMap<&str, String> = HashMap::new();
            params.insert("error", query.error.unwrap_or(String::new()));
            reqwest::Url::parse_with_params("https://google.com", params).unwrap()
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
    let _ = tool.login_done.0.send(true);
    Ok(warp::reply::html(body))
}
