use super::model;
use super::Youtube;
use crate::utils;
use anyhow::Result;
use reqwest;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[macro_export]
macro_rules! get {
    ( $val:expr, $( $x:expr ),* ) => {
        {
            let mut val: Option<&Value> = Some($val);
            $(
                val = val.and_then(|v| v.get($x));
            )*
            val
        }
    };
}

fn find<'a>(val: &'a Value, pattern: Box<Vec<&dyn serde_json::value::Index>>) -> Vec<&'a Value> {
    let mut matches: Vec<&'a Value> = Vec::new();
    let mut stack: Vec<&'a Value> = Vec::new();
    stack.push(val);
    while !stack.is_empty() {
        let current = stack.pop().unwrap();
        // check if it matches the pattern
        if let Some(v) = {
            let mut val: Option<&Value> = Some(current);
            for p in pattern.iter() {
                val = val.and_then(|v| v.get(p));
            }
            val
        } {
            matches.push(v);
        }
        match current {
            Value::Array(arr) => {
                for v in arr {
                    stack.push(v);
                }
            }
            Value::Object(obj) => {
                for v in obj.values() {
                    stack.push(v);
                }
            }
            _ => {
                // null, bool, number and string
            }
        }
    }
    matches
}

impl Youtube {
    pub fn api_headers(&self, client: Option<&model::Innertube>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        let cfg = client.unwrap_or(&model::Innertube::Web).config();
        for pair in vec![
            (
                HeaderName::from_lowercase(b"x-youtube-client-name").ok(),
                Some(HeaderValue::from(cfg.context_client_name)),
            ),
            (
                HeaderName::from_lowercase(b"x-youtube-client-version").ok(),
                HeaderValue::from_str(cfg.client_version).ok(),
            ),
            (
                HeaderName::from_lowercase(b"origin").ok(),
                HeaderValue::from_str(&format!("https://{}", cfg.host)).ok(),
            ),
        ] {
            if let (Some(name), Some(value)) = pair {
                headers.insert(name, value);
            }
        }
        // headers.insert(HeaderName::from_static("X-Youtube-Identity-Token"), bearer);
        // headers.insert(HeaderName::from_static("X-Goog-PageId"), bearer);
        // headers.insert(HeaderName::from_static("X-Goog-Visitor-Id"), bearer);
        headers
    }

    pub async fn get_search_response(
        &self,
        search_query: String,
        client: Option<model::Innertube>,
    ) -> Result<Value> {
        let innertube = client.unwrap_or(model::Innertube::Web);
        let innertube_config = innertube.config();
        let url = reqwest::Url::parse(&format!(
            "https://{}/youtubei/v1/search",
            innertube_config.host
        ))?;

        let headers = self.api_headers(Some(&innertube));
        let data = json!({
            "context": innertube_config.context(),
            "query": search_query,
            "params": "EgIQAQ%3D%3D"
        });
        let query = json!({"key": innertube_config.api_key });
        let response: Value = self
            .client
            .post(url)
            .json(&data)
            .headers(headers)
            .query(&query)
            .send()
            .await?
            .json()
            .await?;
        Ok(response)
    }

    pub async fn search(&self, search_query: String) -> Result<model::SearchResultPage> {
        let search_response = self.get_search_response(search_query, None).await?;
        let slr_contents = vec![
            get!(
                &search_response,
                "contents",
                "twoColumnSearchResultsRenderer",
                "primaryContents",
                "sectionListRenderer",
                "contents"
            ),
            get!(
                &search_response,
                "onResponseReceivedCommands",
                0,
                "appendContinuationItemsAction",
                "continuationItems"
            ),
        ];
        let slr_contents = slr_contents
            .into_iter()
            .filter_map(|x| x)
            .collect::<Vec<&Value>>();
        let slr_contents = slr_contents.first().unwrap().to_owned().to_owned();
        let parsed: model::SearchResultPage =
            serde_json::from_value(json!({ "results": slr_contents }))?;
        Ok(parsed)
    }
}
