//! Yandex Video — preview aggregator + native VH player API.

mod preview;
mod vh;

use crate::error::Result;
use crate::extract::ExtractOptions;
use crate::model::{FetchedPage, Stream};
use crate::sites::registry;
use url::Url;

pub use preview::film_id;
pub use vh::content_id;

pub fn matches_host(url: &Url) -> bool {
    if url.host_str() == Some("frontend.vh.yandex.ru") {
        return true;
    }
    let host = url.host_str().unwrap_or("");
    if !is_yandex_host(host) {
        return false;
    }
    url.path().contains("/video/preview")
        || url
            .query()
            .is_some_and(|q| q.contains("stream_id="))
}

fn is_yandex_host(host: &str) -> bool {
    let h = host
        .strip_prefix("www.")
        .unwrap_or(host);
    h.starts_with("yandex.") || h.ends_with(".yandex.ru")
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if is_preview(&page.info.url) {
        let source = preview::resolve(page)?;
        return registry::extract_upstream(
            client,
            &source.url,
            &source.title,
            options,
        )
        .await;
    }
    if let Some(id) = content_id(&page.info.url) {
        return vh::extract(
            client,
            &id,
            Some(&page.info.title),
            options,
        )
        .await;
    }
    Ok(Vec::new())
}

fn is_preview(url: &Url) -> bool {
    url.path().contains("/video/preview")
        || url
            .query()
            .is_some_and(|q| q.contains("filmId="))
}

pub async fn extract_vh(
    client: &reqwest::Client,
    id: &str,
    title: Option<&str>,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    vh::extract(client, id, title, options).await
}
