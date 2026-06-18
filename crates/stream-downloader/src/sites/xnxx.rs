//! XNXX / XVideos — `html5player.setVideoUrl*` embed (shared player stack).

use crate::client;
use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::hls;
use crate::model::{FetchedPage, MediaKind, Stream};
use crate::quality;
use url::Url;

const USER_AGENT: &str = crate::client::BROWSER_UA;

const MP4_METHODS: &[&str] = &["setVideoUrlLow", "setVideoUrlHigh", "setVideoUrlHD"];

pub fn matches_host(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        let h = host.strip_prefix("www.").unwrap_or(host);
        h.contains("xnxx") || h.ends_with("xvideos.com") || h.ends_with(".xvideos.com")
    })
}

pub fn is_video_page(url: &Url) -> bool {
    let path = url.path();
    path.starts_with("/video-")
        || path.starts_with("/video.")
        || path
            .strip_prefix("/video")
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !options.kinds.contains(&MediaKind::Video) || !is_video_page(&page.info.url) {
        return Ok(Vec::new());
    }
    let referer = page.info.url.as_str();
    let streams = all_streams(client, &page.html, referer).await?;
    quality::pick_streams(streams, options.quality)
}

/// Re-fetch the page so signed CDN URLs are fresh before download.
pub async fn refresh_stream(
    client: &reqwest::Client,
    page: &FetchedPage,
    stream: &Stream,
) -> Result<Stream> {
    if !stream
        .url
        .host_str()
        .is_some_and(|h| h.contains("xnxx-cdn.com"))
    {
        return Ok(stream.clone());
    }
    let height = quality::height_hint(stream);
    let referer = page.info.url.as_str();
    let html = client::browser_get(client, page.info.url.clone(), referer)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let streams = all_streams(client, &html, referer).await?;
    streams
        .iter()
        .find(|s| quality::height_hint(s) == height && s.hls == stream.hls)
        .cloned()
        .or_else(|| {
            streams
                .iter()
                .max_by_key(|s| quality::height_hint(s))
                .cloned()
        })
        .ok_or(Error::NoStreamsFound)
}

async fn all_streams(client: &reqwest::Client, html: &str, referer: &str) -> Result<Vec<Stream>> {
    let mut streams = Vec::new();
    for method in MP4_METHODS {
        let Some(url) = parse_player_url(html, method) else {
            continue;
        };
        let height = mp4_height(&url).unwrap_or_else(|| fallback_mp4_height(method));
        streams.push(mp4_stream(&url, height)?);
    }
    if let Some(hls_url) = parse_player_url(html, "setVideoHLS") {
        streams.extend(hls_streams(client, &hls_url, referer).await?);
    }
    if streams.is_empty() {
        Err(Error::NoStreamsFound)
    } else {
        Ok(streams)
    }
}

fn mp4_stream(url: &str, height: u32) -> Result<Stream> {
    Ok(Stream {
        url: Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?,
        kind: MediaKind::Video,
        label: Some(format!("{height}p")),
        download_user_agent: Some(USER_AGENT),
        mux_audio: None,
        hls: false,
        download_referer: None,
    })
}

async fn hls_streams(
    client: &reqwest::Client,
    master_url: &str,
    referer: &str,
) -> Result<Vec<Stream>> {
    let master = Url::parse(master_url).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    let body = client::browser_get(client, master.clone(), referer)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let base = master
        .join("./")
        .map_err(|e| Error::InvalidUrl(e.to_string()))?;
    Ok(hls::parse_master_with_base(&body, Some(&base))?
        .into_iter()
        .map(|v| Stream {
            url: v.url,
            kind: MediaKind::Video,
            label: Some(format!("{}p", v.height)),
            download_user_agent: Some(USER_AGENT),
            mux_audio: None,
            hls: true,
            download_referer: None,
        })
        .collect())
}

fn parse_player_url(html: &str, method: &str) -> Option<String> {
    let needle = format!("html5player.{method}('");
    let start = html.find(&needle)? + needle.len();
    let end = html[start..].find('\'')?;
    Some(html[start..start + end].to_string())
}

fn mp4_height(url: &str) -> Option<u32> {
    let marker = "video_";
    let rest = url.split(marker).nth(1)?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn fallback_mp4_height(method: &str) -> u32 {
    match method {
        "setVideoUrlLow" => 240,
        "setVideoUrlHigh" => 360,
        "setVideoUrlHD" => 720,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_xnxx_and_xvideos_hosts() {
        let xnxx = Url::parse("https://www.xnxx.com/video-1cpbmdea/wild_gangbang").unwrap();
        assert!(matches_host(&xnxx));
        let xv = Url::parse("https://www.xvideos.com/video.ufkthho1234/test").unwrap();
        assert!(matches_host(&xv));
        let other = Url::parse("https://example.com/video-1").unwrap();
        assert!(!matches_host(&other));
    }

    #[test]
    fn detects_video_pages() {
        let xnxx = Url::parse("https://www.xnxx.com/video-1cpbmdea/wild_gangbang").unwrap();
        assert!(is_video_page(&xnxx));
        let xv = Url::parse("https://www.xvideos.com/video.ufkthho1234/test").unwrap();
        assert!(is_video_page(&xv));
        let home = Url::parse("https://www.xnxx.com/").unwrap();
        assert!(!is_video_page(&home));
    }

    #[test]
    fn parses_player_urls_from_fixture() {
        let html = include_str!("../../tests/fixtures/xnxx_player.html");
        assert_eq!(
            parse_player_url(html, "setVideoUrlLow").as_deref(),
            Some("https://mp4-gcore.xnxx-cdn.com/vid/0/video_240p.mp4?secure=abc")
        );
        assert_eq!(
            parse_player_url(html, "setVideoHLS").as_deref(),
            Some("https://hls-gcore.xnxx-cdn.com/token/vid/0/hls.m3u8")
        );
    }

    #[test]
    fn reads_height_from_mp4_filename() {
        assert_eq!(
            mp4_height("https://cdn.example.com/video_1080p.mp4?secure=x"),
            Some(1080)
        );
        assert_eq!(mp4_height("https://cdn.example.com/no-height.mp4"), None);
    }

    #[test]
    fn builds_mp4_streams_from_fixture() {
        let html = include_str!("../../tests/fixtures/xnxx_player.html");
        let streams: Vec<Stream> = MP4_METHODS
            .iter()
            .filter_map(|method| {
                let url = parse_player_url(html, method)?;
                let height = mp4_height(&url).unwrap_or_else(|| fallback_mp4_height(method));
                mp4_stream(&url, height).ok()
            })
            .collect();
        assert_eq!(streams.len(), 2);
        assert!(streams.iter().all(|s| !s.hls));
        assert_eq!(quality::height_hint(&streams[0]), 240);
        assert_eq!(quality::height_hint(&streams[1]), 360);
    }
}
