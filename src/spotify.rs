use super::config::Persist;
use super::serialization::{duration_second, space_separated_scopes};
use super::utils::{random_string, Alphanumeric, PKCECodeVerifier};
use anyhow::Result;
use base64;
use chrono::{DateTime, Duration, Utc};
use reqwest;
use reqwest::{header::HeaderMap, Error as HttpError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use url::Url;
use webbrowser;

pub mod api {
    pub const AUTHORIZE: &str = "https://accounts.spotify.com/authorize";
    pub const TOKEN: &str = "https://accounts.spotify.com/api/token";
}

pub mod param {
    pub const CLIENT_ID: &str = "client_id";
    pub const CODE: &str = "code";
    pub const GRANT_TYPE: &str = "grant_type";
    pub const GRANT_TYPE_AUTH_CODE: &str = "authorization_code";
    pub const GRANT_TYPE_CLIENT_CREDS: &str = "client_credentials";
    pub const GRANT_TYPE_REFRESH_TOKEN: &str = "refresh_token";
    pub const REDIRECT_URI: &str = "redirect_uri";
    pub const REFRESH_TOKEN: &str = "refresh_token";
    pub const RESPONSE_TYPE_CODE: &str = "code";
    pub const RESPONSE_TYPE: &str = "response_type";
    pub const SCOPE: &str = "scope";
    pub const SHOW_DIALOG: &str = "show_dialog";
    pub const STATE: &str = "state";
    pub const CODE_CHALLENGE: &str = "code_challenge";
    pub const CODE_VERIFIER: &str = "code_verifier";
    pub const CODE_CHALLENGE_METHOD: &str = "code_challenge_method";
    pub const CODE_CHALLENGE_METHOD_S256: &str = "S256";
}

#[macro_export]
macro_rules! scopes {
    ($($key:expr),*) => {{
        let mut container = ::std::collections::HashSet::new();
        $(
            container.insert($key.to_owned());
        )*
        container
    }};
}

#[inline]
fn join_scopes(scopes: &HashSet<String>) -> String {
    scopes
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Error, Deserialize)]
pub enum ApiError {
    #[error("{status}: {message}")]
    #[serde(alias = "error")]
    Regular { status: u16, message: String },

    #[error("{status} ({reason}): {message}")]
    #[serde(alias = "error")]
    Player {
        status: u16,
        message: String,
        reason: String,
    },
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("json parse error: {0}")]
    ParseJson(#[from] serde_json::Error),

    #[error("input/output error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("json parse error: {0}")]
    ParseJson(#[from] serde_json::Error),

    #[error("url parse error: {0}")]
    ParseUrl(#[from] url::ParseError),

    // Note that this type is boxed because its size might be very large in
    // comparison to the rest. For more information visit:
    // https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant
    #[error("http error: {0}")]
    Http(Box<HttpError>),

    #[error("input/output error: {0}")]
    Io(#[from] std::io::Error),

    // #[cfg(feature = "cli")]
    // #[error("cli error: {0}")]
    // Cli(String),
    #[error("cache file error: {0}")]
    CacheFile(String),

    #[error("model error: {0}")]
    Model(#[from] ModelError),
}

impl From<HttpError> for ClientError {
    fn from(err: HttpError) -> Self {
        ClientError::Http(Box::new(err))
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
pub type ModelResult<T> = Result<T, ModelError>;
pub type ClientResult<T> = Result<T, ClientError>;

// pub const DEFAULT_API_PREFIX: &str = "https://api.spotify.com/v1/";
// pub const DEFAULT_CACHE_PATH: &str = ".spotify_token_cache.json";
pub const DEFAULT_PAGINATION_CHUNKS: u32 = 50;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // pub token: Arc<Mutex<Option<Option<Token>>>>,
    pub token: Option<Token>,
    pub app_client_id: Option<String>,
}

impl Persist for Config {
    // fn config_file_name() -> String {
    //     "spotify-config.json"
    // }
}

// impl Config {
//     pub fn from_cache<T: AsRef<Path>>(path: T) -> ModelResult<Self> {
//         let mut file = fs::File::open(path)?;
//         let mut tok_str = String::new();
//         file.read_to_string(&mut tok_str)?;
//         let tok = serde_json::from_str::<Token>(&tok_str)?;

//         Ok(tok)
//     }

//     pub fn write_cache<T: AsRef<Path>>(&self, path: T) -> ModelResult<()> {
//         let token_info = serde_json::to_string(&self)?;

//         let mut file = fs::OpenOptions::new().write(true).create(true).open(path)?;
//         file.set_len(0)?;
//         file.write_all(token_info.as_bytes())?;

//         Ok(())
//     }
// }

// impl Default for Config {
//     fn default() -> Self {
//         Config {
//             token: None,
//             app_client_id: None,
//             cache_dir: None,
//         }
//     }
// }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Token {
    pub access_token: String,
    #[serde(with = "duration_second")]
    pub expires_in: Duration,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_token: Option<String>,
    #[serde(default, with = "space_separated_scopes", rename = "scope")]
    pub scopes: HashSet<String>,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            access_token: String::new(),
            expires_in: Duration::seconds(0),
            expires_at: Some(Utc::now()),
            refresh_token: None,
            scopes: HashSet::new(),
        }
    }
}

impl Token {
    // pub fn from_cache<T: AsRef<Path>>(path: T) -> ModelResult<Self> {
    //     let mut file = fs::File::open(path)?;
    //     let mut tok_str = String::new();
    //     file.read_to_string(&mut tok_str)?;
    //     let tok = serde_json::from_str::<Token>(&tok_str)?;

    //     Ok(tok)
    // }

    // pub fn write_cache<T: AsRef<Path>>(&self, path: T) -> ModelResult<()> {
    //     let token_info = serde_json::to_string(&self)?;

    //     let mut file = fs::OpenOptions::new().write(true).create(true).open(path)?;
    //     file.set_len(0)?;
    //     file.write_all(token_info.as_bytes())?;

    //     Ok(())
    // }

    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(true, |expiration| {
            Utc::now() + Duration::seconds(10) >= expiration
        })
    }

    pub fn auth_headers(&self) -> HashMap<String, String> {
        let auth = "authorization".to_owned();
        let value = format!("Bearer {}", self.access_token);

        let mut headers = HashMap::new();
        headers.insert(auth, value);
        headers
    }
}

#[derive(Debug, Clone, Default)]
pub struct Credentials {
    pub id: String,
    // pub secret: Option<String>,
}

impl Credentials {
    // pub fn new(id: &str, secret: &str) -> Self {
    //     Credentials {
    //         id: id.to_owned(),
    //         secret: Some(secret.to_owned()),
    //     }
    // }

    pub fn new_pkce(id: &str) -> Self {
        Credentials {
            id: id.to_owned(),
            // secret: None,
        }
    }

    pub fn auth_headers(&self) -> Option<HashMap<String, String>> {
        let auth = "authorization".to_owned();
        let value = format!("{}:{}", self.id, ""); // self.secret.as_ref()?);
        let value = format!("Basic {}", base64::encode(value));

        let mut headers = HashMap::new();
        headers.insert(auth, value);
        Some(headers)
    }
}

#[derive(Debug, Clone)]
pub struct OAuth {
    pub redirect_uri: String,
    /// The state is generated by default, as suggested by the OAuth2 spec:
    /// [Cross-Site Request Forgery](https://tools.ietf.org/html/rfc6749#section-10.12)
    pub state: String,
    pub scopes: HashSet<String>,
    pub proxies: Option<String>,
}

impl Default for OAuth {
    fn default() -> Self {
        OAuth {
            redirect_uri: String::new(),
            state: random_string(16, Alphanumeric),
            scopes: HashSet::new(),
            proxies: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthCodePkceSpotify {
    pub creds: Credentials,
    pub oauth: OAuth,
    /// The code verifier for the authentication process
    pub verifier: Option<String>,
    // pub config_file: Option<PathBuf>,
    pub config_file: PathBuf,
    pub config: Arc<RwLock<Config>>,
    pub client: Arc<reqwest::Client>,
}

impl AuthCodePkceSpotify {
    pub async fn new<P: AsRef<Path> + Send + Sync>(
        config_dir: P,
        creds: Credentials,
        oauth: OAuth,
    ) -> Result<Self> {
        let config_file = config_dir.as_ref().join("spotify-config.json");
        let config = Config::load(&config_file).await?;
        let client = Self {
            creds,
            oauth,
            config_file,
            verifier: None,
            // config: Arc::new(RwLock::new(Config::default())),
            config: Arc::new(RwLock::new(config)),
            client: Arc::new(reqwest::Client::new()),
        };
        client.read_config().await?;
        Ok(client)
    }

    /// Generate the verifier code and the challenge code.
    fn generate_codes(&self, verifier_bytes: usize) -> (String, String) {
        println!("Generating PKCE codes");

        debug_assert!(verifier_bytes >= 43);
        debug_assert!(verifier_bytes <= 128);
        // The code verifier is just the randomly generated string.
        let verifier = random_string(verifier_bytes, PKCECodeVerifier);

        // The code challenge is the code verifier hashed with SHA256 and then
        // encoded with base64url.
        //
        // NOTE: base64url != base64; it uses a different set of characters. See
        // https://datatracker.ietf.org/doc/html/rfc4648#section-5 for more
        // information.
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge = hasher.finalize();

        let challenge = base64::encode_config(challenge, base64::URL_SAFE_NO_PAD);

        (verifier, challenge)
    }

    pub fn get_authorize_url(&mut self, verifier_bytes: Option<usize>) -> ClientResult<String> {
        println!("Building auth URL");

        let scopes = join_scopes(&self.oauth.scopes);
        let verifier_bytes = verifier_bytes.unwrap_or(43);
        let (verifier, challenge) = self.generate_codes(verifier_bytes);
        // The verifier will be needed later when requesting the token
        self.verifier = Some(verifier);

        let mut payload: HashMap<&str, &str> = HashMap::new();
        // todo: convert to serde struct
        payload.insert(param::CLIENT_ID, &self.creds.id);
        payload.insert(param::RESPONSE_TYPE, param::RESPONSE_TYPE_CODE);
        payload.insert(param::REDIRECT_URI, &self.oauth.redirect_uri);
        payload.insert(
            param::CODE_CHALLENGE_METHOD,
            param::CODE_CHALLENGE_METHOD_S256,
        );
        payload.insert(param::CODE_CHALLENGE, &challenge);
        payload.insert(param::STATE, &self.oauth.state);
        payload.insert(param::SCOPE, &scopes);

        let parsed = Url::parse_with_params(api::AUTHORIZE, payload)?;
        Ok(parsed.into())
    }

    // async fn read_token_cache(&mut self, allow_expired: bool) -> ClientResult<Option<Token>> {
    //     println!("Reading auth token cache");
    //     let token = Token::from_cache(&self.config.cache_path)?;
    //     if !self.oauth.scopes.is_subset(&token.scopes) || (!allow_expired && token.is_expired()) {
    //         // Invalid token, since it doesn't have at least the currently
    //         // required scopes or it's expired.
    //         Ok(None)
    //     } else {
    //         Ok(Some(token))
    //     }
    // }

    async fn read_config(&self) -> Result<()> {
        let new_config = Config::load(&self.config_file).await?;
        let mut config = self.config.write().await; // .deref();
        *config = new_config;
        Ok(())
    }

    async fn write_config(&self) -> Result<()> {
        let config = self.config.read().await;
        config.save(&self.config_file).await
    }
    // async fn write_token_cache(&self) -> ClientResult<()> {
    //     println!("Writing token cache");
    //     if let Some(token) = self.config.token.lock().await.as_ref() {
    //         token.write_cache(&self.config.cache_path)?;
    //     }

    //     Ok(())
    // }

    async fn request_token(&mut self, code: &str) -> Result<()> {
        println!("Requesting PKCE Auth Code token");

        let verifier = self.verifier.as_ref().expect("Unknown code verifier");

        let mut data = HashMap::new();
        // todo: convert to serde struct
        data.insert(param::CLIENT_ID, self.creds.id.to_owned());
        data.insert(param::GRANT_TYPE, param::GRANT_TYPE_AUTH_CODE.to_string());
        data.insert(param::CODE, code.to_string());
        data.insert(param::REDIRECT_URI, self.oauth.redirect_uri.to_owned());
        data.insert(param::CODE_VERIFIER, verifier.to_owned());

        let new_token = self.fetch_access_token(&data).await?;
        // *self.token.lock().await.unwrap() = Some(token);
        let mut config = self.config.write().await;
        config.token = Some(new_token);

        self.write_config().await
    }

    fn get_code_from_user(&self, auth_url: &str) -> ClientResult<String> {
        println!("Opening brower with auth URL");
        match webbrowser::open(auth_url) {
            Ok(_) => println!("Opened {} in your browser.", auth_url),
            Err(why) => eprintln!(
                "Error when trying to open an URL in your browser: {:?}. \
                 Please navigate here manually: {}",
                why, auth_url
            ),
        }

        println!("Prompting user for code");
        println!("Please enter the URL you were redirected to: ");
        // let mut input = String::new();
        // std::io::stdin().read_line(&mut input)?;
        // let code = self
        //     .parse_response_code(&input)
        //     .ok_or_else(|| ClientError::Cli("unable to parse the response code".to_string()))?;

        Ok("test".to_string())
    }

    pub async fn check_token(&mut self, auth_url: &str) -> Result<()> {
        if let Some(cached_token) = &self.config.read().await.token {
            let sufficient = self.oauth.scopes.is_subset(&cached_token.scopes);
            let expired = cached_token.is_expired();

            if sufficient && !expired {
                return Ok(());
            }
            if expired {
                match self.refetch_token().await? {
                    Some(refreshed_token) => {
                        println!("Successfully refreshed expired token from token cache");
                        let mut config = self.config.write().await;
                        config.token = Some(refreshed_token);
                        return self.write_config().await;
                    }
                    None => {
                        println!("Unable to refresh expired token from token cache");
                    }
                }
            }
        }
        let code = self.get_code_from_user(auth_url)?;
        self.request_token(&code).await?;
        self.write_config().await

        // if sufficient && !expired {
        //     token =
        // }
        // if ! || ) {
        // match self.read_token_cache(true).await {
        //     Ok(Some(new_token)) => {
        //         let expired = new_token.is_expired();

        //         // Load token into client regardless of whether it's expired o
        //         // not, since it will be refreshed later anyway.
        //         // *self.token.lock().await.unwrap() = Some(new_token);
        //         let mut token = self.config.token.lock().await.as_ref();
        //         token = Some(&new_token);

        //         if expired {
        //             // Ensure that we actually got a token from the refetch
        //             match self.refetch_token().await? {
        //                 Some(refreshed_token) => {
        //                     println!("Successfully refreshed expired token from token cache");
        //                     let mut token = self.config.token.lock().await.as_ref();
        //                     token = Some(&refreshed_token);
        //                     // *self.token.lock().await.unwrap() = Some(refreshed_token)
        //                 }
        //                 // If not, prompt the user for it
        //                 None => {
        //                     println!("Unable to refresh expired token from token cache");
        //                     let code = self.get_code_from_user(auth_url)?;
        //                     // let code = "test";
        //                     self.request_token(&code).await?;
        //                 }
        //             }
        //         }
        //     }
        //     // Otherwise following the usual procedure to get the token.
        //     _ => {
        //         let code = self.get_code_from_user(auth_url)?;
        //         // let code = "test";
        //         self.request_token(&code).await?;
        //     }
        // }
    }

    // fn get_token(&self) -> Arc<Mutex<Option<Token>>> {
    //     Arc::clone(&self.token)
    // }

    // fn get_creds(&self) -> &Credentials {
    //     &self.creds
    // }

    // fn get_config(&self) -> &Config {
    //     &self.config
    // }

    async fn fetch_access_token(&self, payload: &HashMap<&str, String>) -> ClientResult<Token> {
        let headers = HeaderMap::new();
        let mut response = self
            .client
            .post(api::TOKEN)
            .headers(headers)
            .json(&payload)
            .send()
            .await?
            .text()
            // .json()
            .await?;
        println!("{:?}: {:?}", api::TOKEN, payload);
        println!("{:?}: {:?}", api::TOKEN, response);
        // let response = self.post_form(api::TOKEN, headers, payload).await?;

        let token = Token::default();
        Ok(token)
        // let mut tok = serde_json::from_str::<Token>(&response)?;
        // token.expires_at = Utc::now().checked_add_signed(token.expires_in);
        // Ok(token)
    }

    async fn refetch_token(&self) -> ClientResult<Option<Token>> {
        match self.config.read().await.token.as_ref() {
            // .deref() {
            // .as_ref() {
            Some(Token {
                refresh_token: Some(refresh_token),
                ..
            }) => {
                let mut data = HashMap::new();
                data.insert(
                    param::GRANT_TYPE,
                    param::GRANT_TYPE_REFRESH_TOKEN.to_string(),
                );
                data.insert(param::REFRESH_TOKEN, refresh_token.to_string());
                data.insert(param::CLIENT_ID, self.creds.id.to_owned());

                let mut token = self.fetch_access_token(&data).await?;
                token.refresh_token = Some(refresh_token.to_string());
                Ok(Some(token))
            }
            _ => Ok(None),
        }
    }
}
