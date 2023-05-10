use super::ConfigError;
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::AsyncReadExt;
use std::path::Path;

#[async_trait::async_trait]
pub trait Persist: Serialize + DeserializeOwned {
    async fn load<P: AsRef<Path> + Send + Sync>(config_file: P) -> Result<Self, ConfigError> {
        let mut file = tokio::fs::File::open(config_file).await?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).await?;
        let test = buf.to_owned();
        let deser =
            serde_json::from_str::<Self>(&test).map_err(|err| ConfigError::ParseError(err))?;
        Ok(deser)
    }

    async fn save<P: AsRef<Path> + Send + Sync>(&self, config_file: P) -> Result<(), ConfigError> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(config_file)?;
        serde_json::to_writer(&file, &self)?;
        Ok(())
    }
}
