//! Rutube — play options API (common Yandex preview upstream).

use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::model::{MediaKind, Stream};
use crate::quality::Quality;
use serde::Deserialize;
use url::Url;

pub const REFERER: &str = "https://rutube.ru/";

pub fn matches_host(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|h| h == "rutube.ru" || h.ends_with(".rutube.ru"))
}

pub fn video_id(url: &Url) -> Option<String> {
    let path = url.path();
    let segment = path.strip_prefix("/video/")?.trim_end_matches('/');
    (!segment.is_empty()).then(|| segment.to_owned())
}

pub async fn extract(
    client: &reqwest::Client,
    url: &Url,
    title: &str,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !options.kinds.contains(&MediaKind::Video) {
        return Ok(Vec::new());
    }
    let id = video_id(url).ok_or_else(|| Error::InvalidUrl("missing Rutube video id".into()))?;
    let options_json = fetch_play_options(client, &id).await?;
    let streams = streams_from_options(&options_json, title)?;
    pick_quality(streams, options.quality)
}

async fn fetch_play_options(client: &reqwest::Client, id: &str) -> Result<PlayOptions> {
    let url = format!("https://rutube.ru/api/play/options/{id}/?format=json");
    Ok(client
        .get(url)
        .header("Referer", REFERER)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn streams_from_options(body: &PlayOptions, title: &str) -> Result<Vec<Stream>> {
    let balancer = body
        .video_balancer
        .as_ref()
        .ok_or(Error::NoStreamsFound)?;
    let mut out = Vec::new();
    for (name, url) in balancer {
        let is_hls = url.contains(".m3u8");
        out.push(Stream {
            url: Url::parse(url).map_err(|e| Error::InvalidUrl(e.to_string()))?,
            kind: MediaKind::Video,
            label: Some(format!("{title} ({name})")),
            download_user_agent: None,
            mux_audio: None,
            hls: is_hls,
            download_referer: Some(REFERER),
        });
    }
    if out.is_empty() {
        return Err(Error::NoStreamsFound);
    }
    Ok(out)
}

fn pick_quality(mut streams: Vec<Stream>, quality: Quality) -> Result<Vec<Stream>> {
    streams.sort_by_key(|b| std::cmp::Reverse(height_rank(b)));
    Ok(match quality {
        Quality::All => streams,
        Quality::Best => vec![streams.remove(0)],
        Quality::Height(h) => {
            let picked: Vec<_> = streams
                .into_iter()
                .filter(|s| height_rank(s) == h)
                .collect();
            if picked.is_empty() {
                return Err(Error::NoStreamsFound);
            }
            picked
        }
    })
}

fn height_rank(stream: &Stream) -> u32 {
    stream
        .label
        .as_deref()
        .and_then(|l| l.rsplit('(').next()?.trim_end_matches(')').parse().ok())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
struct PlayOptions {
    video_balancer: Option<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_video_id() {
        let url = Url::parse("http://rutube.ru/video/abc-def/").unwrap();
        assert_eq!(video_id(&url).as_deref(), Some("abc-def"));
    }
}
