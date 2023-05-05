#![allow(warnings)]

pub mod auth;
pub mod error;
pub mod config;
pub use rspotify_model as spotify_model;

use std::sync::Arc;

#[derive(Clone)]
pub struct Spotify {
    pub authenticator: Arc<Box<dyn auth::Authenticator + Send + Sync>>,
    pub client: Arc<reqwest::Client>,
}
