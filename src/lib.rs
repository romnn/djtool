use tokio;
pub mod config;
#[cfg(feature = "debug")]
mod debug;
pub mod download;
pub mod ffmpeg;
mod ffmpeg_sys;
pub mod id3;
pub mod proto;
pub mod sink;
pub mod source;
pub mod spotify;
pub mod transcode;
pub mod utils;
pub mod youtube;

use anyhow::Result;
use config::Persist;
use dirs;
use futures::Stream;
use futures_util::pin_mut;
use futures_util::stream;
use futures_util::{StreamExt, TryStreamExt};
use reqwest;
use serde::Deserialize;
use spotify::auth::{Credentials, OAuth};
use spotify::model::{Id, PlaylistId, UserId};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::thread;
use tauri::Event;
use tempdir::TempDir;
use tokio::sync::{mpsc, watch, Mutex, RwLock, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server as TonicServer, Code, Request, Response, Status};
use warp::{Filter, Rejection, Reply};
use youtube::Youtube;

const DEFAULT_PORT: u16 = 21011;

pub type SpotifyClient = Arc<spotify::Spotify>;
pub type YoutubeClient = Arc<youtube::Youtube>;

pub type Source = Arc<Box<dyn source::Source + Send + Sync>>;
pub type Sink = Arc<Box<dyn sink::Sink + Send + Sync>>;

#[derive(Clone)]
pub struct DjTool {
    // backends: Youtube,
    sources: Arc<RwLock<HashMap<proto::djtool::Service, Source>>>,
    sinks: Arc<RwLock<HashMap<proto::djtool::Service, Sink>>>,
    transcoder: Arc<Box<dyn transcode::Transcoder + Sync + Send>>,
    data_dir: Option<PathBuf>,
    config: Arc<RwLock<Option<config::Config>>>,
    // spotify: SpotifyClient,
    connection: Option<String>,

    // semaphores to limit concurrency
    request_spotify_download: Arc<Semaphore>,
    // pub shutdown_rx: watch::Receiver<bool>,
    // pub sessions: Arc<RwLock<HashMap<proto::grpc::SessionToken, Session<VU, CU>>>>,
    // connections: Arc<RwLock<mpsc::Sender<Result<VU, Status>>>>,
}

// fn enable(is_enabled: bool) -> impl Filter<Extract = (), Error = Rejection> + Copy {
//     warp::any()
//         .and_then(move || {
//             if is_enabled {
//                 Ok(())
//             } else {
//                 // or warp::reject::custom if something besides 404
//                 Err(warp::reject::not_found())
//             }
//         })
//         // this weirdo deals with the Ok(())
//         .untuple_one()
// }

impl DjTool {
    // fn with_spotify(
    //     spotify: SpotifyClient,
    // ) -> impl Filter<Extract = (SpotifyClient,), Error = Infallible> + Send + Clone {
    //     warp::any().map(move || spotify.clone())
    // }

    // fn with_youtube(
    //     youtube: YoutubeClient,
    // ) -> impl Filter<Extract = (YoutubeClient,), Error = Infallible> + Send + Clone {
    //     warp::any().map(move || youtube.clone())
    // }

    pub async fn serve(&self, shutdown_rx: watch::Receiver<bool>) -> Result<()> {
        let host = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let grpc_addr = SocketAddr::new(host, 21022);
        let http_addr = SocketAddr::new(host, 21011);

        println!("grpc listening at {}", grpc_addr);
        println!("frontend served at {}", http_addr);

        // let djtool_grpc_service = proto::djtool::dj_tool_server::DjToolServer::new(self.clone());
        // let test = Arc::new(self);
        let djtool_grpc_service = proto::djtool::dj_tool_server::DjToolServer::new(self.clone());
        let djtool_grpc_service = tonic_web::config()
            // .allow_origins(vec!["localhost", "127.0.0.1"])
            .enable(djtool_grpc_service);

        let grpc_server = TonicServer::builder()
            .accept_http1(true)
            .max_concurrent_streams(128)
            .add_service(djtool_grpc_service);

        // tokio::task::spawn(async move {
        //     let web = warp::get().and(warp::fs::dir("../../www/build"));
        //     warp::serve(web).run(static_addr).await;
        // });
        let library_dir = {
            let config = self.config.read().await;
            config.as_ref().map(|c| c.library.library_dir.to_owned())
        };
        let http_tool = self.clone();
        let shutdown_clone = shutdown_rx.clone();
        let http_server = tokio::spawn(async move {
            // config.library.library_dir
            let library = warp::path("library").and(warp::fs::dir(library_dir.unwrap()));

            let spotify_pkce_callback = warp::get()
                .and(warp::path!("spotify" / "pkce" / "callback"))
                .and(warp::query::<spotify::auth::pkce::CallbackQuery>())
                .and(warp::any().map(move || http_tool.clone()))
                // .and(with_spotify(spotify_client.clone()))
                .and_then(spotify_pkce_callback_handler);

            // #[cfg(feature = "debug")]
            // let routes = {
            //     let debug_spotify_playlists = warp::get()
            //         .and(warp::path!("debug" / "spotify" / "playlists"))
            //         .and(warp::query::<debug::DebugSpotifyPlaylistsQuery>())
            //         .and(with_spotify(spotify_client.clone()))
            //         .and_then(debug::debug_spotify_playlists_handler);

            //     let youtube = Arc::new(Youtube::new().unwrap());
            //     let debug_youtube_search = warp::get()
            //         .and(warp::path!("debug" / "youtube" / "search"))
            //         .and(warp::query::<debug::DebugYoutubeSearchQuery>())
            //         .and(with_youtube(youtube.clone()))
            //         .and_then(debug::debug_youtube_search_handler);

            //     spotify_pkce_callback
            //         .or(library)
            //         .or(debug_youtube_search)
            //         .or(debug_spotify_playlists)
            // };

            // #[cfg(not(feature = "debug"))]
            let routes = spotify_pkce_callback.or(library);

            println!("starting server now ...");
            let (_, server) = warp::serve(routes)
                .try_bind_with_graceful_shutdown(http_addr, async move {
                    // shutdown_rx
                    shutdown_clone
                        .clone()
                        .changed()
                        .await
                        .expect("failed to shutdown");
                })
                .expect("failed to bind");
            server.await;
            // tokio::run(server);
            // .try_bind_with_graceful_shutdown(([127, 0, 0, 1], DEFAULT_PORT), )
            // .run(([0, 0, 0, 0], DEFAULT_PORT))
            // .run(http_addr)
            // .await;
        });

        grpc_server
            .serve_with_shutdown(grpc_addr, async {
                shutdown_rx
                    .clone()
                    .changed()
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

fn request_user_login(auth_url: reqwest::Url) -> Result<()> {
    // todo: figure out how to notify the ui to show the link too if it cannot be opened
    webbrowser::open(auth_url.as_str())?;
    Ok(())
}

impl Default for DjTool {
    fn default() -> Self {
        Self {
            data_dir: None,
            transcoder: Arc::new(Box::new(transcode::FFmpegTranscoder::default())),
            sources: Arc::new(RwLock::new(HashMap::new())),
            sinks: Arc::new(RwLock::new(HashMap::new())),
            connection: None,
            config: Arc::new(RwLock::new(None)),
            request_spotify_download: Arc::new(Semaphore::new(10)),
        }
    }
}

impl DjTool {
    pub async fn is_connected(&self, source: proto::djtool::Service) -> bool {
        let sources = self.sources.read().await;
        sources.contains_key(&source)
    }

    pub async fn connect_spotify(&self, creds: Credentials, oauth: OAuth) -> Result<()> {
        // check if already connected
        // if self.has_source(proto::djtool::Source::Spotify) {
        //     return Err(anyhow!("spotify already connected"));
        // }
        // todo: fix unwrap
        let spotify_client =
            spotify::Spotify::pkce(&self.data_dir.as_ref().unwrap(), creds, oauth).await?;
        spotify_client
            .authenticator
            .reauthenticate()
            .await
            .or_else(|err| match err {
                spotify::error::Error::Auth(spotify::error::AuthError::RequireUserLogin {
                    auth_url,
                }) => request_user_login(auth_url),
                err => Err(err.into()),
            })?;

        // silently reconnect
        let mut sources = self.sources.write().await;
        sources.insert(
            proto::djtool::Service::Spotify,
            Arc::new(Box::new(spotify_client)),
        );
        println!("connected with spotify");
        Ok(())
    }

    pub async fn connect_sources(&self) {
        // todo: store the credentials in the spotify config
        let creds = spotify::auth::Credentials::pkce("893474f878934ae89fff417e4722e147");
        let oauth = spotify::auth::OAuth {
            redirect_uri: format!("http://localhost:{}/spotify/pkce/callback", DEFAULT_PORT),
            scopes: scopes!("playlist-read-private"),
            ..Default::default()
        };
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
        // let backends = Youtube::new()?; // config.debug_dir())?;
        // let config = config::Config::open(&data_dir).await.unwrap();
        Ok(Self {
            data_dir: Some(data_dir.to_owned()),
            config: Arc::new(RwLock::new(Some(config))),
            ..Default::default()
        })
    }

    pub async fn sync_library(&self) -> Result<()> {
        println!("starting sync");
        let temp_dir = TempDir::new("djtool")?;
        // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.aiff");
        let downloaded_audio = temp_dir.path().join(&"test");

        let sources = Arc::new(HashSet::from([proto::djtool::Service::Spotify]));
        // let playlists = Arc::new(HashSet::new::<(proto::djtool::Service, String)>());
        let playlists = Arc::new(HashSet::from([(
            proto::djtool::Service::Soundcloud,
            String::from("test"),
        )]));

        // create lots of nested hash maps here that will store the final data
        // use rwlocks
        // we could also allow concurrent access?
        //
        let playlists_failed = Arc::new(Mutex::new(0u64));
        let playlists_succeeded = Arc::new(Mutex::new(0u64));

        let tracks_failed = Arc::new(Mutex::new(0u64));
        let tracks_succeeded = Arc::new(Mutex::new(0u64));
        let tracks_in_progress = Arc::new(Mutex::new(0u64));

        let sources_lock = self.sources.read().await;
        println!("got sources lock");

        // let stream = stream::iter(self.sources.read().await.iter())
        let track_stream = stream::iter(sources_lock.iter())
            // .filter(|(source_id, source): &(proto::djtool::Source, Source)| {
            .filter_map(|(source_id, source): (&proto::djtool::Service, &Source)| {
                // test
                let sources_clone = sources.clone();
                async move {
                    if sources_clone.is_empty() || sources_clone.contains(&source_id) {
                        Some(source_id)
                    } else {
                        None
                    }
                }
            })
            // .filter_map(|(source_id, source): (&proto::djtool::Source, &Source)| {
            .filter_map(|source_id: &proto::djtool::Service| {
                let playlists_failed = playlists_failed.clone();
                // let source_clone = source.clone();
                // let playlist_stream =
                //     playlist_stream.map(move |playlist| (source_clone.clone(), playlist));
                // let playlist_stream = playlist_stream
                //     .map(|s| Box::pin(s.map(|playlist| (source_clone.clone(), playlist))));

                // let source: &Source = &sources_lock[source_id];
                let source: &Source = &sources_lock[source_id];
                async move {
                    // let sources = &self.sources.read().await;
                    let playlist_stream = source.user_playlists_stream("todo");
                    match playlist_stream {
                        Ok(playlist_stream) => Some(playlist_stream),
                        Err(err) => {
                            println!("playlist create stream error: {}", err);
                            let mut fp = playlists_failed.lock().await;
                            *fp += 1;
                            None
                        }
                    }
                }
            })
            // flatten
            // .flat_map(|track_stream| track_stream)
            .flat_map(|playlist_stream| playlist_stream)
            // filter out Err(playlist_track_stream)
            .filter_map(
                // |(source, playlist): (Source, Result<proto::djtool::Playlist>)| {
                |(playlist): (Result<proto::djtool::Playlist>)| {
                    // test
                    let playlists_failed = playlists_failed.clone();
                    let source_id = playlist
                        .as_ref()
                        .ok()
                        .and_then(|pl| pl.id.as_ref())
                        .map(|id| id.source)
                        .and_then(proto::djtool::Service::from_i32);

                    async move {
                        // match (source_id, playlist) {
                        match playlist {
                            // (Some(source_id), Ok(pl)) => Some((source_id, pl)),
                            // Ok(pl) => Some(pl),
                            Ok(pl) => {
                                source_id.map(|id| (id, pl))
                                // match source_id {
                                //     Some(source_id
                                // }
                                //     Some
                                //     // match pl {
                                //     //     proto::djtool::Playlist {
                                //     //         id: Some(proto::djtool::PlaylistId{
                                //     //             source, ..
                                //     //         }),
                                //     //         ..
                                //     //     }
                            }
                            Err(err) => {
                                println!("playlist error: {}", err);
                                let mut fp = playlists_failed.lock().await;
                                *fp += 1;
                                None
                            }
                        }
                    }
                },
            )
            // // // .flat_map(|playlist_stream| playlist_stream)
            // // filter playlists
            // .filter_map(
            //     |(source, playlist): (Source, proto::djtool::Playlist)| async {
            //         match playlist.id.as_ref().map(|id| id.id.clone()) {
            //             Some(id) => {
            //                 let key = (source.id(), id);
            //                 if playlists.is_empty() || playlists.contains(&key) {
            //                     Some((source, playlist))
            //                 } else {
            //                     None
            //                 }
            //             }
            //             None => None,
            //         }
            //     },
            // )
            // .filter_map(|(source, playlist): (Source, proto::djtool::Playlist)| {
            .filter_map(
                |(source_id, playlist): (proto::djtool::Service, proto::djtool::Playlist)| {
                    // .filter_map(|(playlist): (proto::djtool::Playlist)| {
                    let playlists_failed = playlists_failed.clone();
                    let playlists_succeeded = playlists_succeeded.clone();
                    let source: &Source = &sources_lock[&source_id];
                    // let source = source.clone();
                    // // let tracks_stream = source.clone().user_playlist_tracks_stream(playlist_id);
                    // let source_clone = source.clone();
                    // let tracks_stream = ;
                    // let playlist_clone = playlist.clone();
                    async move {
                        // let tracks_stream = tracks_stream.map(|s| {
                        //     //     let source_clone = source.clone();
                        //     //     s.map(move |track| (source_clone.clone(), playlist_clone.clone(), track))
                        //     s
                        // });
                        // let playlist_id = playlist.id.as_ref().unwrap().id.to_owned();
                        let tracks_stream = source.user_playlist_tracks_stream(playlist);
                        match tracks_stream {
                            Ok(track_stream) => {
                                let mut f = playlists_succeeded.lock().await;
                                *f += 1;
                                Some(track_stream)
                            }
                            Err(err) => {
                                println!("track stream error: {}", err);
                                let mut f = playlists_failed.lock().await;
                                *f += 1;
                                None
                            }
                        }
                    }
                },
            )
            .flat_map(|track_stream| track_stream)
            // filter out Err(Track)
            .filter_map(
                // |(source, playlist): (Source, Result<proto::djtool::Playlist>)| {
                |(track): (Result<proto::djtool::Track>)| {
                    let tracks_failed = tracks_failed.clone();
                    async move {
                        match track {
                            Ok(track) => Some(track),
                            Err(err) => {
                                println!("track error: {}", err);
                                let mut fp = tracks_failed.lock().await;
                                *fp += 1;
                                None
                            }
                        }
                    }
                },
            );
        // .map(|(source, playlist, track)| (source.id(), playlist, track))
        // .collect::<Vec<(
        //     proto::djtool::Source,
        //     proto::djtool::Playlist,
        //     Result<proto::djtool::Track>,
        // )>>()
        // .collect::<Vec<(proto::djtool::Source, Result<proto::djtool::Playlist>)>>()
        // .collect::<Vec<Result<proto::djtool::Track>>>()
        // .collect::<Vec<(Source, Result<proto::djtool::Playlist>)>>()
        // .collect::<Vec<Result<(proto::djtool::Source, proto::djtool::Playlist)>>>()
        // .collect::<Vec<Result<proto::djtool::Playlist>>>()
        // lol
        // .collect::<Vec<proto::djtool::Track>>()
        // .await;
        //
        let process_track = track_stream
            .for_each_concurrent(Some(1), |track: proto::djtool::Track| async move {
                println!("{:?}", track);
            })
            // .then(|track: proto::djtool::Track| async move {
            //     async move {
            //         println!("{:?}", track);
            //     }
            // })
            // .buffer_unordered(100)
            // // .collect::<Vec<proto::djtool::Track>>()
            // .collect::<Vec<()>>()
            .await;

        println!("playlists failed: {:?}", playlists_failed.lock().await);
        println!(
            "playlists succeeded: {:?}",
            playlists_succeeded.lock().await
        );
        println!("tracks failed: {:?}", tracks_failed.lock().await);
        println!("tracks succeeded: {:?}", tracks_succeeded.lock().await);
        println!("sync completed");
        return Ok(());
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
        let input_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.webm");
        let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.mp3");
        // let output_file = PathBuf::from("/Users/roman/dev/djtool/Touchpad.mp3");
        let transcoder = self.transcoder.clone();
        let res = tokio::task::spawn_blocking(move || {
            transcoder.transcode_blocking(
                // &audio.audio_file,
                &input_file,
                &output_file,
                None,
                &Box::new(|progress: transcode::TranscodeProgress| {
                    println!("{}", progress.frame_count);
                }),
            );
            // let mut transcoder = Transcoder::new(audio.audio_file, output_file).unwrap();
            // transcoder.start().unwrap();
        })
        .await?;
        Ok(())
    }
}

pub async fn spotify_pkce_callback_handler(
    query: spotify::auth::pkce::CallbackQuery,
    // sp: SpotifyClient,
    tool: DjTool,
) -> std::result::Result<impl Reply, Infallible> {
    let redirect_url = match query.code {
        Some(code) => {
            // sp.authenticator
            //     .handle_user_login_callback(spotify::auth::SpotifyLoginCallback::Pkce {
            //         code,
            //         state: query.state.unwrap_or(String::new()),
            //     })
            //     .await
            //     .unwrap();
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
    Ok(warp::reply::html(body))
}

// #[tokio::main]
// async fn main_async() {
//     let debug_dir = dirs::home_dir().unwrap().join(".djtool").join("debug");
//     let mock = Youtube::new(debug_dir).unwrap();
//     let results = mock.search("Touchpad Two Shell".to_string()).await.unwrap();
//     println!("search results: {:?}", results);
// }
