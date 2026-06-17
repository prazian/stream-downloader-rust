//! Site-specific extractors. Each module owns one host family.

pub mod ok;
pub mod okxxx;
pub mod rutube;
pub mod vk;
pub mod yandex;
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
    if yandex::matches_host(&page.info.url) {
        return yandex::extract(client, page, options).await;
    }
    if vk::matches_host(&page.info.url) {
        return vk::extract(client, &page.info.url, &page.info.title, options).await;
    }
    if rutube::matches_host(&page.info.url) {
        return rutube::extract(client, &page.info.url, &page.info.title, options).await;
    }
    if ok::matches_host(&page.info.url) {
        return ok::extract(client, &page.info.url, &page.info.title, options).await;
    }
    if okxxx::matches_host(&page.info.url) {
        return okxxx::extract(client, page, options).await;
    }
    Ok(Vec::new())
}
