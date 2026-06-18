use crate::error::Result;
use crate::model::{FetchedPage, MediaKind, Stream};
use crate::quality::Quality;
use std::collections::HashSet;
use url::Url;

pub use engine::PageExtractor;
pub use profile::{
    HostMatch, JsonMapRule, ProfileRegistry, ProfileRule, SiteProfile,
};

mod engine;
mod generic;
mod json;
mod profile;
pub mod profiles;

pub(crate) use generic::playable_video_sources;
pub(crate) use json::extract_json_object;

/// Options passed to extractors.
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub kinds: Vec<MediaKind>,
    pub quality: Quality,
}

/// Strategy for discovering downloadable media files in a fetched page.
pub trait Extractor {
    fn extract<'a>(
        &'a self,
        client: &'a reqwest::Client,
        page: &'a FetchedPage,
        options: &'a ExtractOptions,
    ) -> impl std::future::Future<Output = Result<Vec<Stream>>> + 'a;
}

pub(crate) fn resolve_url(
    base: &Url,
    reference: &str,
) -> Option<Url> {
    base.join(reference.trim()).ok()
}

pub(crate) fn infer_kind(
    url: &Url,
    hint: Option<MediaKind>,
) -> MediaKind {
    if let Some(kind) = hint {
        return kind;
    }
    let path = url.path().to_ascii_lowercase();
    if MediaKind::Audio
        .file_extensions()
        .iter()
        .any(|ext| path.ends_with(&format!(".{ext}")))
    {
        return MediaKind::Audio;
    }
    MediaKind::Video
}

pub(crate) fn dedupe_streams(mut streams: Vec<Stream>) -> Vec<Stream> {
    let mut seen = HashSet::new();
    streams.retain(|s| seen.insert(s.url.clone()));
    streams
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_urls_against_page() {
        let base = Url::parse("https://cdn.example.com/watch/").unwrap();
        let resolved = resolve_url(&base, "../media/clip.mp4").unwrap();
        assert_eq!(
            resolved.as_str(),
            "https://cdn.example.com/media/clip.mp4"
        );
    }
}
