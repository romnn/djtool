use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;
// use tokio::io::{BufReader, BufWriter};
use async_trait::async_trait;
use std::marker::{Send, Sync};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub library_path: PathBuf,
    // spotify_app_client_id
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub config_file: PathBuf,
    pub library: Library,
}

#[async_trait]
pub trait Persist: Serialize + DeserializeOwned {
    async fn load<P: AsRef<Path> + Send + Sync>(config_file: P) -> Result<Self> {
        let mut file = tokio::fs::File::open(config_file).await?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).await?;
        let test = buf.to_owned();
        let deser = serde_json::from_str::<Self>(&test)?;
        Ok(deser)
    }

    async fn save<P: AsRef<Path> + Send + Sync>(&self, config_file: P) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(config_file)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, &self)?;
        Ok(())
    }
}

impl Persist for Library {}

impl Config {
    pub async fn open<T: AsRef<Path> + Send + Sync>(config_dir: T) -> Result<Self> {
        let config_file = config_dir.as_ref().join("config.json");
        let library = Library::load(&config_file).await;
        let library = match library {
            Err(_) => {
                let default_library_path = dirs::audio_dir()
                    .or(dirs::download_dir())
                    .unwrap()
                    .join("djtool");
                let empty = Library {
                    library_path: default_library_path,
                };
                empty.save(&config_file).await?;
                empty
            }
            Ok(library) => library,
        };
        let _ = tokio::fs::create_dir_all(&library.library_path).await;
        println!("loaded library: {:?}", library);
        Ok(Self {
            library,
            config_file,
        })
    }

    // pub async fn load<T: AsRef<Path>>(config_dir: T) -> Result<Self> {
    //     match tokio::fs::File::open(config_dir.as_ref().join("config.json")).await {
    //         Ok(mut file) => {
    //             let mut config = String::new();
    //             file.read_to_string(&mut config).await?;
    //             let config = serde_json::from_str::<Config>(&config)?;
    //             Ok(config)
    //         }
    //         Err(err) => {
    //             println!("err kind: {:?}", err.kind());
    //             if err.kind() == std::io::ErrorKind::NotFound {
    //                 Config::new(config_dir).await
    //             } else {
    //                 Err(err.into())
    //             }
    //         }
    //     }
    // }

    // pub async fn save<T: AsRef<Path>>(&mut self, config_dir: T) -> Result<()> {
    //     let file = std::fs::OpenOptions::new()
    //         .write(true)
    //         .create(true)
    //         .open(config_dir.as_ref().join("config.json"))?;
    //     // let file = std::fs::File::open()?;
    //     let writer = std::io::BufWriter::new(file);
    //     serde_json::to_writer(writer, &self)?;
    //     Ok(())
    // }
}
