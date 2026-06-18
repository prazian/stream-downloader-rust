//! Vimeo — macOS app API (`api.vimeo.com`) for progressive MP4 + HLS links.

use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::model::{FetchedPage, Stream};
use crate::quality;
use crate::sites::common::{self, parse_url, video_stream, wants_video};
use serde::Deserialize;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use url::Url;

pub const REFERER: &str = "https://vimeo.com/";

const MACOS_AUTH: &str = "NDc1N2JlN2Y5ZjZmMjU3NzE3NTRkZTg1NmY2YzU2MTI0OTFlNjJiYjpwVUNDWUlBZmZqSHhQcndBYWxGMzgyYys2NkN5d1JrREJZZXdPcEdsU05tdjFlVVo2aE1lYk9GcWE3ZW9KVldlYnFlOWh5Vno5UWtpUGJ5empYZFBpYkFwV0FFTnB5VWV4ZEh3aHZnRUNEL0VySnBzTmFraDdNbS9nMXhWanhIcw==";
const MACOS_UA: &str = "Vimeo/1.6.3 (com.vimeo.mac; build:251121.142637.0; macOS 13.7.8) Alamofire/5.9.0 VimeoNetworking/5.0.0";
const API_ACCEPT: &str = "application/vnd.vimeo.*+json; version=3.4.10";
const VIDEO_FIELDS: &str = "files,play";

static TOKEN: OnceLock<Mutex<Option<CachedToken>>> = OnceLock::new();

struct CachedToken {
    value: String,
    expires_at: Instant,
}

pub fn matches_host(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|h| h == "vimeo.com" || h.ends_with(".vimeo.com") || h == "player.vimeo.com")
}

pub fn video_id(url: &Url) -> Option<String> {
    if url.host_str() == Some("player.vimeo.com") {
        let rest = url.path().strip_prefix("/video/")?;
        let id = rest.split(&['/', '?'][..]).next()?;
        return (!id.is_empty() && id.chars().all(|c| c.is_ascii_digit())).then(|| id.to_owned());
    }
    let mut segments = url.path_segments()?;
    let id = segments.next()?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(id.to_owned())
}

pub fn unlisted_hash(url: &Url) -> Option<String> {
    if let Some(h) = url
        .query_pairs()
        .find(|(k, _)| k == "h")
        .map(|(_, v)| v.into_owned())
        .filter(|h| !h.is_empty())
    {
        return Some(h);
    }
    let mut segments = url.path_segments()?;
    let id = segments.next()?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let hash = segments.next()?;
    if hash.is_empty() || hash.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(hash.to_owned())
}

pub fn player_hash_from_html(html: &str) -> Option<String> {
    const MARKERS: &[&str] = &[
        "player.vimeo.com/video/",
        "og:video:secure_url\" content=\"https://player.vimeo.com/video/",
        "og:video:url\" content=\"https://player.vimeo.com/video/",
    ];
    for marker in MARKERS {
        let Some(pos) = html.find(marker) else {
            continue;
        };
        let after = &html[pos + marker.len()..];
        let id_end = after
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after.len());
        let rest = &after[id_end..];
        if let Some(hpos) = rest.find("h=") {
            let hash = &rest[hpos + 2..];
            let end = hash
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(hash.len());
            if end > 0 {
                return Some(hash[..end].to_owned());
            }
        }
    }
    None
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !wants_video(options) {
        return Ok(Vec::new());
    }
    let id = video_id(&page.info.url)
        .ok_or_else(|| Error::InvalidUrl("missing Vimeo video id".into()))?;
    let hash = unlisted_hash(&page.info.url).or_else(|| player_hash_from_html(&page.html));
    let video = fetch_video(client, &id, hash.as_deref()).await?;
    let streams = streams_from_video(&video)?;
    quality::pick_streams_prefer_progressive(streams, options.quality)
}

pub async fn refresh_stream(
    client: &reqwest::Client,
    page: &FetchedPage,
    stream: &Stream,
) -> Result<Stream> {
    if !stream.url.host_str().is_some_and(|h| h.contains("vimeo")) {
        return Ok(stream.clone());
    }
    let id = video_id(&page.info.url)
        .ok_or_else(|| Error::InvalidUrl("missing Vimeo video id".into()))?;
    let hash = unlisted_hash(&page.info.url).or_else(|| player_hash_from_html(&page.html));
    let video = fetch_video(client, &id, hash.as_deref()).await?;
    let streams = streams_from_video(&video)?;
    common::match_refreshed(stream, &streams, false)
}

async fn fetch_video(client: &reqwest::Client, id: &str, hash: Option<&str>) -> Result<VimeoVideo> {
    let token = oauth_token(client).await?;
    let url = match hash {
        Some(h) => format!("https://api.vimeo.com/videos/{id}:{h}"),
        None => format!("https://api.vimeo.com/videos/{id}"),
    };
    client
        .get(url)
        .query(&[("fields", VIDEO_FIELDS)])
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", MACOS_UA)
        .header("Accept", API_ACCEPT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(|e| Error::InvalidUrl(e.to_string()))
}

async fn oauth_token(client: &reqwest::Client) -> Result<String> {
    if let Some(token) = cached_token() {
        return Ok(token);
    }
    let response: OAuthResponse = client
        .post("https://api.vimeo.com/oauth/authorize/client")
        .header("Authorization", format!("Basic {MACOS_AUTH}"))
        .header("User-Agent", MACOS_UA)
        .header("Accept", API_ACCEPT)
        .form(&[
            ("grant_type", "client_credentials"),
            ("scope", "public video_files"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .map_err(|e| Error::InvalidUrl(e.to_string()))?;
    store_token(&response);
    Ok(response.access_token)
}

fn cached_token() -> Option<String> {
    let lock = TOKEN.get_or_init(|| Mutex::new(None));
    let guard = lock.lock().expect("vimeo token mutex");
    guard.as_ref().and_then(|cached| {
        (cached.expires_at > Instant::now() + Duration::from_secs(120))
            .then(|| cached.value.clone())
    })
}

fn store_token(response: &OAuthResponse) {
    let lock = TOKEN.get_or_init(|| Mutex::new(None));
    let mut guard = lock.lock().expect("vimeo token mutex");
    *guard = Some(CachedToken {
        value: response.access_token.clone(),
        expires_at: Instant::now() + Duration::from_secs(response.expires_in.saturating_sub(60)),
    });
}

fn streams_from_video(video: &VimeoVideo) -> Result<Vec<Stream>> {
    let mut streams = Vec::new();
    if let Some(files) = &video.files {
        for file in files {
            if let Some(stream) = stream_from_file(file) {
                streams.push(stream?);
            }
        }
    }
    if let Some(play) = &video.play
        && let Some(progressive) = &play.progressive
    {
        for file in progressive {
            if let Some(stream) = stream_from_file(file) {
                streams.push(stream?);
            }
        }
    }
    if streams.is_empty() {
        return Err(Error::NoStreamsFound);
    }
    streams.sort_by_key(quality::height_hint);
    streams.dedup_by_key(|s| quality::height_hint(s));
    Ok(streams)
}

fn stream_from_file(file: &VimeoFile) -> Option<Result<Stream>> {
    let link = file.link.as_deref().filter(|u| !u.is_empty())?;
    if file.quality.as_deref() == Some("hls") && !link.contains(".m3u8") {
        return None;
    }
    let hls = link.contains(".m3u8");
    if !hls && file.kind.as_deref() != Some("video/mp4") && file.rendition.is_none() {
        return None;
    }
    let height = file
        .height
        .or_else(|| rendition_height(file.rendition.as_deref()?))
        .unwrap_or(0);
    if height == 0 {
        return None;
    }
    Some(parse_url(link).map(|url| video_stream(url, height, hls, Some(REFERER))))
}

fn rendition_height(rendition: &str) -> Option<u32> {
    rendition.trim_end_matches('p').parse().ok()
}

#[derive(Debug, Deserialize)]
struct OAuthResponse {
    access_token: String,
    #[serde(default = "default_token_ttl")]
    expires_in: u64,
}

fn default_token_ttl() -> u64 {
    3600
}

#[derive(Debug, Deserialize)]
struct VimeoVideo {
    files: Option<Vec<VimeoFile>>,
    play: Option<VimeoPlay>,
}

#[derive(Debug, Deserialize)]
struct VimeoPlay {
    progressive: Option<Vec<VimeoFile>>,
}

#[derive(Debug, Deserialize)]
struct VimeoFile {
    link: Option<String>,
    height: Option<u32>,
    rendition: Option<String>,
    quality: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_vimeo_hosts() {
        let url = Url::parse("https://vimeo.com/1181852916").unwrap();
        assert!(matches_host(&url));
        let url = Url::parse("https://player.vimeo.com/video/1181852916?h=abc").unwrap();
        assert!(matches_host(&url));
    }

    #[test]
    fn parses_video_id_and_hash() {
        let url = Url::parse("https://vimeo.com/1181852916?fl=wc").unwrap();
        assert_eq!(video_id(&url).as_deref(), Some("1181852916"));
        assert_eq!(unlisted_hash(&url), None);
        let url = Url::parse("https://vimeo.com/1181852916/edbe0f686b").unwrap();
        assert_eq!(unlisted_hash(&url).as_deref(), Some("edbe0f686b"));
        let url = Url::parse("https://player.vimeo.com/video/1181852916?h=edbe0f686b").unwrap();
        assert_eq!(video_id(&url).as_deref(), Some("1181852916"));
        assert_eq!(unlisted_hash(&url).as_deref(), Some("edbe0f686b"));
    }

    #[test]
    fn parses_player_hash_from_meta() {
        let html = r#"<meta property="og:video:secure_url" content="https://player.vimeo.com/video/1181852916?h=edbe0f686b">"#;
        assert_eq!(player_hash_from_html(html).as_deref(), Some("edbe0f686b"));
    }

    #[test]
    fn builds_streams_from_fixture() {
        let video: VimeoVideo =
            serde_json::from_str(include_str!("../../tests/fixtures/vimeo_files.json")).unwrap();
        let streams = streams_from_video(&video).unwrap();
        assert_eq!(streams.len(), 2);
        assert!(streams.iter().all(|s| !s.hls));
        assert_eq!(quality::height_hint(&streams[0]), 720);
        assert_eq!(quality::height_hint(&streams[1]), 1080);
    }
}
