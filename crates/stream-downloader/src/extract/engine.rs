use crate::extract::generic;
use crate::extract::json;
use crate::extract::profile::{ProfileRegistry, ProfileRule, SiteProfile};
use crate::extract::{ExtractOptions, Extractor, dedupe_streams};
use crate::model::{FetchedPage, Stream};
use crate::sites;
use scraper::Html;

#[derive(Debug, Clone)]
pub struct PageExtractor {
    registry: ProfileRegistry,
}

impl Default for PageExtractor {
    fn default() -> Self {
        Self::new(ProfileRegistry::default())
    }
}

impl PageExtractor {
    pub fn new(registry: ProfileRegistry) -> Self {
        Self { registry }
    }

    pub async fn extract_page(
        &self,
        client: &reqwest::Client,
        page: &FetchedPage,
        options: &ExtractOptions,
    ) -> crate::error::Result<Vec<Stream>> {
        let mut streams = {
            let document = Html::parse_document(&page.html);
            let mut found = generic::extract(&document, &page.info.url, options);
            for profile in self.registry.matching(&page.info.url) {
                found.extend(apply_profile(
                    profile,
                    &page.html,
                    &document,
                    &page.info.url,
                    options,
                ));
            }
            found
        };

        streams.extend(sites::extract_for_host(client, page, options).await?);
        Ok(options
            .quality
            .filter_streams(dedupe_streams(streams)))
    }
}

impl Extractor for PageExtractor {
    async fn extract(
        &self,
        client: &reqwest::Client,
        page: &FetchedPage,
        options: &ExtractOptions,
    ) -> crate::error::Result<Vec<Stream>> {
        self.extract_page(client, page, options).await
    }
}

fn apply_profile(
    profile: &SiteProfile,
    html: &str,
    document: &Html,
    base: &url::Url,
    options: &ExtractOptions,
) -> Vec<Stream> {
    profile
        .rules
        .iter()
        .flat_map(|rule| match rule {
            ProfileRule::Selector {
                selector,
                attr,
                kind,
                label_attr,
            } => generic::select_attr(document, selector, attr, base, Some(*kind), *label_attr),
            ProfileRule::EmbeddedJson(map) => json::apply_rule(html, map, options),
        })
        .collect()
}
