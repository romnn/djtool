use super::{Error, ProgressTx};
use futures_util::StreamExt;
use http::header::HeaderMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[allow(
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]
pub fn compute_chunk_size(
    content_len: u64,
    concurrency: usize,
    min: Option<u64>,
    max: Option<u64>,
) -> u64 {
    let cs = content_len as f64 / concurrency as f64;
    let mut cs = cs.abs().round() as u64;

    // if chunk size >= 102400000 bytes set default to (chunk size / 2)
    while cs >= 102_400_000 {
        cs /= 2;
    }

    // set default min chunk size to 2M, or file size / 2
    let mut min = min.unwrap_or(2_097_152_u64);
    if min >= content_len {
        min = content_len / 2;
    }

    // if chunk size < min size set chunk size to min
    cs = cs.max(min);

    // change chunk size if max chunk size are set and chunk size > max size
    if let Some(max) = max {
        cs = cs.min(max);
    }

    // while chunk size > total file size, divide chunk / 2
    while cs >= content_len {
        cs = content_len / 2;
    }
    cs
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Range {
    pub idx: u64,
    pub start: u64,
    pub end: u64,
}

#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]
pub fn compute_ranges(content_len: u64, chunk_size: u64) -> impl Iterator<Item = Range> {
    let num_chunks = content_len as f64 / chunk_size as f64;
    let num_chunks = num_chunks.ceil().abs() as u64;
    (0..num_chunks).map(move |idx| {
        // let idx = idx as u64;
        let start = idx * chunk_size;
        let end = (idx + 1) * chunk_size;
        let end = (end - 1).min(content_len);
        Range { idx, start, end }
    })
}

#[derive(Debug)]
pub struct Chunk {
    pub client: Arc<reqwest::Client>,
    pub headers: HeaderMap,
    pub url: reqwest::Url,
    pub dest: PathBuf,
    pub range_start: u64,
    pub range_end: u64,
}

impl std::fmt::Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chunk({}-{})", self.range_start, self.range_end)
    }
}

impl Chunk {
    pub async fn download(&self, progress: &ProgressTx) -> Result<(), Error> {
        let response = self
            .client
            .get(self.url.clone())
            .headers(self.headers.clone())
            .header(
                "Range",
                format!("bytes={}-{}", self.range_start, self.range_end),
            )
            .send()
            .await?
            .error_for_status()?;

        let mut dest = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.dest)
            .await?;

        let mut data_stream = response.bytes_stream();
        while let Some(byte_chunk) = data_stream.next().await {
            let byte_chunk = byte_chunk?;
            dest.write_all(&byte_chunk).await?;
            progress.send(byte_chunk.len() as u64).await.ok();
        }
        Ok(())
    }
}
