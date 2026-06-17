//! VK Video — `al_video.php` embed API (used by Yandex preview upstream).

use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::model::{MediaKind, Stream};
use crate::quality::Quality;
use url::Url;

pub const REFERER: &str = "https://vk.com/";
const USER_AGENT: &str = crate::client::BROWSER_UA;

pub fn matches_host(url: &Url) -> bool {
    url.host_str().is_some_and(|h| {
        matches!(h, "vk.com" | "vk.ru" | "m.vk.com" | "vkvideo.ru")
            || h.ends_with(".vk.com")
    })
}

pub fn video_key(url: &Url) -> Option<String> {
    if let Some(q) = url
        .query_pairs()
        .find(|(k, _)| k == "z" || k == "id")
        .map(|(_, v)| v.into_owned())
    {
        return normalize_key(&q);
    }
    let path = url.path();
    let rest = path.strip_prefix("/video")?;
    normalize_key(rest)
}

fn normalize_key(raw: &str) -> Option<String> {
    let key = raw.trim_start_matches('-');
    let (oid, id) = key.split_once('_')?;
    if oid.is_empty() || id.is_empty() {
        return None;
    }
    let oid = if oid.starts_with('-') {
        oid.to_owned()
    } else {
        format!("-{oid}")
    };
    Some(format!("{oid}_{id}"))
}

pub async fn extract(
    client: &reqwest::Client,
    url: &Url,
    _title: &str,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !options.kinds.contains(&MediaKind::Video) {
        return Ok(Vec::new());
    }
    let key = video_key(url).ok_or_else(|| Error::InvalidUrl("missing VK video id".into()))?;
    let text = fetch_player_payload(client, &key).await?;
    let streams = parse_sources(&text)?;
    pick_quality(streams, options.quality)
}

async fn fetch_player_payload(client: &reqwest::Client, key: &str) -> Result<String> {
    let bytes = client
        .post("https://vk.com/al_video.php")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Referer", REFERER)
        .form(&[
            ("act", "show"),
            ("al", "1"),
            ("autoplay", "1"),
            ("module", "embed"),
            ("video", key),
        ])
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(decode_vk_payload(&String::from_utf8_lossy(&bytes)))
}

fn decode_vk_payload(text: &str) -> String {
    text.replace("\\/", "/")
        .replace("\\\"", "\"")
        .replace("&amp;", "&")
}

fn parse_sources(text: &str) -> Result<Vec<Stream>> {
    let mut streams = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find("https://") {
        rest = &rest[idx..];
        let end = rest[8..]
            .find(|c: char| c == '"' || c == '\'' || c == '<' || c.is_whitespace())
            .map(|i| i + 8)
            .unwrap_or(rest.len());
        let url = &rest[..end];
        if url.contains("getVideoPreview") {
            rest = &rest[end..];
            continue;
        }
        let is_hls = url.contains(".m3u8");
        let is_mp4 = url.contains("vkvd") && url.contains("okcdn.ru") && !is_hls;
        if is_hls || is_mp4 {
            let height = vk_type_height(url).unwrap_or(if is_hls { 1080 } else { 720 });
            streams.push(Stream {
                url: Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?,
                kind: MediaKind::Video,
                label: Some(format!("{height}p")),
                download_user_agent: Some(USER_AGENT),
                mux_audio: None,
                hls: is_hls,
                download_referer: Some(REFERER),
            });
        }
        rest = &rest[end..];
    }
    if streams.is_empty() {
        return Err(Error::NoStreamsFound);
    }
    Ok(streams)
}

fn vk_type_height(url: &str) -> Option<u32> {
    let t = url.split("type=").nth(1)?.chars().next()?.to_digit(10)?;
    Some(match t {
        5..=7 => 1080,
        3 => 720,
        2 => 480,
        1 => 360,
        0 => 240,
        4 => 144,
        _ => 0,
    })
}

fn pick_quality(mut streams: Vec<Stream>, quality: Quality) -> Result<Vec<Stream>> {
    streams.sort_by_key(|b| (b.hls, std::cmp::Reverse(crate::quality::height_hint(b))));
    Ok(match quality {
        Quality::All => streams,
        Quality::Best => vec![streams.remove(0)],
        Quality::Height(h) => {
            let exact: Vec<_> = streams
                .iter()
                .filter(|s| crate::quality::height_hint(s) == h)
                .cloned()
                .collect();
            if !exact.is_empty() {
                return Ok(exact);
            }
            let fallback: Vec<_> = streams
                .into_iter()
                .filter(|s| crate::quality::height_hint(s) <= h)
                .take(1)
                .collect();
            if fallback.is_empty() {
                return Err(Error::NoStreamsFound);
            }
            fallback
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_video_keys() {
        let url = Url::parse("http://vk.com/video-223140511_456239655").unwrap();
        assert_eq!(video_key(&url).as_deref(), Some("-223140511_456239655"));
    }

    #[test]
    fn parses_sources_from_fixture() {
        let html = decode_vk_payload(include_str!("../../tests/fixtures/vk_al_video.html"));
        let streams = parse_sources(&html).unwrap();
        assert!(streams.iter().any(|s| s.hls));
        assert!(streams.iter().any(|s| !s.hls));
        assert!(!streams.iter().any(|s| s.url.as_str().contains("getVideoPreview")));
        let best = pick_quality(streams, Quality::Best).unwrap();
        assert!(!best[0].hls);
    }
}
