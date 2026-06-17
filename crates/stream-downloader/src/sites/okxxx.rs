//! ok.xxx — KTPlayer pages; `<source>` tags redirect to HLS.

use crate::client;
use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::extract::playable_video_sources;
use crate::hls;
use crate::model::{FetchedPage, MediaKind, Stream};
use crate::quality;
use scraper::Html;
use url::Url;

const USER_AGENT: &str = crate::client::BROWSER_UA;

pub fn matches_host(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|h| h == "ok.xxx" || h.ends_with(".ok.xxx"))
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !options.kinds.contains(&MediaKind::Video) {
        return Ok(Vec::new());
    }

    let document = Html::parse_document(&page.html);
    let gate = playable_video_sources(&document, &page.info.url)
        .into_iter()
        .next()
        .ok_or(Error::NoStreamsFound)?;

    let referer = page.info.url.as_str();
    let master = client::browser_get(client, gate.url, referer)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let streams = hls::parse_master(&master)?
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
        .collect();

    quality::pick_streams(streams, options.quality)
}
