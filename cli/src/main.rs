use clap::Parser;
use std::path::PathBuf;
use tokio::signal;
use tokio::sync::broadcast;

#[derive(Parser, Debug, Clone)]
pub enum Command {
    #[cfg(feature = "spotify")]
    #[clap(name = "spotify", about = "spotify commands")]
    Spotify(spotify::cli::Options),
}

#[derive(Parser, Debug, Clone)]
#[clap(
    name = "djtool cli",
    version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"),
    author = "romnn <contact@romnn.com>",
    about = "djtool",
)]
pub struct Opts {
    #[clap(long = "library")]
    pub library_dir: Option<PathBuf>,
    #[clap(subcommand)]
    pub subcommand: Command,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load environment variables
    dotenv::dotenv().ok();

    let opts = Opts::parse();
    let (shutdown_tx, _) = broadcast::channel(10);
    let shutdown_tx_signal = shutdown_tx.clone();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    runtime.spawn(async move {
        signal::ctrl_c().await.ok().map(|_| {
            println!("received shutdown");
            let _ = shutdown_tx_signal.send(true);
        });
    });

    match opts.subcommand {
        Command::Spotify(cfg) => {
            spotify::cli::CLI::parse(&runtime, shutdown_tx, cfg);
        }
    };
    runtime.shutdown_background();
    Ok(())
}
