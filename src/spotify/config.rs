use crate::config::Persist;
use rspotify_model::Token;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub token: Option<Token>,
    /// The code verifier for the authentication process
    pub verifier: Option<String>,
    pub app_client_id: Option<String>,
    pub user_id: Option<String>,
}

impl Persist for Config {}
