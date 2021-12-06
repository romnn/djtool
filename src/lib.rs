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
use futures_util::{StreamExt, TryStreamExt};
use reqwest;
use serde::Deserialize;
use spotify::auth::{Credentials, OAuth};
use spotify::model::{Id, PlaylistId, UserId};
use std::collections::HashMap;
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

#[derive(Clone)]
pub struct DjTool {
    // backends: Youtube,
    sources: Arc<RwLock<HashMap<proto::djtool::Source, Box<dyn source::Source + Send + Sync>>>>,
    sinks: Arc<RwLock<HashMap<proto::djtool::Sink, Box<dyn sink::Sink + Send + Sync>>>>,
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

impl DjTool {
    pub async fn serve(&self, shutdown_rx: watch::Receiver<bool>) -> Result<()> {
        let host = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let grpc_addr = SocketAddr::new(host, 21022);
        let static_addr = SocketAddr::new(host, 21011);

        println!("grpc listening at {}", grpc_addr);
        println!("frontend served at {}", static_addr);

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

impl DjTool {
    pub async fn is_connected(&self, source: proto::djtool::Source) -> bool {
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
        sources.insert(proto::djtool::Source::Spotify, Box::new(spotify_client));
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
        let _ = self.connect_spotify(creds, oauth);
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
        // Ok(Self {
        //     // backends,
        //     sources: Arc::new(RwLock::new(HashMap::new())),
        //     connection: None,
        //     config: Arc::new(RwLock::new(config)),
        //     // spotify: Arc::new(client),
        //     // spotify: Arc::new(client),
        //     // transcoder,
        //     request_spotify_download: Arc::new(Semaphore::new(10)),
        // })
    }
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

// async fn download_youtube(&self, video_id: String) -> Result<()> {
//     let temp_dir = TempDir::new("djtool")?;
//     // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.aiff");
//     let downloaded_audio = temp_dir.path().join(&video_id);
//     let audio = self
//         .downloader
//         // .download_audio(video_id, temp_dir.path().to_path_buf())
//         .download_audio(
//             video_id,
//             // PathBuf::from("/home/roman/dev/djtool/Touchpad.webm"),
//             &downloaded_audio,
//             // temp_dir.join(video_id),
//             // PathBuf::from(format!("/home/roman/dev/djtool/Touchpad.webm"),
//         )
//         .await?;

//     // transcode to MP3
//     // println!("temp dir: {}", temp_dir.path().display());
//     // println!("audio output: {}", audio.audio_file.display());
//     // let input_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.webm");
//     // let output_file = PathBuf::from("/home/roman/dev/djtool/Touchpad.mp3");
//     let output_file = PathBuf::from("/Users/roman/dev/djtool/Touchpad.mp3");
//     let res = tokio::task::spawn_blocking(move || {
//         transcode2::test(audio.audio_file, output_file);
//         // let mut transcoder = Transcoder::new(audio.audio_file, output_file).unwrap();
//         // transcoder.start().unwrap();
//     })
//     .await?;
//     Ok(())
// }

fn with_spotify(
    spotify: SpotifyClient,
) -> impl Filter<Extract = (SpotifyClient,), Error = Infallible> + Send + Clone {
    warp::any().map(move || spotify.clone())
}

fn with_youtube(
    youtube: YoutubeClient,
) -> impl Filter<Extract = (YoutubeClient,), Error = Infallible> + Send + Clone {
    warp::any().map(move || youtube.clone())
}

pub async fn spotify_pkce_callback_handler(
    query: spotify::auth::pkce::CallbackQuery,
    sp: SpotifyClient,
) -> std::result::Result<impl Reply, Infallible> {
    let redirect_url = match query.code {
        Some(code) => {
            sp.authenticator
                .handle_user_login_callback(spotify::auth::SpotifyLoginCallback::Pkce {
                    code,
                    state: query.state.unwrap_or(String::new()),
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
    Ok(warp::reply::html(body))
}

// #[tokio::main]
// async fn main_async() {
//     let debug_dir = dirs::home_dir().unwrap().join(".djtool").join("debug");
//     let mock = Youtube::new(debug_dir).unwrap();
//     let results = mock.search("Touchpad Two Shell".to_string()).await.unwrap();
//     println!("search results: {:?}", results);
// }
