//! OK.ru — embed page metadata with per-quality MP4 URLs.
//!
//! Also a Yandex Video preview upstream. Not related to [`super::okxxx`] (`ok.xxx`).

use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::model::{MediaKind, Stream};
use crate::quality;
use serde_json::Value;
use url::Url;

pub const REFERER: &str = "https://ok.ru/";
const USER_AGENT: &str = crate::client::BROWSER_UA;

pub fn matches_host(url: &Url) -> bool {
    url.host_str().is_some_and(|h| {
        matches!(
            h,
            "ok.ru" | "m.ok.ru" | "mobile.ok.ru" | "odnoklassniki.ru"
        ) || h.ends_with(".ok.ru")
    })
}

pub fn video_id(url: &Url) -> Option<String> {
    let path = url.path();
    for prefix in ["/video/", "/videoembed/"] {
        if let Some(id) = path.strip_prefix(prefix) {
            let id = id.trim_end_matches('/');
            if !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '-')
            {
                return Some(id.to_owned());
            }
        }
    }
    None
}

pub async fn extract(
    client: &reqwest::Client,
    url: &Url,
    title: &str,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !options
        .kinds
        .contains(&MediaKind::Video)
    {
        return Ok(Vec::new());
    }
    let _ = title;
    let id = video_id(url)
        .ok_or_else(|| Error::InvalidUrl("missing OK.ru video id".into()))?;
    let metadata = fetch_metadata(client, &id).await?;
    let streams = streams_from_metadata(&metadata)?;
    quality::pick_streams(streams, options.quality)
}

async fn fetch_metadata(
    client: &reqwest::Client,
    id: &str,
) -> Result<Value> {
    let html = client
        .get(format!("https://ok.ru/videoembed/{id}"))
        .header("Referer", REFERER)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let options = parse_data_options(&html)?;
    let flashvars = options
        .get("flashvars")
        .ok_or(Error::NoStreamsFound)?;
    if let Some(url) = flashvars
        .get("url")
        .and_then(Value::as_str)
        && options
            .get("isExternalPlayer")
            .and_then(Value::as_bool)
            == Some(true)
    {
        return Err(Error::InvalidUrl(format!(
            "external player: {url}"
        )));
    }
    if let Some(raw) = flashvars.get("metadata") {
        return Ok(match raw {
            | Value::String(s) => serde_json::from_str(s)
                .map_err(|e| Error::InvalidUrl(e.to_string()))?,
            | v => v.clone(),
        });
    }
    let metadata_url = flashvars
        .get("metadataUrl")
        .and_then(Value::as_str)
        .ok_or(Error::NoStreamsFound)?;
    Ok(client
        .get(metadata_url)
        .header("Referer", REFERER)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn parse_data_options(html: &str) -> Result<Value> {
    let marker = "data-options=\"";
    let start = html
        .find(marker)
        .ok_or(Error::NoStreamsFound)?
        + marker.len();
    let end = html[start..]
        .find('"')
        .ok_or(Error::NoStreamsFound)?
        + start;
    let raw = html[start..end].replace("&quot;", "\"");
    serde_json::from_str(&raw).map_err(|e| Error::InvalidUrl(e.to_string()))
}

fn streams_from_metadata(metadata: &Value) -> Result<Vec<Stream>> {
    let mut streams = Vec::new();
    if let Some(videos) = metadata
        .get("videos")
        .and_then(Value::as_array)
    {
        for video in videos {
            let Some(url) = video.get("url").and_then(Value::as_str) else {
                continue;
            };
            if video
                .get("disallowed")
                .and_then(Value::as_bool)
                == Some(true)
            {
                continue;
            }
            let name = video
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let height = ok_name_height(name);
            streams.push(Stream {
                url: Url::parse(url)
                    .map_err(|e| Error::InvalidUrl(e.to_string()))?,
                kind: MediaKind::Video,
                label: Some(format!("{height}p")),
                download_user_agent: Some(USER_AGENT),
                mux_audio: None,
                hls: false,
                download_referer: Some(REFERER),
            });
        }
    }
    if streams.is_empty() {
        return Err(Error::NoStreamsFound);
    }
    Ok(streams)
}

fn ok_name_height(name: &str) -> u32 {
    match name {
        | "full" => 1080,
        | "hd" => 720,
        | "sd" => 480,
        | "low" => 360,
        | "lowest" => 240,
        | "mobile" => 144,
        | _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_video_id() {
        let url = Url::parse("http://ok.ru/video/9729432226499").unwrap();
        assert_eq!(
            video_id(&url).as_deref(),
            Some("9729432226499")
        );
    }

    #[test]
    fn maps_quality_names() {
        assert_eq!(ok_name_height("full"), 1080);
        assert_eq!(ok_name_height("hd"), 720);
    }
}
