#![allow(warnings)]

use djtool_model::{self as model, source};
use library::Library;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, watch, Mutex, RwLock, Semaphore};

pub type DynSource = Arc<Box<dyn source::Source + Send + Sync>>;

pub struct DjTool {
    sources: Arc<RwLock<HashMap<model::Service, DynSource>>>,
    library: Library,
    // sinks: Arc<RwLock<HashMap<proto::djtool::Service, Sink>>>,
    // transcoder: Arc<Box<dyn transcode::Transcoder + Sync + Send>>,
    // data_dir: Option<PathBuf>,
    // config: Arc<RwLock<Option<config::Config>>>,
    // login_done: Arc<(broadcast::Sender<bool>, broadcast::Receiver<bool>)>,
    // host: IpAddr,
    // port: u16,
}

impl std::fmt::Debug for DjTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DjTool").finish()
    }
}

impl std::fmt::Display for DjTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DjTool").finish()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {}

impl DjTool {
    pub fn new(
        // data_dir: Option<impl AsRef<Path> + Sync + Send + Clone>,
        library: Library,
    ) -> Result<Self, Error> {
        // let data_dir = data_dir
        //     .map(|d| d.as_ref().to_path_buf())
        //     .or(dirs::home_dir().map(|d| d.join(".djtool")))
        //     .ok_or(anyhow::anyhow!("no data dir available"))?;
        //
        // let config = config::Config::open(&data_dir).await?;
        // Ok(Self {
        //     data_dir: Some(data_dir.to_owned()),
        //     config: Arc::new(RwLock::new(Some(config))),
        //     ..Default::default()
        // })
        Ok(Self {
            sources: Arc::new(RwLock::new(HashMap::new())),
            library,
        })
    }
}
