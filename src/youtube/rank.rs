use super::model;
use super::Youtube;
use crate::proto;
use crate::sink::Method;
use anyhow::Result;
use futures::stream::Stream;
use futures_util::stream::{StreamExt, TryStreamExt};

impl Youtube {
    pub async fn find_best_video(
        &self,
        track: &proto::djtool::Track,
        method: Method,
    ) -> Result<String> {
        let query = format!("{} {}", track.name, track.artist);
        // println!("youtube query: {}", query);
        let search_results = self
            .search_stream(query)
            .filter_map(|video: Result<model::YoutubeVideo>| {
                // test
                async move { video.ok() }
            })
            .take(10)
            .collect::<Vec<model::YoutubeVideo>>()
            .await;

        // println!("youtube search results : {:?}", search_results);
        let first_hit = search_results
            .first()
            .ok_or(anyhow::anyhow!("no results"))?;
        // println!("youtube first hit: {:?}", first_hit);

        Ok(first_hit.video_id.to_owned())
    }

    pub async fn rank_results(&self) -> Result<()> {
        Ok(())
    }
}
