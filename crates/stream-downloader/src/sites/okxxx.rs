//! ok.xxx — KTPlayer pages; `<source>` tags redirect to HLS.

use crate::client;
use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::extract::playable_video_sources;
use crate::hls;
use crate::model::{FetchedPage, Stream};
use crate::quality;
use crate::sites::common::{video_stream, wants_video};
use scraper::Html;

pub fn matches_host(url: &url::Url) -> bool {
    url.host_str()
        .is_some_and(|h| h == "ok.xxx" || h.ends_with(".ok.xxx"))
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !wants_video(options) {
        return Ok(Vec::new());
    }

    let gate_url = {
        let document = Html::parse_document(&page.html);
        playable_video_sources(&document, &page.info.url)
            .into_iter()
            .next()
            .ok_or(Error::NoStreamsFound)?
            .url
    };

    let referer = page.info.url.as_str();
    let master = client::browser_get(client, gate_url, referer)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let streams = hls::parse_master(&master)?
        .into_iter()
        .map(|v| video_stream(v.url, v.height, true, None))
        .collect();

    quality::pick_streams(streams, options.quality)
}
