//! Pornhub — `flashvars_*` embed JSON with `mediaDefinitions` (HLS + remote MP4).

use crate::client;
use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::extract::extract_json_object;
use crate::model::{FetchedPage, Stream};
use crate::quality;
use crate::sites::common::{self, parse_url, wants_video};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

pub fn matches_host(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        let h = host.strip_prefix("www.").unwrap_or(host);
        h.ends_with("pornhub.com")
            || h.ends_with(".pornhub.com")
            || h.ends_with("pornhub.org")
            || h.ends_with(".pornhub.org")
    })
}

pub fn view_key(url: &Url) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == "viewkey")
        .map(|(_, v)| v.into_owned())
        .filter(|k| !k.is_empty())
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !wants_video(options) {
        return Ok(Vec::new());
    }
    let _ = view_key(&page.info.url);
    let referer = page.info.url.as_str();
    let defs = parse_media_definitions(&page.html)?;
    let defs = resolve_remote(client, defs, referer).await?;
    let streams = streams_from_definitions(&defs)?;
    quality::pick_streams_prefer_progressive(streams, options.quality)
}

/// Re-sign CDN URLs from `get_media` right before download (tokens are IP/time-bound).
pub async fn refresh_stream(
    client: &reqwest::Client,
    page: &FetchedPage,
    stream: &Stream,
) -> Result<Stream> {
    if !stream
        .url
        .host_str()
        .is_some_and(|h| h.contains("phncdn.com"))
    {
        return Ok(stream.clone());
    }
    let referer = page.info.url.as_str();
    let defs = parse_media_definitions(&page.html)?;
    let defs = resolve_remote(client, defs, referer).await?;
    let streams = streams_from_definitions(&defs)?;
    common::match_refreshed(stream, &streams, false)
}

fn parse_media_definitions(html: &str) -> Result<Vec<MediaDefinition>> {
    let flashvars = parse_flashvars(html)?;
    flashvars
        .get("mediaDefinitions")
        .ok_or(Error::NoStreamsFound)
        .and_then(|value| {
            serde_json::from_value(value.clone()).map_err(|e| Error::InvalidUrl(e.to_string()))
        })
}

fn parse_flashvars(html: &str) -> Result<Value> {
    let marker = "var flashvars_";
    let pos = html.find(marker).ok_or(Error::NoStreamsFound)?;
    let after = &html[pos + marker.len()..];
    let brace = after.find('{').ok_or(Error::NoStreamsFound)?;
    let raw = extract_json_object(&after[brace..]).ok_or(Error::NoStreamsFound)?;
    serde_json::from_str(&raw).map_err(|e| Error::InvalidUrl(e.to_string()))
}

async fn resolve_remote(
    client: &reqwest::Client,
    defs: Vec<MediaDefinition>,
    referer: &str,
) -> Result<Vec<MediaDefinition>> {
    let Some(remote) = defs.iter().find(|d| d.remote == Some(true)) else {
        return Ok(defs);
    };
    client::browser_get(client, remote.video_url.trim(), referer)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(|e| Error::InvalidUrl(e.to_string()))
}

fn streams_from_definitions(defs: &[MediaDefinition]) -> Result<Vec<Stream>> {
    let mut streams = Vec::new();
    for def in defs {
        if def.remote == Some(true) {
            continue;
        }
        let url = unescape_url(&def.video_url);
        let is_hls = def.format.as_deref() == Some("hls") || url.contains(".m3u8");
        let height = def.height.unwrap_or_else(|| parse_quality(&def.quality));
        if height == 0 {
            continue;
        }
        streams.push(common::video_stream(parse_url(&url)?, height, is_hls, None));
    }
    if streams.is_empty() {
        Err(Error::NoStreamsFound)
    } else {
        Ok(streams)
    }
}

fn unescape_url(raw: &str) -> String {
    raw.replace("\\/", "/")
}

fn parse_quality(value: &Value) -> u32 {
    match value {
        Value::String(s) => s.parse().unwrap_or(0),
        Value::Number(n) => n.as_u64().unwrap_or(0) as u32,
        _ => 0,
    }
}

#[derive(Debug, Deserialize)]
struct MediaDefinition {
    height: Option<u32>,
    quality: Value,
    format: Option<String>,
    #[serde(rename = "videoUrl")]
    video_url: String,
    remote: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_pornhub_hosts() {
        let url = Url::parse("https://www.pornhub.com/view_video.php?viewkey=abc").unwrap();
        assert!(matches_host(&url));
        let url = Url::parse("https://de.pornhub.com/view_video.php?viewkey=abc").unwrap();
        assert!(matches_host(&url));
    }

    #[test]
    fn parses_view_key() {
        let url =
            Url::parse("https://www.pornhub.com/view_video.php?viewkey=660c3362e7ef7").unwrap();
        assert_eq!(view_key(&url).as_deref(), Some("660c3362e7ef7"));
    }

    #[test]
    fn parses_media_definitions_from_fixture() {
        let html = include_str!("../../tests/fixtures/pornhub_flashvars.html");
        let defs = parse_media_definitions(html).unwrap();
        assert!(defs.iter().any(|d| d.format.as_deref() == Some("hls")));
        assert!(defs.iter().any(|d| d.remote == Some(true)));
        let streams = streams_from_definitions(&defs).unwrap();
        assert!(streams.iter().any(|s| s.hls));
    }
}
