//! XVideos-family `html5player.setVideoUrl*` / `setVideoHLS` embed.

use crate::error::{Error, Result};
use crate::model::Stream;
use crate::sites::common::{self, parse_url, video_stream};

const MP4_METHODS: &[&str] = &["setVideoUrlLow", "setVideoUrlHigh", "setVideoUrlHD"];

pub async fn extract(client: &reqwest::Client, html: &str, referer: &str) -> Result<Vec<Stream>> {
    let mut streams = Vec::new();
    for method in MP4_METHODS {
        let Some(url) = parse_player_url(html, method) else {
            continue;
        };
        let height = mp4_height(&url).unwrap_or_else(|| fallback_mp4_height(method));
        streams.push(video_stream(parse_url(&url)?, height, false, None));
    }
    if let Some(hls_url) = parse_player_url(html, "setVideoHLS") {
        streams.extend(common::hls_streams_from_master(client, &hls_url, referer).await?);
    }
    if streams.is_empty() {
        Err(Error::NoStreamsFound)
    } else {
        Ok(streams)
    }
}

pub fn parse_player_url(html: &str, method: &str) -> Option<String> {
    let needle = format!("html5player.{method}('");
    let start = html.find(&needle)? + needle.len();
    let end = html[start..].find('\'')?;
    Some(html[start..start + end].to_string())
}

pub fn mp4_height(url: &str) -> Option<u32> {
    let rest = url.split("video_").nth(1)?;
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
    fn parses_player_urls_from_fixture() {
        let html = include_str!("../../../tests/fixtures/xnxx_player.html");
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
}
