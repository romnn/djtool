#![allow(warnings)]

use color_eyre::eyre;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();

    let client_id = std::env::var("INSTAGRAM_CLIENT_ID")?;
    let client_secret = std::env::var("INSTAGRAM_CLIENT_SECRET")?;
    let username = std::env::var("INSTAGRAM_USERNAME")?;
    let password = std::env::var("INSTAGRAM_PASSWORD")?;

    let redirect_url = "test";

    println!("username {username} with password {password}");
    println!("client {client_id} with password {client_secret}");

    let client = reqwest::Client::default();
    let scopes = vec![];
    let redirect_url = get_authorize_login_url(&client, &scopes, &client_id, &redirect_url).await?;

    dbg!(&redirect_url);
    Ok(())
}

const host: &str = "api.instagram.com";
const base_path: &str = "/v1";
const access_token_field: &str = "access_token";
const authorize_url: &str = "https://api.instagram.com/oauth/authorize";
const access_token_url: &str = "https://api.instagram.com/oauth/access_token";
const protocol: &str = "https";
const api_name: &str = "Instagram";

// pub async fn spotify_pkce_callback_handler(
//     query: spotify::auth::pkce::CallbackQuery,
//     tool: DjTool,
// ) -> std::result::Result<impl Reply, Infallible> {

async fn get_authorize_login_url(
    client: &reqwest::Client,
    scopes: &Vec<String>,
    client_id: &str,
    redirect_url: &str,
) -> eyre::Result<String> {
    // get_authorize_login_url
    let scopes = scopes.join(" ");
    let params: HashMap<_, _> = [
        ("client_id", client_id),
        ("response_type", "code"),
        ("redirect_uri", redirect_url),
        ("scope", &scopes),
    ]
    .into_iter()
    .collect();
    dbg!(&params);

    let response = client
        .get(authorize_url)
        // .headers(headers.unwrap_or(HeaderMap::new()))
        .form(&params)
        // .form(&params)
        .send()
        .await?;

    let status = response.status();
    assert_eq!(status, reqwest::StatusCode::OK);

    let body = response.text().await?;

    let spotify_pkce_callback = warp::get()
                .and(warp::path!("instagram" / "oauth" / "window"))
                // .and(warp::query::<spotify::auth::pkce::CallbackQuery>())
                // .and(warp::any().map(move || http_tool.clone()))
                // .and_then(spotify_pkce_callback_handler);
    .map(|| format!("Hello, {}!", "test"));
    dbg!(&body);

    // .map_err(|err| Error::Api(ApiError::Http(err)))?;

    // let data: serde_json::Value = response.json().await?;
    // redirected_to = response['content-location']
    // dbg!(&data);

    // return "%s?%s" % (self.api.authorize_url, url_params)

    // url = self._url_for_authorize(scope=scope)
    // response, content = http_object.request(url)
    // if response['status'] != '200':
    //     raise OAuth2AuthExchangeError("The server returned a non-200 response for URL %s" % url)
    // redirected_to = response['content-location']
    // return redirected_to
    Ok("".into())
}

// print ("Visit this page and authorize access in your browser: "+ redirect_uri)
//
// code = (str(input("Paste in code in query string after redirect: ").strip()))
//
// access_token = api.exchange_code_for_access_token(code)
