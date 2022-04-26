#![allow(warnings)]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod cli;
// mod spotify;
// mod source;
// mod utils;
// mod proto;
// mod config;

use anyhow::Result;
use clap::Parser;
use dirs;
use djtool::config::Persist;
use djtool::spotify::model::{Id, PlaylistId, UserId};
use djtool::youtube::Youtube;
use djtool::{DjTool, SPLASH_LOGO};
use futures::Stream;
use futures_util::pin_mut;
use futures_util::{StreamExt, TryStreamExt};
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Deref;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::thread;
use tauri::Event;
use tauri::Manager;
use tempdir::TempDir;
use tokio::signal;
use tokio::sync::{broadcast, mpsc, watch, Mutex, RwLock, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server as TonicServer, Code, Request, Response, Status};
use warp::{Filter, Rejection, Reply};
// remove
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

#[derive(Parser, Debug, Clone)]
pub enum Command {
    // #[clap(name = "start", about = "start the server")]
    // Start(StartOpts),
    #[cfg(feature = "spotify")]
    #[clap(name = "spotify", about = "spotify commands")]
    Spotify(djtool::spotify::cli::Options),
    // Spotify(crate::spotify::cli::Options),
    // Spotify(djtool::spotify::cli::Options),
    // Spotify(spotify::cli::Options),
    #[cfg(feature = "youtube")]
    #[clap(name = "youtube", about = "youtube commands")]
    Youtube(djtool::cli::YoutubeOpts),
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
fn build_ui() -> anyhow::Result<tauri::App> {
    let menu = tauri::Menu::new().add_submenu(tauri::Submenu::new(
        "djtool",
        tauri::Menu::new().add_native_item(tauri::MenuItem::Quit),
    ));

    let mut app = tauri::Builder::default()
        .menu(menu)
        .build(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(app)
}

fn main2() -> Result<(), Box<dyn std::error::Error>> {
    let m = MultiProgress::new();
    let status = m.add(ProgressBar::new_spinner());

    // let sty = ProgressStyle::default_bar();
    // status.set_style(sty.clone());

    let pb = m.add(ProgressBar::new(128));
    // pb.set_style(sty.clone());

    let h1 = thread::spawn(move || {
        for i in 0..128 {
            pb.set_message(format!("item #{}", i + 1));
            pb.inc(1);
            thread::sleep(Duration::from_millis(15));
        }
        pb.finish_with_message("done");
    });
    m.join_and_clear()?;
    // h1.join();
    // m.clear().unwrap();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", SPLASH_LOGO);
    let opts = Opts::parse();
    let (shutdown_tx, _) = broadcast::channel(10);
    let shutdown_tx_signal = shutdown_tx.clone();
    let shutdown_tx_ui = shutdown_tx.clone();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        // .enable_time()
        // .enable_io()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    runtime.spawn(async move {
        signal::ctrl_c().await.ok().map(|_| {
            println!("received shutdown");
            let _ = shutdown_tx_signal.send(true);
            // .expect("send shutdown signal");
            // wait for all to finish up
            // app.trigger_global("test", None);
            // for (_, window) in app.windows().into_iter() {
            //     window.close();
            // }
            // app.exit(0);
        });
    });

    match opts.subcommand {
        None => {
            let app = build_ui()?;
            let _ = thread::spawn(move || {
                runtime.block_on(async move {
                    // let config_dir = dirs::home_dir().unwrap().join(".djtool");
                    // println!("config dir: {}", config_dir.display());
                    // let tool = DjTool::persistent(&config_dir).await.unwrap();
                    let tool = Arc::new(DjTool::persistent(None::<PathBuf>).await.unwrap());
                    // tool.connect_sources().await;
                    let tool_clone = tool.clone();
                    tokio::task::spawn(async move {
                        tool_clone.connect_sources().await;
                        // tool.connect_sources().await;
                    });
                    // println!("connected sources");

                    // tool.sync_library().await.unwrap();
                    tool.serve(shutdown_tx).await;
                    std::process::exit(0);
                    // tool.serve(async move {
                    //     shutdown_rx.recv().await.expect("failed to shutdown");
                    // })
                    // .await;
                });
            });

            app.run(move |handle, event| {
                match event {
                    Event::ExitRequested { api, .. } => {
                        let _ = shutdown_tx_ui.send(true);
                        // .unwrap();
                        println!("exiting");
                        // thread::sleep(std::time::Duration::from_secs(10));
                        // println!("exiting for real");
                        // api.prevent_exit();
                    }
                    _ => {}
                }
            });
        }
        Some(Command::Spotify(cfg)) => {
            // let m = MultiProgress::new();
            // let status = m.add(ProgressBar::new_spinner());
            // let pb = m.add(ProgressBar::new(128));

            // let h1 = thread::spawn(move || {
            //     for i in 0..128 {
            //         pb.set_message(format!("item #{}", i + 1));
            //         pb.inc(1);
            //         thread::sleep(Duration::from_millis(15));
            //     }
            //     pb.finish_with_message("done");
            //     status.finish_with_message("done");
            // });
            // m.join_and_clear()?;
            // h1.join();
            djtool::spotify::cli::CLI::parse(runtime, shutdown_tx, cfg);
        }
        Some(Command::Youtube(cfg)) => {}
    };
    println!("waiting for runtime to become idle");
    // runtime.shutdown_on_idle().wait().unwrap();
    Ok(())
}
