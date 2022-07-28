use crate::config::ConfigError;
use crate::config::Persist;
use crate::proto;
use crate::spotify;
use crate::spotify::auth::{join_scopes, Authenticator, Credentials, OAuth};
use crate::spotify::config::Config;
use crate::spotify::error::{ApiError, AuthError, Error};
use crate::spotify::model::Token;
use crate::utils::{random_string, Alphanumeric, PKCECodeVerifier};
use async_trait::async_trait;
use base64;
use chrono::{DateTime, Duration, Utc};
use reqwest;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
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

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub error: Option<String>,
    pub state: Option<String>,
    pub code: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PkceAuthenticator {
    pub creds: Credentials,
    pub oauth: OAuth,
    pub config_file: PathBuf,
    pub config: Arc<RwLock<Config>>,
    pub client: Arc<reqwest::Client>,
}

impl spotify::Spotify {
    pub async fn pkce<P: AsRef<Path> + Send + Sync>(
        config_dir: P,
        creds: Credentials,
        oauth: OAuth,
    ) -> Result<Self, Error> {
        let config_file = config_dir.as_ref().join("spotify-config.json");
        let config = Config::load(&config_file).await;
        let config = match config {
            Err(_) => {
                let empty = Config::default();
                // empty.save(&config_file).await.map_err(Error::from)?;
                // .map_err(|err| Error::Config())?;
                empty
            }
            Ok(config) => config,
        };
        let client = Arc::new(reqwest::Client::new());
        let mut authenticator = PkceAuthenticator {
            creds,
            oauth,
            config_file,
            config: Arc::new(RwLock::new(config)),
            client: client.clone(),
        };
        authenticator.read_config().await?;
        Ok(Self {
            authenticator: Arc::new(Box::new(authenticator)),
            client,
        })
    }
}

#[async_trait]
impl Authenticator for PkceAuthenticator {
    async fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let config = self.config.read().await;
        if let Some(bearer) = config
            .token
            .as_ref()
            .map(|token| format!("Bearer {}", token.access_token))
            .and_then(|value| HeaderValue::from_str(value.as_str()).ok())
        {
            headers.insert(HeaderName::from_static("authorization"), bearer);
        }
        headers
    }

    async fn reauthenticate(&self) -> std::result::Result<(), Error> {
        self.authenticate().await
    }

    async fn handle_user_login_callback(
        &self,
        data: proto::djtool::SpotifyUserLoginCallback,
    ) -> Result<(), Error> {
        match data.method {
            Some(proto::djtool::spotify_user_login_callback::Method::Pkce(
                proto::djtool::SpotifyUserLoginCallbackPkce { code, state },
            )) => {
                println!("handling received code: {} state: {}", code, state);
                if self.oauth.state != state {
                    panic!("state does not match");
                }
                self.request_token(&code).await?;
                println!("handled");
                Ok(())
            }
            other => Err(Error::Auth(AuthError::Unsupported(other))),
        }
    }
}

impl PkceAuthenticator {
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

    pub async fn get_authorize_url(
        &self,
        verifier_bytes: Option<usize>,
    ) -> std::result::Result<reqwest::Url, Error> {
        let scopes = join_scopes(&self.oauth.scopes);
        let verifier_bytes = verifier_bytes.unwrap_or(43);
        let (verifier, challenge) = self.generate_codes(verifier_bytes);

        // the verifier will be needed later when requesting the token
        {
            let mut config = self.config.write().await;
            config.verifier = Some(verifier);
        };

        let mut payload: HashMap<&str, &str> = HashMap::new();
        // todo: convert to serde struct
        payload.insert(param::CLIENT_ID, &self.creds.client_id);
        payload.insert(param::RESPONSE_TYPE, param::RESPONSE_TYPE_CODE);
        payload.insert(param::REDIRECT_URI, &self.oauth.redirect_uri);
        payload.insert(
            param::CODE_CHALLENGE_METHOD,
            param::CODE_CHALLENGE_METHOD_S256,
        );
        payload.insert(param::CODE_CHALLENGE, &challenge);
        payload.insert(param::STATE, &self.oauth.state);
        payload.insert(param::SCOPE, &scopes);

        Url::parse_with_params(api::AUTHORIZE, payload).map_err(Error::BadUrl)
    }

    async fn read_config(&self) -> Result<(), Error> {
        let new_config = Config::load(&self.config_file).await?;
        {
            let mut config = self.config.write().await;
            *config = new_config;
        };
        println!("read the config");
        Ok(())
    }

    async fn write_config(&self, config: &Config) -> std::result::Result<(), Error> {
        config.save(&self.config_file).await.map_err(Error::from)
        // .map_err(|err| Error::Config(err))
    }

    async fn request_token(&self, code: &str) -> std::result::Result<(), Error> {
        println!("Requesting PKCE Auth Code token");

        let verifier = {
            let config = self.config.read().await;
            config.verifier.to_owned().unwrap()
        };
        let mut data = HashMap::new();
        // todo: convert to serde struct
        data.insert(param::CLIENT_ID, self.creds.client_id.to_owned());
        data.insert(param::GRANT_TYPE, param::GRANT_TYPE_AUTH_CODE.to_string());
        data.insert(param::CODE, code.to_string());
        data.insert(param::REDIRECT_URI, self.oauth.redirect_uri.to_owned());
        data.insert(param::CODE_VERIFIER, verifier.to_owned());

        let new_token = self.fetch_access_token(&data, None).await?;
        // {
        let mut config = self.config.write().await;
        config.token = Some(new_token);
        self.write_config(&config).await
        // }

        // println!("writing config");
        // self.write_config().await
    }

    pub async fn authenticate(&self) -> std::result::Result<(), Error> {
        let cached_token = {
            let config = self.config.read().await;
            // println!("got the read lock");
            // config.token.to_owned()
            config.token.to_owned()
        };
        if let Some(cached_token) = cached_token {
            let sufficient = self.oauth.scopes.is_subset(&cached_token.scopes);
            let expired = cached_token.is_expired();

            if sufficient && !expired {
                return Ok(());
            }
            if expired {
                println!("attempt to refetch expired token");
                match self.refetch_token().await {
                    Ok(Some(refreshed_token)) => {
                        println!("successfully refreshed expired token from token cache");
                        // {
                        let mut config = self.config.write().await;
                        config.token = Some(refreshed_token);
                        println!("updated token");
                        return self.write_config(&config).await;
                        // };
                        // let test = self.write_config(&config).await;
                        // println!("wrote config");
                        // return test;
                    }
                    _ => {
                        println!("unable to refresh expired token from token cache");
                    }
                }
            }
        }
        // at this point we have to start over with the auth flow
        let mut auth_url = self.get_authorize_url(None).await?.to_owned();
        auth_url.query_pairs_mut().append_pair("target", "_blank");
        println!("auth url: {}", auth_url);
        Err(Error::Auth(AuthError::RequireUserLogin { auth_url }))
    }

    async fn fetch_access_token(
        &self,
        form: &HashMap<&str, String>,
        headers: Option<HeaderMap>,
    ) -> std::result::Result<Token, Error> {
        let response = self
            .client
            .post(api::TOKEN)
            .headers(headers.unwrap_or(HeaderMap::new()))
            .form(&form)
            .send()
            .await
            .map_err(|err| Error::Api(ApiError::Http(err)))?;
        // invalid grant refresh token revoked
        // println!(
        //     "fetch access token response: {}",
        //     response.text().await.unwrap()
        // );
        let mut token: Token = response
            .json()
            .await
            .map_err(|err| Error::Api(ApiError::Http(err)))?;
        // let mut token = Token::default();
        println!("{:?}: {:?}", api::TOKEN, token);
        token.expires_at = Utc::now().checked_add_signed(token.expires_in);
        Ok(token)
    }

    async fn refetch_token(&self) -> std::result::Result<Option<Token>, spotify::error::Error> {
        let current_token = {
            let config = self.config.read().await;
            config.token.to_owned()
        };
        match current_token {
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
                data.insert(param::CLIENT_ID, self.creds.client_id.to_owned());

                let mut token = self.fetch_access_token(&data, None).await?;
                token.refresh_token = Some(refresh_token.to_string());
                Ok(Some(token))
            }
            _ => Ok(None),
        }
    }
}
