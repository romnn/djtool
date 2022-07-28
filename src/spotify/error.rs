use super::model;
use super::proto;
use crate::config::ConfigError;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing client_id")]
    MissingClientID(),
    #[error("missing user_id")]
    MissingUserID(),
    #[error("user must login at: {auth_url}")]
    RequireUserLogin { auth_url: reqwest::Url },
    #[error("auth method \"`{0:?}`\"unsupported")]
    Unsupported(Option<proto::djtool::spotify_user_login_callback::Method>),
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("http error: `{0:?}`")]
    Http(#[from] reqwest::Error),
    #[error("url parse error: `{0:?}`")]
    ParseError(#[from] url::ParseError),
    #[error("invalid id: `{0:?}`")]
    InvalidID(#[from] model::IdError),
    #[error("invalid media type (neither track or episode)")]
    InvalidMediaType,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("auth: {0}")]
    Auth(#[from] AuthError),
    #[error("config: {0}")]
    Config(#[from] ConfigError),
    #[error("bad url: {0}")]
    BadUrl(#[from] url::ParseError),
    #[error("api request error: {0}")]
    Api(#[from] ApiError),

    #[error("not found")]
    NotFound,

    #[error("search result is not of type `{0:?}`")]
    InvalidSearchResultType(model::SearchType),
    #[error("unknown spotify error: `{0:?}`")]
    Unknown(#[from] Box<dyn std::error::Error + Send + Sync>),
}

// #[derive(Debug, Error, Deserialize)]
// pub enum ApiError {
//     #[error("{status}: {message}")]
//     #[serde(alias = "error")]
//     Regular { status: u16, message: String },

//     #[error("{status} ({reason}): {message}")]
//     #[serde(alias = "error")]
//     Player {
//         status: u16,
//         message: String,
//         reason: String,
//     },
// }

// #[derive(Debug, Error)]
// pub enum ModelError {
//     #[error("json parse error: {0}")]
//     ParseJson(#[from] serde_json::Error),

//     #[error("input/output error: {0}")]
//     Io(#[from] std::io::Error),
// }

// #[derive(Debug, Error)]
// pub enum ClientError {
//     #[error("json parse error: {0}")]
//     ParseJson(#[from] serde_json::Error),

//     #[error("url parse error: {0}")]
//     ParseUrl(#[from] url::ParseError),

//     // Note that this type is boxed because its size might be very large in
//     // comparison to the rest. For more information visit:
//     // https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
//     #[error("http error: {0}")]
//     Http(Box<HttpError>),

//     #[error("input/output error: {0}")]
//     Io(#[from] std::io::Error),

//     // #[cfg(feature = "cli")]
//     // #[error("cli error: {0}")]
//     // Cli(String),
//     #[error("cache file error: {0}")]
//     CacheFile(String),

//     #[error("model error: {0}")]
//     Model(#[from] ModelError),
// }

// impl From<HttpError> for ClientError {
//     fn from(err: HttpError) -> Self {
//         ClientError::Http(Box::new(err))
//     }
// }

// pub type ApiResult<T> = Result<T, ApiError>;
// pub type ModelResult<T> = Result<T, ModelError>;
// pub type ClientResult<T> = Result<T, ClientError>;
