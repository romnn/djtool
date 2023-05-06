// #![allow(warnings)]

pub mod external;
pub mod internal;

use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TranscodeProgress {
    pub elapsed: Duration,
    pub frame: usize,
    pub total_frames: usize,
    pub duration: Duration,
    pub timestamp: Duration,
}

pub type ProgressHandlerFunc = dyn FnMut(TranscodeProgress);

#[derive(Debug, Clone, Copy, Ord, PartialEq, Eq, PartialOrd)]
pub enum Codec {
    MP3,
    PCM,
}

#[derive(Default, Debug, Clone)]
pub struct TranscoderOptions {
    pub codec: Option<Codec>,
    pub bitrate_kbps: Option<usize>,
    pub sample_rate: Option<usize>,
    pub loudness_normalize: bool,
}

impl TranscoderOptions {
    #[must_use] pub fn mp3() -> Self {
        Self {
            codec: Some(Codec::MP3),
            bitrate_kbps: Some(192),
            sample_rate: None,
            loudness_normalize: true,
        }
    }

    #[must_use] pub fn matching() -> Self {
        Self {
            codec: Some(Codec::PCM),
            bitrate_kbps: None,
            // most importantly, we resample
            sample_rate: Some(22_050),
            loudness_normalize: false,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("transcode error: `{0:?}`")]
    Custom(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait Transcoder {
    fn transcode_blocking(
        &self,
        input_path: &Path,
        output_path: &Path,
        options: Option<&TranscoderOptions>,
        progess_handler: &mut ProgressHandlerFunc,
    ) -> Result<(), Error>;
}

#[cfg(test)]
mod tests {}
