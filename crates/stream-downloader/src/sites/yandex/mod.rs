//! Yandex Video — preview aggregator + native VH player API.

mod preview;
mod vh;

use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::model::{FetchedPage, Stream};
use crate::sites::{ok, rutube, vk, youtube};
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
    url.path().contains("/video/preview") || url.query().is_some_and(|q| q.contains("stream_id="))
}

fn is_yandex_host(host: &str) -> bool {
    let h = host.strip_prefix("www.").unwrap_or(host);
    h.starts_with("yandex.") || h.ends_with(".yandex.ru")
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if is_preview(&page.info.url) {
        let source = preview::resolve(page)?;
        return extract_upstream(client, &source.url, &source.title, options).await;
    }
    if let Some(id) = content_id(&page.info.url) {
        return vh::extract(client, &id, Some(&page.info.title), options).await;
    }
    Ok(Vec::new())
}

fn is_preview(url: &Url) -> bool {
    url.path().contains("/video/preview") || url.query().is_some_and(|q| q.contains("filmId="))
}

async fn extract_upstream(
    client: &reqwest::Client,
    url: &Url,
    title: &str,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if youtube::matches_host(url) {
        let page = fetch_page(client, url.clone(), title).await?;
        return youtube::extract(client, &page, options).await;
    }
    if vk::matches_host(url) {
        return vk::extract(client, url, title, options).await;
    }
    if rutube::matches_host(url) {
        return rutube::extract(client, url, title, options).await;
    }
    if ok::matches_host(url) {
        return ok::extract(client, url, title, options).await;
    }
    if let Some(id) = content_id(url) {
        return vh::extract(client, &id, Some(title), options).await;
    }
    Err(Error::InvalidUrl(format!(
        "unsupported Yandex upstream host: {}",
        url.host_str().unwrap_or("?")
    )))
}

async fn fetch_page(client: &reqwest::Client, url: Url, title: &str) -> Result<FetchedPage> {
    let response = client.get(url.clone()).send().await?.error_for_status()?;
    let final_url = response.url().clone();
    let html = response.text().await?;
    Ok(FetchedPage {
        info: crate::model::PageInfo {
            url: final_url,
            title: title.to_owned(),
        },
        html,
    })
}
