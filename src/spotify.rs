use super::serialization::{duration_second, space_separated_scopes};
use super::utils::{random_string, Alphanumeric, PKCECodeVerifier};
use base64;
use chrono::{DateTime, Duration, Utc};
use reqwest;
use reqwest::{header::HeaderMap, Error as HttpError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
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

/// Matches errors that are returned from the Spotfiy
/// API as part of the JSON response object.
#[derive(Debug, Error, Deserialize)]
pub enum ApiError {
    /// See [Error Object](https://developer.spotify.com/documentation/web-api/reference/#object-errorobject)
    #[error("{status}: {message}")]
    #[serde(alias = "error")]
    Regular { status: u16, message: String },

    /// See [Play Error Object](https://developer.spotify.com/documentation/web-api/reference/#object-playererrorobject)
    #[error("{status} ({reason}): {message}")]
    #[serde(alias = "error")]
    Player {
        status: u16,
        message: String,
        reason: String,
    },
}

/// Groups up the kinds of errors that may happen in this crate.
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

pub const DEFAULT_API_PREFIX: &str = "https://api.spotify.com/v1/";
pub const DEFAULT_CACHE_PATH: &str = ".spotify_token_cache.json";
pub const DEFAULT_PAGINATION_CHUNKS: u32 = 50;

// pub const ALPHANUM: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
// pub const PKCE_CODE_VERIFIER: &[u8] =
//     b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";

/// Struct to configure the Spotify client.
#[derive(Debug, Clone)]
pub struct Config {
    /// The Spotify API prefix, [FAULT_API_PREFIX by default.
    pub prefix: String,

    /// The cache file path, in case it's used. By default it's
    /// [FAULT_CACHE_PATH
    pub cache_path: PathBuf,

    /// The pagination chunk size used when performing automatically paginated
    /// requests, like [rtist_albums(crate::clients::BaseClient). This
    /// means that a request will be performed every items.
    /// By default this is [FAULT_PAGINATION_CHUNKS.
    ///
    /// Note that most endpoints set a maximum to the number of items per
    /// request, which most times is 50.
    pub pagination_chunks: u32,

    /// Whether or not to save the authentication token into a JSON file,
    /// then reread the token from JSON file when launching the program without
    /// following the full auth process again
    pub token_cached: bool,

    /// Whether or not to check if the token has expired when sending a
    /// request with credentials, and in that case, automatically refresh it.
    pub token_refreshing: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            prefix: String::from(DEFAULT_API_PREFIX),
            cache_path: PathBuf::from(DEFAULT_CACHE_PATH),
            pagination_chunks: DEFAULT_PAGINATION_CHUNKS,
            token_cached: false,
            token_refreshing: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Token {
    /// An access token that can be provided in subsequent calls
    pub access_token: String,
    /// The time period for which the access token is valid.
    #[serde(with = "duration_second")]
    pub expires_in: Duration,
    /// The valid time for which the access token is available represented
    /// in ISO 8601 combined date and time.
    pub expires_at: Option<DateTime<Utc>>,
    /// A token that can be sent to the Spotify Accounts service
    /// in place of an authorization code
    pub refresh_token: Option<String>,
    /// A list of [scopes](https://developer.spotify.com/documentation/general/guides/authorization/scopes/)
    /// which have been granted for this access token
    /// The token response from spotify is singular, hence the rename to scope
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
    /// Tries to initialize the token from a cache file.
    pub fn from_cache<T: AsRef<Path>>(path: T) -> ModelResult<Self> {
        let mut file = fs::File::open(path)?;
        let mut tok_str = String::new();
        file.read_to_string(&mut tok_str)?;
        let tok = serde_json::from_str::<Token>(&tok_str)?;

        Ok(tok)
    }

    /// Saves the token information into its cache file.
    pub fn write_cache<T: AsRef<Path>>(&self, path: T) -> ModelResult<()> {
        let token_info = serde_json::to_string(&self)?;

        let mut file = fs::OpenOptions::new().write(true).create(true).open(path)?;
        file.set_len(0)?;
        file.write_all(token_info.as_bytes())?;

        Ok(())
    }

    /// Check if the token is expired. It includes a margin of 10 seconds (which
    /// is how much a request would take in the worst case scenario).
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(true, |expiration| {
            Utc::now() + Duration::seconds(10) >= expiration
        })
    }

    /// Generates an HTTP token authorization header with proper formatting
    pub fn auth_headers(&self) -> HashMap<String, String> {
        let auth = "authorization".to_owned();
        let value = format!("Bearer {}", self.access_token);

        let mut headers = HashMap::new();
        headers.insert(auth, value);
        headers
    }
}

/// Simple client credentials object for Spotify.
#[derive(Debug, Clone, Default)]
pub struct Credentials {
    pub id: String,
    /// PKCE doesn't require a client secret
    pub secret: Option<String>,
}

impl Credentials {
    /// Initialization with both the client ID and the client secret
    pub fn new(id: &str, secret: &str) -> Self {
        Credentials {
            id: id.to_owned(),
            secret: Some(secret.to_owned()),
        }
    }

    /// Initialization with just the client ID
    pub fn new_pkce(id: &str) -> Self {
        Credentials {
            id: id.to_owned(),
            secret: None,
        }
    }

    /// Parses the credentials from the environment variables
    /// SPOTIFY_CLIENT_IDfile.
    // pub fn from_env() -> Option<Self> {
    //     #[cfg(feature = "env-file")]
    //     {
    //         dotenv::dotenv().ok();
    //     }

    //     Some(Credentials {
    //         id: env::var("RSPOTIFY_CLIENT_ID").ok()?,
    //         secret: env::var("RSPOTIFY_CLIENT_SECRET").ok(),
    //     })
    // }

    /// Generates an HTTP basic authorization header with proper formatting
    ///
    /// This will only work when the client secret is set to ption::Some
    pub fn auth_headers(&self) -> Option<HashMap<String, String>> {
        let auth = "authorization".to_owned();
        let value = format!("{}:{}", self.id, self.secret.as_ref()?);
        let value = format!("Basic {}", base64::encode(value));

        let mut headers = HashMap::new();
        headers.insert(auth, value);
        Some(headers)
    }
}

/// Structure that holds the required information for requests with OAuth.
#[derive(Debug, Clone)]
pub struct OAuth {
    pub redirect_uri: String,
    /// The state is generated by default, as suggested by the OAuth2 spec:
    /// [Cross-Site Request Forgery](https://tools.ietf.org/html/rfc6749#section-10.12)
    pub state: String,
    /// You could use macro [scopes!](crate::scopes) to build it at compile time easily
    pub scopes: HashSet<String>,
    pub proxies: Option<String>,
}

impl Default for OAuth {
    fn default() -> Self {
        OAuth {
            redirect_uri: String::new(),
            state: random_string(16, Alphanumeric), // ALPHANUM
            scopes: HashSet::new(),
            proxies: None,
        }
    }
}

// impl OAuth {
//     pub fn from_env(scopes: HashSet<String>) -> Option<Self> {
//         #[cfg(feature = "env-file")]
//         {
//             dotenv::dotenv().ok();
//         }

//         Some(OAuth {
//             scopes,
//             redirect_uri: env::var("RSPOTIFY_REDIRECT_URI").ok()?,
//             ..Default::default()
//         })
//     }
// }

#[derive(Clone, Debug, Default)]
pub struct AuthCodePkceSpotify {
    pub creds: Credentials,
    pub oauth: OAuth,
    pub config: Config,
    pub token: Arc<Mutex<Option<Token>>>,
    /// The code verifier for the authentication process
    pub verifier: Option<String>,
    // pub(in crate) http: HttpClient,
    pub client: Arc<reqwest::Client>,
    // pub http: HttpClient,
}

#[inline]
fn join_scopes(scopes: &HashSet<String>) -> String {
    scopes
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}

impl AuthCodePkceSpotify {
    pub fn new(creds: Credentials, oauth: OAuth) -> Self {
        AuthCodePkceSpotify {
            creds,
            oauth,
            ..Default::default()
        }
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

    async fn read_token_cache(&mut self, allow_expired: bool) -> ClientResult<Option<Token>> {
        if !self.config.token_cached {
            println!("Auth token cache read ignored (not configured)");
            return Ok(None);
        }

        println!("Reading auth token cache");
        let token = Token::from_cache(&self.config.cache_path)?;
        if !self.oauth.scopes.is_subset(&token.scopes) || (!allow_expired && token.is_expired()) {
            // Invalid token, since it doesn't have at least the currently
            // required scopes or it's expired.
            Ok(None)
        } else {
            Ok(Some(token))
        }
    }

    async fn write_token_cache(&self) -> ClientResult<()> {
        if !self.config.token_cached {
            println!("Token cache write ignored (not configured)");
            return Ok(());
        }

        println!("Writing token cache");
        if let Some(token) = self.token.lock().await.as_ref() {
            token.write_cache(&self.config.cache_path)?;
        }

        Ok(())
    }

    async fn request_token(&mut self, code: &str) -> ClientResult<()> {
        println!("Requesting PKCE Auth Code token");

        let verifier = self.verifier.as_ref().expect("Unknown code verifier");
        // let mut data = Form::new();
        // data.insert(params::CLIENT_ID, &self.creds.id);
        // data.insert(params::GRANT_TYPE, params::GRANT_TYPE_AUTH_CODE);
        // data.insert(params::CODE, code);
        // data.insert(params::REDIRECT_URI, &self.oauth.redirect_uri);
        // data.insert(params::CODE_VERIFIER, verifier);

        let mut data = HashMap::new();
        // todo: convert to serde struct
        data.insert(param::CLIENT_ID, self.creds.id.to_owned());
        data.insert(param::GRANT_TYPE, param::GRANT_TYPE_AUTH_CODE.to_string());
        data.insert(param::CODE, code.to_string());
        data.insert(param::REDIRECT_URI, self.oauth.redirect_uri.to_owned());
        data.insert(param::CODE_VERIFIER, verifier.to_owned());

        let new_token = self.fetch_access_token(&data).await?;
        // *self.token.lock().await.unwrap() = Some(token);
        let mut token = self.token.lock().await.as_ref();
        token = Some(&new_token);

        self.write_token_cache().await
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

    pub async fn load_token(&mut self, auth_url: &str) -> ClientResult<()> {
        match self.read_token_cache(true).await {
            Ok(Some(new_token)) => {
                let expired = new_token.is_expired();

                // Load token into client regardless of whether it's expired o
                // not, since it will be refreshed later anyway.
                // *self.token.lock().await.unwrap() = Some(new_token);
                let mut token = self.token.lock().await.as_ref();
                token = Some(&new_token);

                if expired {
                    // Ensure that we actually got a token from the refetch
                    match self.refetch_token().await? {
                        Some(refreshed_token) => {
                            println!("Successfully refreshed expired token from token cache");
                            let mut token = self.token.lock().await.as_ref();
                            token = Some(&refreshed_token);
                            // *self.token.lock().await.unwrap() = Some(refreshed_token)
                        }
                        // If not, prompt the user for it
                        None => {
                            println!("Unable to refresh expired token from token cache");
                            let code = self.get_code_from_user(auth_url)?;
                            // let code = "test";
                            self.request_token(&code).await?;
                        }
                    }
                }
            }
            // Otherwise following the usual procedure to get the token.
            _ => {
                let code = self.get_code_from_user(auth_url)?;
                // let code = "test";
                self.request_token(&code).await?;
            }
        }

        self.write_token_cache().await
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
        match self.token.lock().await.as_ref() {
            // .deref() {
            // .as_ref() {
            Some(Token {
                refresh_token: Some(refresh_token),
                ..
            }) => {
                // let mut data = Form::new();
                // data.insert(params::GRANT_TYPE, params::GRANT_TYPE_REFRESH_TOKEN);
                // data.insert(params::REFRESH_TOKEN, refresh_token);
                // data.insert(params::CLIENT_ID, &self.creds.id);
                let mut data = HashMap::new();
                data.insert(
                    param::GRANT_TYPE,
                    param::GRANT_TYPE_REFRESH_TOKEN.to_string(),
                );
                data.insert(param::REFRESH_TOKEN, refresh_token.to_string());
                data.insert(param::CLIENT_ID, self.creds.id.to_owned());

                // let mut token = self.fetch_access_token(&data, None).await?;
                let mut token = self.fetch_access_token(&data).await?;
                token.refresh_token = Some(refresh_token.to_string());
                Ok(Some(token))
                // Ok(None)
            }
            _ => Ok(None),
        }
    }
}
