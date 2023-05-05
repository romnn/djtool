use super::{preflight::preflight, Download, DownloadProgress, Error, ProgressCallback};
use http::header::HeaderMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive()]
pub struct Builder {
    client: Arc<reqwest::Client>,
    concurrency: Option<usize>,
    chunk_size: Option<u64>,
    headers: Option<HeaderMap>,
    progress: Option<ProgressCallback>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            client: Arc::new(reqwest::Client::new()),
            concurrency: None,
            chunk_size: None,
            headers: None,
            progress: None,
        }
    }
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_progress(
        mut self,
        callback: impl Fn(DownloadProgress) -> () + Send + 'static,
    ) -> Self {
        self.progress = Some(Box::new(callback));
        self
    }

    pub fn chunk_size(mut self, chunk_size: u64) -> Self {
        self.chunk_size = Some(chunk_size);
        self
    }

    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = Some(concurrency);
        self
    }

    pub fn client(mut self, client: Arc<reqwest::Client>) -> Self {
        self.client = client;
        self
    }

    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    pub async fn download<W>(
        mut self,
        url: impl reqwest::IntoUrl,
        // dest: impl Into<PathBuf>,
        writer: W,
    ) -> Result<Download<W>, Error>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        let url = url.into_url()?;
        let preflight = preflight(&self.client, url.clone()).await?;

        // let concurrency = self
        //     .concurrency
        //     .unwrap_or_else(|| super::default_concurrency());
        //
        // let chunk_size = self
        //     .chunk_size
        //     .unwrap_or_else(|| super::chunk::compute_size(&preflight, concurrency, None, None));
        //
        Ok(Download {
            client: self.client,
            temp_dir: tempfile::tempdir()?,
            url,
            headers: self.headers.unwrap_or_default(),
            writer,
            concurrency: self.concurrency,
            chunk_size: self.chunk_size,
            preflight,
            progress: self.progress,
        })
    }
}
