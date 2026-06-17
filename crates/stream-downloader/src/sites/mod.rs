//! Site-specific extractors. Each module owns one host family.
//!
//! YouTube follows yt-dlp's `android_vr` innertube client (no player JS, direct file URLs).

pub mod youtube;

use crate::error::Result;
use crate::extract::ExtractOptions;
use crate::model::{FetchedPage, Stream};

pub async fn extract_for_host(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if youtube::matches_host(&page.info.url) {
        return youtube::extract(client, page, options).await;
    }
    Ok(Vec::new())
}
