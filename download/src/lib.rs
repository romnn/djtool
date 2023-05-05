#![allow(warnings)]

mod builder;
mod chunk;
pub mod preflight;

pub use builder::Builder;
use chunk::{compute_chunk_size, compute_ranges, Chunk};
pub use preflight::PreflightResult;

use futures_util::{stream, StreamExt};
use http::header::HeaderMap;
use std::path::{Path, PathBuf};
use std::sync::{atomic, Arc};
use std::time::Instant;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};

type ProgressTx = tokio::sync::mpsc::Sender<u64>;
type ProgressRx = tokio::sync::mpsc::Receiver<u64>;
type ProgressCallback = Box<dyn Fn(DownloadProgress) -> () + Send + 'static>;

pub fn default_concurrency() -> usize {
    let mut c = num_cpus::get() * 3;
    c = c.min(20);
    if c <= 2 {
        c = 4;
    }
    c
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct Download<W> {
    client: Arc<reqwest::Client>,
    temp_dir: tempfile::TempDir,
    url: reqwest::Url,
    headers: HeaderMap,
    writer: W,
    concurrency: Option<usize>,
    chunk_size: Option<u64>,
    preflight: PreflightResult,
    progress: Option<ProgressCallback>,
}

impl<W> std::fmt::Debug for Download<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Download")
            .field("url", &self.url)
            .field("concurrency", &self.concurrency())
            .field("chunk_size", &self.chunk_size())
            .field("temp_dir", &self.temp_dir)
            .field("preflight", &self.preflight)
            .field("headers", &self.headers)
            .finish()
    }
}

impl<W> std::fmt::Display for Download<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Download").field("url", &self.url).finish()
    }
}

impl<W> Download<W> {
    pub async fn new(
        client: Arc<reqwest::Client>,
        url: impl reqwest::IntoUrl,
        writer: W,
    ) -> Result<Self, Error> {
        let url = url.into_url()?;
        let preflight = preflight::preflight(&client, url.clone()).await?;

        Ok(Self {
            client,
            temp_dir: tempfile::tempdir()?,
            url,
            headers: HeaderMap::new(),
            writer,
            concurrency: None,
            chunk_size: None,
            preflight,
            progress: None,
        })
    }

    pub fn chunk_size(&self) -> Option<u64> {
        let concurrency = self.concurrency();
        if let Some(chunk_size) = self.chunk_size {
            return Some(chunk_size);
        }
        if let Some(content_len) = self.preflight.content_len{
            Some(compute_chunk_size(content_len, concurrency, None, None))
        } else {
            None
        }
    }

    pub fn concurrency(&self) -> usize {
        self.concurrency.unwrap_or_else(|| default_concurrency())
    }

    pub fn set_chunk_size(&mut self, chunk_size: u64) {
        self.chunk_size = Some(chunk_size);
    }

    pub fn set_concurrency(&mut self, concurrency: usize) {
        self.concurrency = Some(concurrency);
    }

    pub fn is_rangeable(&self) -> bool {
        self.preflight.rangeable
    }
}

impl<W> Download<W>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    async fn download(mut self) -> Result<(), Error> {
        let response = self.client.get(self.url).send().await?.error_for_status()?;
        let total = response.content_length().or(self.preflight.content_len);

        let mut data_stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = data_stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            if let Some(progress) = &self.progress {
                (progress)(DownloadProgress { downloaded, total });
            }
            tokio::io::copy(&mut chunk.as_ref(), &mut self.writer).await?;
        }
        Ok(())
    }

    async fn download_ranged(mut self, content_len: u64) -> Result<(), Error> {
        let concurrency = self.concurrency();
        let chunk_size = compute_chunk_size(content_len, concurrency, None, None);

        let (tx, mut rx): (ProgressTx, ProgressRx) = mpsc::channel(concurrency * 3);

        let mut downloaded = 0;
        tokio::spawn(async move {
            while let Some(chunk_downloaded) = rx.recv().await {
                downloaded += chunk_downloaded;
                if let Some(progress) = &self.progress {
                    (progress)(DownloadProgress {
                        downloaded,
                        total: Some(content_len),
                    });
                }
            }
        });

        let range_iter = compute_ranges(content_len, chunk_size);
        let mut chunks: Vec<_> = range_iter
            .map(|range| {
                let name = format!("{}-{}.chunk", range.start, range.end);
                Chunk {
                    client: self.client.clone(),
                    headers: self.headers.clone(),
                    url: self.url.clone(),
                    dest: self.temp_dir.path().join(name),
                    range_start: range.start,
                    range_end: range.end,
                }
            })
            .collect();
        let chunks = Arc::new(chunks);

        let cancel = Arc::new(atomic::AtomicBool::new(false));
        let chunk_results = stream::iter(chunks.iter())
            .map(|chunk| {
                let progress = tx.clone();
                let cancel = cancel.clone();
                async move {
                    if cancel.load(atomic::Ordering::Relaxed) {
                        // println!("chunk {} canceled", &chunk);
                        return None;
                    }
                    // println!("chunk {} started", &chunk);
                    let res = chunk.download(&progress).await;
                    // println!("chunk {} done", &chunk);
                    if let Err(err) = &res {
                        cancel.store(true, atomic::Ordering::Relaxed);
                    }
                    Some(res)
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<Option<Result<(), Error>>>>()
            .await;

        // fail download if any chunk failed
        chunk_results
            .into_iter()
            .filter_map(|res| res)
            .collect::<Result<Vec<_>, _>>()?;

        // recombine chunks to destination sequentially
        for chunk in chunks.iter() {
            let mut chunk_file = fs::File::open(&chunk.dest).await?;
            tokio::io::copy(&mut chunk_file, &mut self.writer).await?;
        }
        Ok(())
    }

    pub async fn start(mut self) -> Result<(), Error> {
        match self.preflight.content_len{
            Some(content_len) if self.preflight.rangeable => {
                self.download_ranged(content_len).await
            }
            _ => self.download().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Builder, Download};
    use anyhow::Result;
    use futures_util::StreamExt;
    use std::path::PathBuf;
    use tokio::{
        fs,
        io::{self, AsyncWriteExt},
    };

    const INCOMPETECH: &str = "https://incompetech.com/music/royalty-free/mp3-royaltyfree/";

    async fn download_to(
        url: impl reqwest::IntoUrl,
        mut writer: impl io::AsyncWrite + Unpin,
    ) -> Result<()> {
        let client = reqwest::Client::default();
        let response = client.get(url).send().await?.error_for_status()?;
        dbg!(response.content_length());
        // let bytes = response.bytes().await?;
        // dbg!(&bytes.len());
        // writer.write_all(&bytes).await?;

        let mut data_stream = response.bytes_stream();
        while let Some(chunk) = data_stream.next().await {
            tokio::io::copy(&mut chunk?.as_ref(), &mut writer).await?;
        }
        Ok(())
    }

    pub fn hash(data: impl AsRef<[u8]>) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let challenge = hasher.finalize();
        format!("{:?}", challenge)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_non_rangeable_download() -> Result<()> {
        let mut data = io::BufWriter::new(Vec::new());
        let mut expected = io::BufWriter::new(Vec::new());

        let url = "https://llvm.org/img/LLVMWyvernSmall.png";
        dbg!(&url);
        let mut dl = Builder::new().download(url.clone(), &mut data).await?;
        assert!(dl.is_rangeable());
        dbg!(&dl.preflight);
        dl.start().await?;

        download_to(url, &mut expected).await?;
        let data = data.into_inner();
        let expected = expected.into_inner();
        assert_eq!(&data.len(), &expected.len());
        assert_eq!(hash(&data), hash(&expected));
        assert!(false);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rangeable_download() -> Result<()> {
        let mut data = io::BufWriter::new(Vec::new());
        let mut expected = io::BufWriter::new(Vec::new());

        // let mut options = fs::OpenOptions::new();
        // options.create(true);
        // options.truncate(true);
        // options.write(true);
        //
        // let mut data = options
        //     .open(&PathBuf::from(concat!(
        //         env!("CARGO_MANIFEST_DIR"),
        //         "/downloaded/music.mp3"
        //     )))
        //     .await?;
        // let mut expected = options
        //     .open(&PathBuf::from(concat!(
        //         env!("CARGO_MANIFEST_DIR"),
        //         "/downloaded/music_expected.mp3"
        //     )))
        //     .await?;

        let url = format!("{}/Aerosol%20of%20my%20Love.mp3", &INCOMPETECH);
        dbg!(&url.as_str());
        let mut dl = Builder::new().download(url.clone(), &mut data).await?;
        dbg!(&dl.preflight);
        assert!(dl.is_rangeable());
        dl.start().await?;

        // data.flush();
        // expected.flush();
        download_to(url, &mut expected).await?;
        let data = data.into_inner();
        let expected = expected.into_inner();
        assert_eq!(&data.len(), &expected.len());
        assert_eq!(hash(&data), hash(&expected));
        assert!(false);
        Ok(())
    }
}
