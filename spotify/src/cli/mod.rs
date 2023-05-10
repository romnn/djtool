mod options;
pub use options::Options;

use crate::auth::{Credentials, OAuth};
use std::{net::Ipv4Addr, sync::Arc};
use tokio::sync::broadcast;

const RedirectAddr: Ipv4Addr = Ipv4Addr::LOCALHOST;

#[derive(thiserror::Error, Debug)]
pub enum Error {}

pub struct CLI {}

impl CLI {
    pub fn parse<'a>(
        // tool: Arc<DjTool>,
        runtime: &'a tokio::runtime::Runtime,
        mut shutdown_tx: broadcast::Sender<bool>,
        options: Options,
    ) -> Result<(), Error> {
        let shutdown_tx_clone = shutdown_tx.clone();
        runtime.block_on(async move {
            let client_id = std::env::var("SPOTIFY_CLIENT_ID").unwrap();
            let creds = Credentials::pkce(client_id);
            let redirect_uri = format!(
                "http://{}/spotify/pkce/callback",
                RedirectAddr.to_string(),
                // tool.host.to_string(),
                // tool.port
            );

            let oauth = OAuth {
                redirect_uri,
                scopes: crate::scopes!("playlist-read-private"),
                ..Default::default()
            };
            dbg!(&oauth);
            // tool.connect_spotify(creds, oauth).await.unwrap();
            // println!("connected");
            // tool
        });

        Ok(())
    }
}
