//! Discover and download media files served progressively over HTTP.

mod client;
mod download;
mod error;
mod extract;
mod hls;
mod innertube;
mod merge;
mod model;
mod naming;
mod page;
mod progress;
mod quality;
mod sites;

pub use download::{DownloadOptions, DownloadedFile, Downloader};
pub use error::{Error, Result};
pub use extract::profiles;
pub use extract::{
    ExtractOptions, Extractor, HostMatch, JsonMapRule, PageExtractor, ProfileRegistry, ProfileRule,
    SiteProfile,
};
pub use merge::prefetch_tools;
pub use model::{FetchedPage, MediaKind, PageInfo, Stream};
pub use naming::OutputName;
pub use page::PageFetcher;
pub use progress::{DownloadProgress, format_line};
pub use quality::Quality;
pub use sites::yandex;
pub use sites::youtube;

use std::path::Path;

/// High-level orchestration: fetch page → extract file URLs → download.
pub struct Session {
    client: reqwest::Client,
    fetcher: PageFetcher,
    extractor: PageExtractor,
    downloader: Downloader,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        let client = client::build();
        Self {
            client: client.clone(),
            fetcher: PageFetcher::with_client(client.clone()),
            extractor: PageExtractor::default(),
            downloader: Downloader::with_client(client),
        }
    }

    pub async fn discover(
        &self,
        page_url: &str,
        kinds: &[MediaKind],
        quality: Quality,
    ) -> Result<Vec<Stream>> {
        let page = self.fetcher.fetch(page_url).await?;
        self.extractor
            .extract(
                &self.client,
                &page,
                &ExtractOptions {
                    kinds: kinds.to_vec(),
                    quality,
                },
            )
            .await
    }

    pub async fn download_streams(
        &self,
        page_url: &str,
        kinds: &[MediaKind],
        quality: Quality,
        options: &DownloadOptions<'_>,
    ) -> Result<Vec<DownloadedFile>> {
        let page = self.fetcher.fetch(page_url).await?;
        let streams = self
            .extractor
            .extract(
                &self.client,
                &page,
                &ExtractOptions {
                    kinds: kinds.to_vec(),
                    quality,
                },
            )
            .await?;
        if streams.is_empty() {
            return Err(Error::NoStreamsFound);
        }

        let referer = page.info.url.as_str();
        let mut results = Vec::with_capacity(streams.len());
        for stream in streams {
            let name = OutputName::for_stream(&page.info, &stream);
            let path = self
                .downloader
                .download(
                    &stream,
                    &name,
                    &DownloadOptions {
                        output_dir: options.output_dir,
                        referer: Some(referer),
                        on_progress: options.on_progress,
                    },
                )
                .await?;
            results.push(DownloadedFile { stream, path });
        }
        Ok(results)
    }
}

/// Validate a page URL before any network I/O.
pub fn validate_page_url(raw: &str) -> Result<url::Url> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidUrl("URL must not be empty".into()));
    }
    let parsed = url::Url::parse(trimmed).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        scheme => Err(Error::InvalidUrl(format!(
            "unsupported scheme `{scheme}` (use http or https)"
        ))),
    }
}

/// Validate an optional output directory.
pub fn validate_output_dir(path: Option<&Path>) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if path.as_os_str().is_empty() {
        return Err(Error::InvalidOutput("output path must not be empty".into()));
    }
    if path.exists() && !path.is_dir() {
        return Err(Error::InvalidOutput(format!(
            "`{}` exists and is not a directory",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rejects_empty_and_non_http_urls() {
        assert!(matches!(validate_page_url(""), Err(Error::InvalidUrl(_))));
        assert!(matches!(
            validate_page_url("ftp://example.com"),
            Err(Error::InvalidUrl(_))
        ));
    }

    #[test]
    fn accepts_https_urls() {
        let url = validate_page_url("https://example.com/watch?v=1").unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn output_dir_must_be_directory_when_present() {
        let file = std::env::temp_dir().join("stream-dl-test-file");
        std::fs::write(&file, b"x").unwrap();
        assert!(matches!(
            validate_output_dir(Some(file.as_path())),
            Err(Error::InvalidOutput(_))
        ));
        let _ = std::fs::remove_file(&file);

        let missing = PathBuf::from("/tmp/stream-dl-missing-dir-test");
        let _ = std::fs::remove_dir_all(&missing);
        assert!(validate_output_dir(Some(missing.as_path())).is_ok());
    }

    #[tokio::test]
    #[ignore = "network"]
    async fn live_okxxx_discover() {
        let session = Session::new();
        let streams = session
            .discover(
                "https://ok.xxx/video/750877/",
                &[MediaKind::Video],
                Quality::Height(720),
            )
            .await
            .expect("discover");
        assert!(!streams.is_empty());
        assert!(streams[0].hls);
        assert_eq!(crate::quality::height_hint(&streams[0]), 720);
    }

    #[tokio::test]
    #[ignore = "network"]
    async fn live_ok_preview_discover() {
        let session = Session::new();
        let streams = session
            .discover(
                "https://yandex.ru/video/preview/11285747945322644110?q_source=title",
                &[MediaKind::Video],
                Quality::Height(1080),
            )
            .await
            .expect("discover");
        assert!(!streams.is_empty());
        assert_eq!(crate::quality::height_hint(&streams[0]), 1080);
    }
}
