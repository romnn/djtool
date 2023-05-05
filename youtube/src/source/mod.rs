use youtube3::{chrono, hyper, hyper_rustls, oauth2, Error, FieldMask, Result, YouTube};

pub async fn test() -> () {
    let result = hub
        .playlists()
        .list(&vec!["nonumy".into()])
        .page_token("sed")
        .on_behalf_of_content_owner_channel("kasd")
        .on_behalf_of_content_owner("Lorem")
        .mine(true)
        .max_results(10)
        .add_id("rebum.")
        .hl("tempor")
        .channel_id("dolore")
        .doit()
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
}
