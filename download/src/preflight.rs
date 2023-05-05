use super::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreflightResult {
    pub content_len: Option<u64>,
    pub content_disposition_name: Option<String>,
    pub rangeable: bool,
}

pub async fn preflight(
    client: &reqwest::Client,
    url: reqwest::Url,
) -> Result<PreflightResult, Error> {
    let response = client
        .get(url)
        .header("Range", "bytes=0-0")
        .send()
        .await?
        .error_for_status()?;

    let headers = response.headers();
    let mut rangeable = false;
    let mut content_len = response.content_length();

    let content_disposition_name = headers
        .get("content-disposition")
        .and_then(|val| val.to_str().map(|s| s.to_string()).ok());

    if let Some(content_range) = headers
        .get("content-range")
        .and_then(|val| val.to_str().ok())
        .filter(|val| !val.is_empty())
    {
        if matches!(content_len, Some(1) | None) {
            let range_parts: Vec<&str> = content_range.split("/").collect();
            if range_parts.len() == 2 {
                content_len = range_parts[1].parse::<u64>().ok();
                rangeable = true;
            }
        }
    }
    Ok(PreflightResult {
        content_len,
        content_disposition_name,
        rangeable,
    })
}
