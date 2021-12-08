use super::model;
use super::Youtube;
use crate::utils;
use anyhow::Result;
use boa;
use futures_util::{stream, StreamExt};
use http::header::HeaderMap;
use lazy_static::lazy_static;
use rayon::prelude::*;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempdir::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};

impl Youtube {
    async fn fetch_player_config(&self, id: &String) -> Result<String> {
        let embed_url = format!("https://youtube.com/embed/{}?hl=en", id);
        let embed_body = self.client.get(embed_url).send().await?.text().await?;

        // example: /s/player/f676c671/player_ias.vflset/en_US/base.js
        lazy_static! {
            static ref BASEJS_PATTERN: Regex =
                Regex::new(r"(/s/player/\w+/player_ias.vflset/\w+/base.js)").unwrap();
        }
        let escaped_basejs_url: Vec<&str> = BASEJS_PATTERN
            .find_iter(&embed_body)
            .map(|m| m.as_str())
            .collect();
        // todo: error handling
        let escaped_basejs_url = escaped_basejs_url.first().unwrap();

        // if escapedBasejsURL == "" {
        // println!("playerConfig: {}", embedBody);
        // rrors.New("unable to find basejs URL in playerConfig")
        // TODO: return error here
        // }
        let basejs_url = format!("https://youtube.com{}", escaped_basejs_url);
        println!("basejs url: {}", basejs_url);
        self.client
            .get(basejs_url)
            .send()
            .await?
            .text()
            .await
            .map_err(|err| err.into())
    }

    async fn get_signature_timestamp(&self, id: &String) -> Result<String> {
        let basejs_body = self.fetch_player_config(id).await?;

        lazy_static! {
            static ref SIGNATURE_PATTERN: Regex =
                Regex::new(r"(?m)(?:^|,)(?:signatureTimestamp:)(\d+)").unwrap();
        }
        let result: Vec<&str> = SIGNATURE_PATTERN
            .captures_iter(&basejs_body)
            .map(|m| m.get(1).map(|c| c.as_str()))
            .filter_map(|m| m)
            .collect();
        // todo: error handling
        // ErrSignatureTimestampNotFound
        let result = result.first().unwrap().to_string();
        println!("signature timestamp: {:?}", result);
        Ok(result)
    }

    async fn video_data_by_innertube(&self, id: &String) -> Result<String> {
        let signature_ts = self.get_signature_timestamp(id).await?;
        let data = model::InnertubeRequest {
            video_id: id.to_string(),
            context: model::InnertubeContext {
                client: model::InnertubeClient {
                    hl: "en".to_string(),
                    gl: "US".to_string(),
                    // client_name: "WEB".to_string(),
                    client_name: "ANDROID".to_string(),
                    // client_version: "2.20210617.01.00".to_string(),
                    // client_version: "2.20210622.10.00".to_string(),
                    client_version: "16.20".to_string(),
                },
            },
            playback_context: model::PlaybackContext {
                content_playback_context: model::ContentPlaybackContext {
                    signature_timestamp: signature_ts,
                },
            },
        };

        let player_key = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";
        // .data(serde_json::to_string(&data)?)
        let player_url = format!(
            "https://www.youtube.com/youtubei/v1/player?key={}",
            player_key
        );
        println!("player_url: {}", player_url);
        let response = self
            .client
            .post(player_url)
            .json(&data)
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }

    pub async fn get_video(&self, id: &String) -> Result<model::Video> {
        let body = self.video_data_by_innertube(id).await?;
        let video_info: model::PlayerResponseData = serde_json::from_str(&body)?;
        println!("info: {:?}", video_info);
        if video_info
            .playability_status
            .as_ref()
            .and_then(|ps| ps.status.clone())
            == Some("LOGIN_REQUIRED".to_string())
        {
            if video_info
                .playability_status
                .as_ref()
                .and_then(|ps| ps.reason.clone())
                == Some("This video is private.".to_string())
            {
                // todo: return and error here
            }
            // todo: return login required error
        }

        if !video_info
            .playability_status
            .as_ref()
            .and_then(|ps| ps.playable_in_embed)
            .unwrap_or(false)
        {
            // todo: return error here
        }

        if video_info
            .playability_status
            .as_ref()
            .and_then(|ps| ps.status.clone())
            != Some("OK".to_string())
        {
            // todo: return error here
        }

        Ok(model::Video::from_player_response(video_info))
    }

    async fn decipher_url(&self, video_id: String, cipher: String) -> Result<String> {
        println!("cipher: {}", cipher);
        // let queryParams = url.ParseQuery(cipher)
        let parsed_url = reqwest::Url::parse(&format!("https://youtube.com?{}", cipher)).unwrap();
        let hash_query: HashMap<_, _> = parsed_url.query_pairs().into_owned().collect();
        println!("cipher: {:?}", hash_query);

        lazy_static! {
            static ref SIG_JS_PATTERNS: Vec<Regex> = vec![
                Regex::new(
                    r#"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\("#
                ).unwrap(),
                Regex::new(
                    r#"\b[a-zA-Z0-9]+\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\("#
                ).unwrap(),
                Regex::new(r#"\bm=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(h\.s\)\)"#).unwrap(),
                Regex::new(r#"\bc&&\(c=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(c\)\)"#).unwrap(),
                Regex::new(
                    r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\);[a-zA-Z0-9$]{2}\.[a-zA-Z0-9$]{2}\(a,\d+\)"#
                ).unwrap(),
                Regex::new(
                    r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#
                ).unwrap(),
                Regex::new(
                    r#"(?P<sig>[a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#
                ).unwrap(),
            ];
        }

        let player_config = self.fetch_player_config(&video_id).await?;
        let matches: Vec<String> = SIG_JS_PATTERNS
            .par_iter()
            .map(|pattern| {
                let test = pattern
                    .captures_iter(&player_config)
                    .map(|m| m.name("sig").map(|g| g.as_str().to_string()))
                    .filter_map(|m| m)
                    .collect::<Vec<String>>();
                test.first().map(|m| m.to_owned())
            })
            .filter_map(|m| m)
            .collect();

        let contents = std::fs::read_to_string("/Users/roman/Desktop/basejsBodyExample.js")
            .expect("Something went wrong reading the file");
        let js_code = boa::parse(&contents, false).unwrap();
        // let js_code = boa::parse(&player_config, false).unwrap();
        // let matches = matches.iter().collect();
        // let matches: Vec<_> = sig_js_patterns.matches(&player_config).into_iter().collect();
        println!("matches:  {:?}", matches);

        Ok("".to_string())
    }

    pub async fn get_stream_url(&self, video: &model::Video, format: &model::Format) -> Result<String> {
        if let Some(url) = &format.url {
            return Ok(url.to_string());
        }
        match &format.signature_cipher {
            Some(cipher) => Ok(self
                .decipher_url(video.id.clone().unwrap(), cipher.clone())
                .await?),
            None => panic!("no cipher"),
        }
    }
}
