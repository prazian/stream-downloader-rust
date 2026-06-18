use crate::client;
use crate::error::Result;
use crate::model::{FetchedPage, PageInfo};
use url::Url;

pub struct PageFetcher {
    client: reqwest::Client,
}

impl Default for PageFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl PageFetcher {
    pub fn new() -> Self {
        Self::with_client(client::build())
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
        }
    }

    pub async fn fetch(
        &self,
        page_url: &str,
    ) -> Result<FetchedPage> {
        let response = self
            .client
            .get(page_url)
            .send()
            .await?
            .error_for_status()?;
        let final_url = response.url().clone();
        let html = response.text().await?;
        let title =
            extract_title(&html).unwrap_or_else(|| fallback_title(&final_url));
        Ok(FetchedPage {
            info: PageInfo {
                url: final_url,
                title,
            },
            html,
        })
    }
}

fn extract_title(html: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("title").ok()?;
    document
        .select(&selector)
        .next()
        .map(|node| {
            node.text()
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .filter(|t| !t.is_empty())
}

fn fallback_title(url: &Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back().map(str::to_owned))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "page".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_title_from_html() {
        let html =
            "<html><head><title>My Clip</title></head><body></body></html>";
        assert_eq!(
            extract_title(html).as_deref(),
            Some("My Clip")
        );
    }

    #[test]
    fn falls_back_to_last_path_segment() {
        let url = Url::parse("https://example.com/videos/clip-42").unwrap();
        assert_eq!(fallback_title(&url), "clip-42");
    }
}
