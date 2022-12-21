use crate::utils;
use anyhow::Result;
use boa;
use futures_util::{stream, StreamExt};
use http::header::HeaderMap;
use lazy_static::lazy_static;
use rayon::prelude::*;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tempdir::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};

#[derive(Clone, Debug)]
pub struct Downloader {
    client: Arc<reqwest::Client>,
}

impl Downloader {
    async fn download() -> Result<()> {
        Ok(())
    }
}

struct PreflightDownloadInfo {
    content_length: usize,
    #[allow(dead_code)]
    content_disposition_name: Option<String>,
    #[allow(dead_code)]
    rangeable: bool,
}

struct Chunk {
    client: Arc<reqwest::Client>,
    headers: HeaderMap,
    url: String,
    path: PathBuf,
    range_start: usize,
    range_end: usize,
    downloaded: usize,
}

impl Chunk {
    async fn download(&mut self, progress: &mpsc::Sender<Result<usize>>) -> Result<()> {
        let res = self
            .client
            .get(self.url.clone())
            .headers(self.headers.clone())
            .header(
                "Range",
                format!("bytes={}-{}", self.range_start, self.range_end),
            )
            .send()
            .await?;

        let mut data_stream = res.bytes_stream();
        let mut chunk_file = tokio::fs::File::create(self.path.clone()).await?;
        // if let Err(err) = chunk_file {
        //     let _ = progress.send(Err(err.into())).await;
        //     break;
        // }
        // let chunk_file = chunk_file.unwrap();
        while let Some(byte_chunk) = data_stream.next().await {
            // let chunk = item.or(Err(format!("Error while downloading file")))?;
            let byte_chunk = byte_chunk?;
            // output_file.write(&chunk).unwrap();
            // .or(Err(format!("Error while writing to file")))?;
            // let new = min(downloaded + (chunk.len() as u64), total_size);
            // let new_percent = 100.0 * (downloaded as f32 / total_size as f32);
            // if new_percent - old_percent > 10.0 {
            //     old_percent = new_percent;
            //     println!("downloaded: {}%", new_percent);
            // }
            // downloaded = new;
            // pb.set_position(new);
            // output_file.start_seek(chunk.range_start).await?;
            chunk_file.write_all(&byte_chunk).await?;
            let _ = progress.send(Ok(byte_chunk.len())).await;
            self.downloaded += byte_chunk.len();
        }
        // chunk_file.close()?;
        Ok(())
    }
}

pub struct DownloadProgress {
    pub downloaded: usize,
    pub total: Option<usize>,
}

pub struct Download {
    client: Arc<reqwest::Client>,
    temp_dir: TempDir,
    concurrency: usize,
    url: String,
    headers: HeaderMap,
    output_path: PathBuf,
    chunk_size: usize,
    info: PreflightDownloadInfo,
    downloaded: usize,
    chunks: Arc<Mutex<Vec<Chunk>>>,
    started_at: Instant,
    progress: Option<Box<dyn Fn(DownloadProgress) -> () + Send + 'static>>,
}

impl Download {
    pub async fn new(url: &String, output_path: impl AsRef<Path>) -> Result<Self> {
        let client = Arc::new(reqwest::Client::new());
        let info = Self::preflight(&client, &url).await?;
        // println!("preflight check completed");
        let concurrency = Self::default_concurrency();
        let chunk_size = Self::default_chunk_size(&info, concurrency, None, None);
        // let sanitized_name = utils::sanitize_filename(video.title.clone().unwrap());
        //
        // create channel for download updates
        // let (tx, mut rx) = mpsc::channel::<Result<usize>>(100);

        let mut download = Self {
            client,
            temp_dir: TempDir::new("djtool")?,
            concurrency,
            url: url.to_string(),
            headers: HeaderMap::new(),
            output_path: output_path.as_ref().to_owned(),
            chunk_size,
            info,
            downloaded: 0,
            chunks: Arc::new(Mutex::new(Vec::<Chunk>::new())),
            started_at: Instant::now(),
            progress: None,
        };
        download.compute_chunks();
        Ok(download)
    }

    pub fn on_progress(&mut self, callback: impl Fn(DownloadProgress) -> () + Send + 'static) {
        self.progress = Some(Box::new(callback));
    }

    #[allow(dead_code)]
    pub fn set_chunk_size(&mut self, chunk_size: usize) {
        self.chunk_size = chunk_size;
        // we do not worry about too much concurrency for too little chunks, as this wont create
        // additional overhead
        self.compute_chunks();
    }

    #[allow(dead_code)]
    pub fn set_concurrency(&mut self, concurrency: usize, min: Option<usize>, max: Option<usize>) {
        self.chunk_size = Self::default_chunk_size(&self.info, concurrency, min, max);
        self.compute_chunks();
    }

    pub fn default_concurrency() -> usize {
        let mut c = num_cpus::get() * 3;
        c = c.min(20);
        if c <= 2 {
            c = 4;
        }
        c
    }

    fn default_chunk_size(
        info: &PreflightDownloadInfo,
        concurrency: usize,
        min: Option<usize>,
        max: Option<usize>,
    ) -> usize {
        // let cs: f32 = NumCast::from(info.content_length).unwrap();
        // let cs: f32 = cs / NumCast::from(concurrency).unwrap();
        // let mut cs: u64 = NumCast::from(cs).unwrap();
        let mut cs = (info.content_length as f32 / concurrency as f32) as usize;

        // if chunk size >= 102400000 bytes set default to (chunk size / 2)
        if cs >= 102400000 {
            cs = cs / 2;
        }

        // set default min chunk size to 2M, or file size / 2
        let mut min = min.unwrap_or(2097152usize);
        if min >= info.content_length {
            min = info.content_length / 2;
        }

        // if chunk size < min size set chunk size to min
        cs = cs.max(min);

        // change chunk size if max chunk size are set and chunk size > max size
        if let Some(max) = max {
            cs = cs.min(max);
        }

        // when chunk size > total file size, divide chunk / 2
        if cs >= info.content_length {
            cs = info.content_length / 2;
        }
        cs
    }

    async fn preflight(client: &reqwest::Client, url: &String) -> Result<PreflightDownloadInfo> {
        let res = client.get(url).header("Range", "bytes=0-0").send().await?;
        if res.status().as_u16() >= 300 {
            // todo: return error
            // Response status code is not ok: %d res.status.as_str()
        }
        let headers = res.headers();
        let mut rangeable = false;
        // let mut content_length: Option<u64> = None;
        let mut content_length = res.content_length().unwrap();
        let content_disposition_name = headers
            .get("content-disposition")
            .and_then(|val| val.to_str().map(|s| s.to_string()).ok());
        if let Some(content_range) = headers
            .get("content-range")
            .and_then(|val| val.to_str().ok())
        {
            // check that content-range header is valid
            if !content_range.is_empty() && content_length == 1 {
                let content_range_parts: Vec<&str> = content_range.split("/").collect();
                if content_range_parts.len() == 2 {
                    content_length = content_range_parts[1].parse::<u64>()?;
                    rangeable = true
                }
            }
        }
        Ok(PreflightDownloadInfo {
            content_length: content_length.try_into().unwrap(),
            // content_length,
            // range_length,
            content_disposition_name,
            rangeable,
        })
    }

    fn compute_chunks(&mut self) {
        let num_chunks =
            0..(self.info.content_length as f32 / self.chunk_size as f32).ceil() as usize;
        // let (tx, _) = &self.progress;
        self.chunks = Arc::new(Mutex::new(
            num_chunks
                .into_iter()
                .map(|chunk| {
                    let range_start = chunk * self.chunk_size;
                    let range_end =
                        ((chunk + 1) * self.chunk_size - 1).min(self.info.content_length);
                    let chunk_name = format!(
                        "{}.{}-{}.chunk",
                        self.output_path
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or(""),
                        range_start,
                        range_end,
                    );
                    let path = self.temp_dir.path().join(chunk_name);

                    Chunk {
                        client: self.client.clone(),
                        // progress: tx.clone(),
                        headers: self.headers.clone(),
                        url: self.url.clone(),
                        path,
                        range_start,
                        range_end,
                        downloaded: 0,
                    }
                })
                .collect(),
        ));
    }

    pub async fn start(
        &mut self,
        // progress: Option<impl Fn(DownloadProgress) -> () + 'static>,
        // progress: impl Fn(DownloadProgress) -> () + 'static,
    ) -> Result<()> {
        self.started_at = Instant::now();

        let (tx, mut rx) = mpsc::channel::<Result<usize>>(100);
        // println!("starting download with {} chunks", chunks.len());

        // tokio::spawn(async move {
        // let chunks: Vec<_> = stream::iter(&self.chunks)
        //     .map(|chunk| {
        //         let sender = tx.clone();
        //         async move {
        //             println!("chunk {}-{}", chunk.range_start, chunk.range_end);
        //             let _ = sender.send(0).await;
        //             0
        //         }
        //     })
        //     .collect()
        //     .await;

        let concurrency = self.concurrency.clone();
        let client = self.client.clone();
        let output_path = self.output_path.clone();
        let headers = self.headers.clone();

        let chunks_clone = self.chunks.clone();
        let download = tokio::spawn(async move {
            let mut chunks = chunks_clone.lock().await;
            stream::iter(chunks.iter_mut())
                .for_each_concurrent(concurrency, move |chunk| {
                    // let progress = tx.clone();
                    // let client = client.clone();
                    // let headers = headers.clone();
                    let progress = tx.clone();
                    // println!("{}", chunk.path.display());

                    async move {
                        // println!("chunk {}-{}", chunk.range_start, chunk.range_end);
                        if let Err(err) = chunk.download(&progress).await {
                            let _ = progress.send(Err(err.into())).await;
                        };
                        // println!("chunk is done");
                    }
                })
                .await;
        });

        // let mut downloaded = 0;
        // let (_, rx) = &mut self.progress;
        while let Some(chunk_downloaded) = rx.recv().await {
            match chunk_downloaded {
                Ok(chunk_downloaded) => {
                    self.downloaded += chunk_downloaded;
                    if let Some(progress) = &self.progress {
                        (progress)(DownloadProgress {
                            downloaded: self.downloaded,
                            total: Some(self.info.content_length),
                        });
                    }
                    // println!("downloaded: {}", self.downloaded);
                }
                Err(err) => eprintln!("chunk failed: {}", err),
            }
        }
        download.await?;
        // println!("download completed: {}", self.downloaded);

        // concat = tokio::spawn(async move {

        // pre-allocate the output file
        let mut output_file = tokio::fs::File::create(self.output_path.clone()).await?;
        // output_file.set_len(self.info.content_length).await?;

        // recombine the chunk files into final file sequentially
        let chunks = self.chunks.lock().await;
        // let chunk_stream = stream::iter(chunks.iter())
        // chunks.iter()
        // .for_each(|chunk| async move {
        for chunk in chunks.iter() {
            let mut chunk_file = tokio::fs::File::open(chunk.path.clone()).await?;
            tokio::io::copy(&mut chunk_file, &mut output_file).await?;
        }
        // println!("concatenated: {}", self.output_path.display());
        Ok(())
    }
}
