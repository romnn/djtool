use anyhow::Result;
use async_stream::stream;
use futures::stream::Stream;
use futures::Future;
use super::model::Page;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

pub fn paginate<'a, T: 'a, Fut, Req: 'a>(
    req: Req,
    page_size: u32,
) -> impl Stream<Item = Result<T>> + 'a + Send
where
    T: Unpin + Send,
    Fut: Future<Output = Result<Page<T>>> + Send,
    Req: Fn(u32, u32) -> Fut + Send + Sync,
{
    let mut offset = 0;
    Box::pin(stream! {
        loop {
            let page = req(page_size, offset).await?;
            offset += page.items.len() as u32;
            for item in page.items {
                yield Ok(item);
            }
            if page.next.is_none() {
                break;
            }
        }
    })
}
