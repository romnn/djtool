#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod cli;

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
use tempdir::TempDir;
use tokio::signal;
use tokio::sync::{broadcast, mpsc, watch, Mutex, RwLock, Semaphore};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server as TonicServer, Code, Request, Response, Status};
use warp::{Filter, Rejection, Reply};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", SPLASH_LOGO);
    let opts = cli::Opts::parse();
    let (shutdown_tx, _) = broadcast::channel(1);
    let shutdown_tx_signal = shutdown_tx.clone();
    let shutdown_tx_ui = shutdown_tx.clone();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.spawn(async move {
        signal::ctrl_c().await.ok().map(|_| {
            println!("received shutdown");
            shutdown_tx_signal.send(true).expect("send shutdown signal");
        });
    });

    match opts.subcommand {
        None => {
            let _ = thread::spawn(move || {
                runtime.block_on(async move {
                    // let config_dir = dirs::home_dir().unwrap().join(".djtool");
                    // println!("config dir: {}", config_dir.display());
                    // let tool = DjTool::persistent(&config_dir).await.unwrap();
                    let tool = DjTool::persistent(None::<PathBuf>).await.unwrap();
                    tool.connect_sources().await;
                    println!("connected sources");

                    // tool.sync_library().await.unwrap();
                    tool.serve(shutdown_tx).await;
                    // tool.serve(async move {
                    //     shutdown_rx.recv().await.expect("failed to shutdown");
                    // })
                    // .await;
                });
            });

            let mut app = tauri::Builder::default()
                .build(tauri::generate_context!())
                .expect("error while running tauri application");

            app.run(move |handle, event| {
                match event {
                    Event::ExitRequested { api, .. } => {
                        shutdown_tx_ui.send(true).unwrap();
                        println!("exiting");
                        // thread::sleep(std::time::Duration::from_secs(10));
                        // println!("exiting for real");
                        // api.prevent_exit();
                    }
                    _ => {}
                }
            });
        }
        Some(cli::Command::Spotify(cfg)) => {}
        Some(cli::Command::Youtube(cfg)) => {}
    };
    println!("waiting for runtime to become idle");
    // runtime.shutdown_on_idle().wait().unwrap();
    Ok(())
}
