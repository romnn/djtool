mod config;
mod persist;
pub use config::{Config, ConfigError};
pub use persist::Persist;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub path: PathBuf,
}

impl Default for Library {
    fn default() -> Self {
        let path = dirs::audio_dir()
            .or(dirs::download_dir())
            .unwrap()
            .join("djtool");
        Library { path }
    }
}

impl Persist for Library {}
