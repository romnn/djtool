use super::model::Page;
use anyhow::Result;
use async_stream::stream;
use futures::stream::Stream;
use futures::Future;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

pub fn paginate<'a, T: 'a, Fut, Req: 'a>(
    req: Req,
) -> impl Stream<Item = Result<T>> + 'a + Send
where
    T: Unpin + Serialize + Send,
    Fut: Future<Output = Result<Page<T>>> + Send,
    Req: Fn(Option<String>) -> Fut + Send + Sync,
{
    let mut continuation: Option<String> = None;
    Box::pin(stream! {
        loop {
            let page = req(continuation).await?;
            continuation = page.continuation;
            for result in page.results.into_iter() {
                yield Ok(result);
            }
            if continuation.is_none() {
                break;
            }
        }
    })
}
