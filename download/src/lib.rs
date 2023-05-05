// #![allow(warnings)]

mod builder;
mod chunk;
pub mod preflight;

pub use builder::Builder;
use chunk::{compute_chunk_size, compute_ranges, Chunk};

use futures_util::{stream, StreamExt};
use http::header::HeaderMap;
use std::sync::{atomic, Arc};
use tokio::fs;
use tokio::sync::mpsc;

type ProgressTx = tokio::sync::mpsc::Sender<u64>;
type ProgressRx = tokio::sync::mpsc::Receiver<u64>;
type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + 'static>;

#[must_use]
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
    url: reqwest::Url,
    headers: HeaderMap,
    writer: W,
    concurrency: Option<usize>,
    chunk_size: Option<u64>,
    preflight: Option<preflight::Response>,
    progress: Option<ProgressCallback>,
}

impl<W> std::fmt::Debug for Download<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Download")
            .field("url", &self.url)
            .field("concurrency", &self.concurrency())
            .field("chunk_size", &self.chunk_size())
            .field("content_length", &self.content_length())
            .field("rangeable", &self.is_rangeable())
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
    /// Create a new download
    ///
    /// # Errors
    /// If the given url is invalid
    pub async fn new(
        client: Arc<reqwest::Client>,
        url: impl reqwest::IntoUrl,
        writer: W,
    ) -> Result<Self, Error> {
        let url = url.into_url()?;
        let preflight = preflight::send(&client, url.clone()).await.ok();

        Ok(Self {
            client,
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
        self.preflight
            .as_ref()
            .and_then(|f| f.content_len)
            .map(|content_len| compute_chunk_size(content_len, concurrency, None, None))
    }

    pub fn concurrency(&self) -> usize {
        self.concurrency.unwrap_or_else(default_concurrency)
    }

    pub fn set_chunk_size(&mut self, chunk_size: u64) {
        self.chunk_size = Some(chunk_size);
    }

    pub fn set_concurrency(&mut self, concurrency: usize) {
        self.concurrency = Some(concurrency);
    }

    pub fn content_length(&self) -> Option<u64> {
        self.preflight.as_ref().and_then(|pf| pf.content_len)
    }

    pub fn is_rangeable(&self) -> bool {
        self.preflight.as_ref().map_or(false, |f| f.rangeable)
    }
}

impl<W> Download<W>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    async fn download(mut self) -> Result<(), Error> {
        let response = self
            .client
            .get(self.url.clone())
            .send()
            .await?
            .error_for_status()?;
        let total = response.content_length().or(self.content_length());

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
        let temp_dir = tempfile::tempdir()?;
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
        let chunks: Vec<_> = range_iter
            .map(|range| {
                let name = format!("{}-{}.chunk", range.start, range.end);
                Chunk {
                    client: self.client.clone(),
                    headers: self.headers.clone(),
                    url: self.url.clone(),
                    dest: temp_dir.path().join(name),
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
                    if let Err(_err) = &res {
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
            .flatten()
            .collect::<Result<Vec<_>, _>>()?;

        // recombine chunks to destination sequentially
        for chunk in chunks.iter() {
            let mut chunk_file = fs::File::open(&chunk.dest).await?;
            tokio::io::copy(&mut chunk_file, &mut self.writer).await?;
        }
        Ok(())
    }

    /// Starts the download
    ///
    /// # Errors
    /// If the download fails.
    pub async fn start(self) -> Result<(), Error> {
        match self.content_length() {
            Some(content_len) if self.is_rangeable() => self.download_ranged(content_len).await,
            _ => self.download().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Builder;
    use anyhow::Result;
    use futures_util::StreamExt;
    use tokio::io;

    const INCOMPETECH: &str = "https://incompetech.com/music/royalty-free/mp3-royaltyfree/";

    async fn download_to(
        url: impl reqwest::IntoUrl,
        mut writer: impl io::AsyncWrite + Unpin,
    ) -> Result<()> {
        let client = reqwest::Client::default();
        let response = client.get(url).send().await?.error_for_status()?;
        dbg!(response.content_length());

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
        format!("{challenge:?}")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_non_rangeable_download() -> Result<()> {
        let mut data = io::BufWriter::new(Vec::new());
        let mut expected = io::BufWriter::new(Vec::new());

        let url = "https://llvm.org/img/LLVMWyvernSmall.png";
        dbg!(&url);
        let dl = Builder::new().download(url, &mut data).await?;
        assert!(dl.is_rangeable());
        dbg!(&dl.preflight);
        dl.start().await?;

        download_to(url, &mut expected).await?;
        let data = data.into_inner();
        let expected = expected.into_inner();
        assert_eq!(&data.len(), &expected.len());
        assert_eq!(hash(&data), hash(&expected));
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rangeable_download() -> Result<()> {
        let mut data = io::BufWriter::new(Vec::new());
        let mut expected = io::BufWriter::new(Vec::new());

        let url = format!("{}/Aerosol%20of%20my%20Love.mp3", &INCOMPETECH);
        dbg!(&url.as_str());
        let dl = Builder::new().download(url.clone(), &mut data).await?;
        dbg!(&dl.preflight);
        assert!(dl.is_rangeable());
        dl.start().await?;

        download_to(url, &mut expected).await?;
        let data = data.into_inner();
        let expected = expected.into_inner();
        assert_eq!(&data.len(), &expected.len());
        assert_eq!(hash(&data), hash(&expected));
        Ok(())
    }
}
