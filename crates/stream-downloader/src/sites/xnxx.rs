//! XNXX / XVideos — thin wrapper around the shared [`super::patterns::html5player`] pattern.

use crate::client;
use crate::error::Result;
use crate::extract::ExtractOptions;
use crate::model::{FetchedPage, Stream};
use crate::quality;
use crate::sites::common::{self, wants_video};
use crate::sites::patterns::html5player;
use url::Url;

pub fn matches_host(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        let h = host
            .strip_prefix("www.")
            .unwrap_or(host);
        h.contains("xnxx")
            || h.ends_with("xvideos.com")
            || h.ends_with(".xvideos.com")
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
    if !wants_video(options) || !is_video_page(&page.info.url) {
        return Ok(Vec::new());
    }
    let referer = page.info.url.as_str();
    let streams = html5player::extract(client, &page.html, referer).await?;
    quality::pick_streams(streams, options.quality)
}

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
    let referer = page.info.url.as_str();
    let html = client::browser_get(client, page.info.url.clone(), referer)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let streams = html5player::extract(client, &html, referer).await?;
    common::match_refreshed(stream, &streams, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_xnxx_and_xvideos_hosts() {
        let xnxx =
            Url::parse("https://www.xnxx.com/video-1cpbmdea/wild_gangbang")
                .unwrap();
        assert!(matches_host(&xnxx));
        let xv = Url::parse("https://www.xvideos.com/video.ufkthho1234/test")
            .unwrap();
        assert!(matches_host(&xv));
        let other = Url::parse("https://example.com/video-1").unwrap();
        assert!(!matches_host(&other));
    }

    #[test]
    fn detects_video_pages() {
        let xnxx =
            Url::parse("https://www.xnxx.com/video-1cpbmdea/wild_gangbang")
                .unwrap();
        assert!(is_video_page(&xnxx));
        let home = Url::parse("https://www.xnxx.com/").unwrap();
        assert!(!is_video_page(&home));
    }
}
