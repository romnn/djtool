use super::{preflight, Download, DownloadProgress, Error, ProgressCallback};
use http::header::HeaderMap;

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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn on_progress(mut self, callback: impl Fn(DownloadProgress) + Send + 'static) -> Self {
        self.progress = Some(Box::new(callback));
        self
    }

    #[must_use]
    pub fn chunk_size(mut self, chunk_size: u64) -> Self {
        self.chunk_size = Some(chunk_size);
        self
    }

    #[must_use]
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = Some(concurrency);
        self
    }

    #[must_use]
    pub fn client(mut self, client: Arc<reqwest::Client>) -> Self {
        self.client = client;
        self
    }

    #[must_use]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Create a new download
    ///
    /// ## Example
    /// ```rust
    /// let buffer = tokio::io::BufWriter::new(Vec::new());
    /// let url = "https://google.com";
    /// let mut dl = Builder::new().download(url, &mut buffer).await?;
    /// dl.start().await?;
    /// assert!(buffer.into_inner().len() > 0);
    /// ```
    ///
    /// ## Errors
    /// If the given url is invalid.
    pub async fn download<W>(
        self,
        url: impl reqwest::IntoUrl,
        writer: W,
    ) -> Result<Download<W>, Error>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        let url = url.into_url()?;
        let preflight = preflight::send(&self.client, url.clone()).await.ok();

        Ok(Download {
            url,
            writer,
            preflight,
            client: self.client,
            headers: self.headers.unwrap_or_default(),
            concurrency: self.concurrency,
            chunk_size: self.chunk_size,
            progress: self.progress,
        })
    }
}
