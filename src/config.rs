use anyhow::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;
use std::io;
use std::marker::{Send, Sync};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub library_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub config_dir: PathBuf,
    pub debug_dir: PathBuf,
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
            .truncate(true)
            .open(config_file)?;
        // let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(&file, &self)?;
        Ok(())
    }
}

impl Persist for Library {}

impl Config {
    pub async fn open<T: AsRef<Path> + Send + Sync>(config_dir: T) -> Result<Self> {
        let debug_dir = config_dir.as_ref().join("debug");
        let config_file = config_dir.as_ref().join("config.json");

        let _ = tokio::fs::create_dir_all(&config_dir).await;
        let _ = tokio::fs::create_dir_all(&debug_dir).await;

        // load the library
        let library = Library::load(&config_file).await;
        let library = match library {
            Err(_) => {
                let default_library_dir = dirs::audio_dir()
                    .or(dirs::download_dir())
                    .unwrap()
                    .join("djtool");
                let empty = Library {
                    library_dir: default_library_dir,
                };
                empty.save(&config_file).await?;
                empty
            }
            Ok(library) => library,
        };
        let _ = tokio::fs::create_dir_all(&library.library_dir).await;

        println!("loaded library: {:?}", library);
        Ok(Self {
            library,
            config_dir: config_dir.as_ref().to_owned(),
            debug_dir,
            config_file,
        })
    }

    pub fn debug_dir(&self) -> &PathBuf {
        &self.debug_dir
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
