#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use anyhow::Result;
use djtool::config::Persist;
use dirs;
use djtool::DjTool;
use futures::Stream;
use futures_util::pin_mut;
use futures_util::{StreamExt, TryStreamExt};
use reqwest;
use djtool::spotify::model::{Id, PlaylistId, UserId};
use djtool::youtube::Youtube;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::thread;
use tauri::Event;
use tempdir::TempDir;
use tokio::sync::{mpsc, watch, Mutex, RwLock, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server as TonicServer, Code, Request, Response, Status};
use warp::{Filter, Rejection, Reply};

const SPLASH_LOGO: &str = r"

 ___/ / (_) /____  ___  / /
/ _  / / / __/ _ \/ _ \/ / 
\_,_/_/ /\__/\___/\___/_/  
   |___/                   
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", SPLASH_LOGO);

    // let _ = thread::spawn(|| {
    //     let runtime = tokio::runtime::Runtime::new().unwrap();
    //     runtime.block_on(async {
    //         let config_dir = dirs::home_dir().unwrap().join(".djtool");
    //         println!("config dir: {}", config_dir.display());

    //         let tool = DjTool::new(&config_dir).await.unwrap();
    //         let spotify_client = tool.spotify.clone();
    //         // let backend_client = tool.backend.clone();

    //         // let results = tool
    //         //     .backends
    //         //     .search("Touchpad Two Shell".to_string())
    //         //     .await
    //         //     .unwrap();
    //         // println!("search results: {:?}", results);

    //         println!("getting lock on the library path");
    //         let (library_dir, _) = {
    //             let config = tool.config.read().await;
    //             (
    //                 config.library.library_dir.to_owned(),
    //                 config.debug_dir.to_owned(),
    //             )
    //         };

    //         // spin up a webserver
    //         let server = tokio::spawn(async move {
    //             let library = warp::path("static").and(warp::fs::dir(library_dir));

    //             let spotify_pkce_callback = warp::get()
    //                 .and(warp::path!("spotify" / "pkce" / "callback"))
    //                 .and(warp::query::<spotify::auth::pkce::CallbackQuery>())
    //                 .and(with_spotify(spotify_client.clone()))
    //                 .and_then(spotify_pkce_callback_handler);

    //             #[cfg(feature = "debug")]
    //             let routes = {
    //                 let debug_spotify_playlists = warp::get()
    //                     .and(warp::path!("debug" / "spotify" / "playlists"))
    //                     .and(warp::query::<debug::DebugSpotifyPlaylistsQuery>())
    //                     .and(with_spotify(spotify_client.clone()))
    //                     .and_then(debug::debug_spotify_playlists_handler);

    //                 let youtube = Arc::new(Youtube::new().unwrap());
    //                 let debug_youtube_search = warp::get()
    //                     .and(warp::path!("debug" / "youtube" / "search"))
    //                     .and(warp::query::<debug::DebugYoutubeSearchQuery>())
    //                     .and(with_youtube(youtube.clone()))
    //                     .and_then(debug::debug_youtube_search_handler);

    //                 spotify_pkce_callback
    //                     .or(library)
    //                     .or(debug_youtube_search)
    //                     .or(debug_spotify_playlists)
    //             };

    //             #[cfg(not(feature = "debug"))]
    //             let routes = spotify_pkce_callback.or(library);

    //             println!("starting server now ...");
    //             warp::serve(routes)
    //                 // .try_bind_with_graceful_shutdown(([127, 0, 0, 1], DEFAULT_PORT), )
    //                 .run(([0, 0, 0, 0], DEFAULT_PORT))
    //                 .await;
    //         });

    //         // tool.download_youtube("_Q8ELKOLudE".to_string())
    //         //     .await
    //         //     .unwrap();

    //         server.await;
    //     });
    // });

    let mut app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|handle, event| {
        match event {
            Event::ExitRequested { api, .. } => {
                println!("exiting");
                // thread::sleep(std::time::Duration::from_secs(10));
                // println!("exiting for real");
                // api.prevent_exit();
            }
            _ => {}
        }
    });

    // unreacheable
    Ok(())
}
