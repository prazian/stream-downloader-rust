//! Central site registry — add one [`SitePlugin`] row per new host family.

use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::model::{FetchedPage, PageInfo, Stream};
use crate::sites::plugin::SitePlugin;
use crate::sites::yandex::content_id;
use crate::sites::{ok, okxxx, pornhub, rutube, vk, xnxx, yandex, youtube};
use std::future::Future;
use std::pin::Pin;
use url::Url;

/// Order matters: first match wins (aggregators before upstream hosts they delegate to).
pub static SITES: &[SitePlugin] = &[
    SitePlugin::new("youtube", youtube::matches_host, extract_youtube, None),
    SitePlugin::new("yandex", yandex::matches_host, extract_yandex, None),
    SitePlugin::new("vk", vk::matches_host, extract_vk, None),
    SitePlugin::new("rutube", rutube::matches_host, extract_rutube, None),
    SitePlugin::new("ok", ok::matches_host, extract_ok, None),
    SitePlugin::new("okxxx", okxxx::matches_host, extract_okxxx, None),
    SitePlugin::new(
        "pornhub",
        pornhub::matches_host,
        extract_pornhub,
        Some(refresh_pornhub),
    ),
    SitePlugin::new("xnxx", xnxx::matches_host, extract_xnxx, Some(refresh_xnxx)),
];

pub async fn extract_for_host(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    extract_first_match(client, page, options).await
}

pub async fn refresh_stream(
    client: &reqwest::Client,
    page: &FetchedPage,
    stream: Stream,
) -> Result<Stream> {
    for site in SITES {
        if (site.matches)(&page.info.url) {
            if let Some(refresh) = site.refresh {
                return refresh(client, page, &stream).await;
            }
            break;
        }
    }
    Ok(stream)
}

/// Resolve a Yandex preview upstream URL through the same registry.
pub async fn extract_upstream(
    client: &reqwest::Client,
    url: &Url,
    title: &str,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if youtube::matches_host(url) {
        let page = fetch_page(client, url.clone(), title).await?;
        return extract_first_match(client, &page, options).await;
    }
    let page = FetchedPage {
        info: PageInfo {
            url: url.clone(),
            title: title.to_owned(),
        },
        html: String::new(),
    };
    let streams = extract_first_match(client, &page, options).await?;
    if !streams.is_empty() {
        return Ok(streams);
    }
    if let Some(id) = content_id(url) {
        return yandex::extract_vh(client, &id, Some(title), options).await;
    }
    Err(Error::InvalidUrl(format!(
        "unsupported Yandex upstream host: {}",
        url.host_str().unwrap_or("?")
    )))
}

async fn extract_first_match(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    for site in SITES {
        if (site.matches)(&page.info.url) {
            return (site.extract)(client, page, options).await;
        }
    }
    Ok(Vec::new())
}

async fn fetch_page(client: &reqwest::Client, url: Url, title: &str) -> Result<FetchedPage> {
    let response = client.get(url.clone()).send().await?.error_for_status()?;
    let final_url = response.url().clone();
    let html = response.text().await?;
    Ok(FetchedPage {
        info: PageInfo {
            url: final_url,
            title: title.to_owned(),
        },
        html,
    })
}

fn extract_youtube<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(youtube::extract(client, page, options))
}

fn extract_yandex<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(yandex::extract(client, page, options))
}

fn extract_vk<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(vk::extract(
        client,
        &page.info.url,
        &page.info.title,
        options,
    ))
}

fn extract_rutube<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(rutube::extract(
        client,
        &page.info.url,
        &page.info.title,
        options,
    ))
}

fn extract_ok<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(ok::extract(
        client,
        &page.info.url,
        &page.info.title,
        options,
    ))
}

fn extract_okxxx<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(okxxx::extract(client, page, options))
}

fn extract_pornhub<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(pornhub::extract(client, page, options))
}

fn extract_xnxx<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    options: &'a ExtractOptions,
) -> Pin<Box<dyn Future<Output = Result<Vec<Stream>>> + Send + 'a>> {
    Box::pin(xnxx::extract(client, page, options))
}

fn refresh_pornhub<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    stream: &'a Stream,
) -> Pin<Box<dyn Future<Output = Result<Stream>> + Send + 'a>> {
    Box::pin(pornhub::refresh_stream(client, page, stream))
}

fn refresh_xnxx<'a>(
    client: &'a reqwest::Client,
    page: &'a FetchedPage,
    stream: &'a Stream,
) -> Pin<Box<dyn Future<Output = Result<Stream>> + Send + 'a>> {
    Box::pin(xnxx::refresh_stream(client, page, stream))
}
