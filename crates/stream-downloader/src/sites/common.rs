//! Shared helpers for site plugins.

use crate::client;
use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::hls;
use crate::model::{MediaKind, Stream};
use crate::quality;
use url::Url;

pub const BROWSER_UA: &str = crate::client::BROWSER_UA;

pub fn wants_video(options: &ExtractOptions) -> bool {
    options
        .kinds
        .contains(&MediaKind::Video)
}

pub fn parse_url(raw: &str) -> Result<Url> {
    Url::parse(raw).map_err(|e| Error::InvalidUrl(e.to_string()))
}

pub fn video_stream(
    url: Url,
    height: u32,
    hls: bool,
    referer: Option<&'static str>,
) -> Stream {
    Stream {
        url,
        kind: MediaKind::Video,
        label: Some(format!("{height}p")),
        download_user_agent: Some(BROWSER_UA),
        mux_audio: None,
        hls,
        download_referer: referer,
    }
}

pub fn match_refreshed(
    stream: &Stream,
    streams: &[Stream],
    match_hls: bool,
) -> Result<Stream> {
    let height = quality::height_hint(stream);
    streams
        .iter()
        .find(|s| {
            quality::height_hint(s) == height
                && (!match_hls || s.hls == stream.hls)
        })
        .cloned()
        .or_else(|| {
            streams
                .iter()
                .max_by_key(|s| quality::height_hint(s))
                .cloned()
        })
        .ok_or(Error::NoStreamsFound)
}

pub async fn hls_streams_from_master(
    client: &reqwest::Client,
    master_url: &str,
    referer: &str,
) -> Result<Vec<Stream>> {
    let master = parse_url(master_url)?;
    let body = client::browser_get(client, master.clone(), referer)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let base = master
        .join("./")
        .map_err(|e| Error::InvalidUrl(e.to_string()))?;
    Ok(
        hls::parse_master_with_base(&body, Some(&base))?
            .into_iter()
            .map(|v| video_stream(v.url, v.height, true, None))
            .collect(),
    )
}
