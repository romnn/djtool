use djtool::config::ConfigError;
use crate::model;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing client_id")]
    MissingClientID(),
    #[error("missing user_id")]
    MissingUserID(),
    #[error("user must login at: {auth_url}")]
    RequireUserLogin { auth_url: reqwest::Url },
    #[error("auth method {0:?} unsupported")]
    Unsupported(Option<model::spotify_user_login_callback::Method>),
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("url parse error: {0}")]
    ParseError(#[from] url::ParseError),
    #[error("invalid id: {0}")]
    InvalidID(#[from] rspotify_model::IdError),
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
    InvalidSearchResultType(rspotify_model::SearchType),
    #[error("unknown spotify error: `{0:?}`")]
    Unknown(#[from] Box<dyn std::error::Error + Send + Sync>),
}
