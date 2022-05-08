// use super::model;
// use super::Youtube;
// use crate::proto;
// use crate::sink;
// use anyhow::Result;
// use futures::stream::Stream;
// use futures_util::stream::{StreamExt, TryStreamExt};

// impl From<model::YoutubeVideo> for proto::djtool::Track {
//     fn from(video: model::YoutubeVideo) -> proto::djtool::Track {
//         proto::djtool::Track {
//             id: Some(proto::djtool::TrackId {
//                 source: proto::djtool::Service::Youtube as i32,
//                 id: video.video_id,
//                 // .map(|id| id.id().to_string())
//                 // .unwrap_or("unknown".to_string()),
//                 playlist_id: None, // unknown at this point
//             }),
//             name: video.title,
//             duration_secs: 0, // track.duration.as_secs(),
//             artwork: None,
//             preview: None,
//             artist: "".to_string(),
//             // artwork: {
//             //     let mut images = track
//             //         .album
//             //         .images
//             //         .into_iter()
//             //         .map(proto::djtool::Artwork::from)
//             //         .collect::<Vec<proto::djtool::Artwork>>();
//             //     images.sort_by(|b, a| (a.width * a.height).cmp(&(b.width * b.height)));
//             //     images.first().map(|a| a.to_owned())
//             // },
//             // preview: track
//             //     .preview_url
//             //     .map(|url| proto::djtool::TrackPreview { url }),
//             // artist: track
//             //     .artists
//             //     .into_iter()
//             //     .map(|a| a.name)
//             //     .collect::<Vec<String>>()
//             //     .join(", "),
//         }
//     }
// }

// impl Youtube {
    
//     pub async fn rank_results(&self) -> Result<()> {
//         Ok(())
//     }
// }
