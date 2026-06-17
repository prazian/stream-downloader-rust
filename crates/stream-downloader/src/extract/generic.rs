use crate::extract::{ExtractOptions, infer_kind, resolve_url};
use crate::model::{MediaKind, Stream};
use scraper::{Html, Selector};
use std::collections::HashSet;
use url::Url;

pub fn extract(document: &Html, base: &Url, options: &ExtractOptions) -> Vec<Stream> {
    let kinds: HashSet<_> = options.kinds.iter().copied().collect();
    let mut streams = Vec::new();

    if kinds.contains(&MediaKind::Video) {
        streams.extend(select_attr(
            document,
            "video",
            "src",
            base,
            Some(MediaKind::Video),
            None,
        ));
        streams.extend(select_attr(
            document,
            "video source",
            "src",
            base,
            Some(MediaKind::Video),
            Some("label"),
        ));
    }
    if kinds.contains(&MediaKind::Audio) {
        streams.extend(select_attr(
            document,
            "audio",
            "src",
            base,
            Some(MediaKind::Audio),
            None,
        ));
        streams.extend(select_attr(
            document,
            "audio source",
            "src",
            base,
            Some(MediaKind::Audio),
            Some("label"),
        ));
    }

    streams.extend(links_by_extension(document, base, &kinds));
    streams
}

pub fn select_attr(
    document: &Html,
    selector: &str,
    attr: &str,
    base: &Url,
    kind_hint: Option<MediaKind>,
    label_attr: Option<&str>,
) -> Vec<Stream> {
    let Ok(sel) = Selector::parse(selector) else {
        return Vec::new();
    };
    document
        .select(&sel)
        .filter_map(|el| {
            let src = el.value().attr(attr)?;
            let url = resolve_url(base, src)?;
            let kind = infer_kind(&url, kind_hint);
            let label = label_attr
                .and_then(|name| el.value().attr(name))
                .map(str::to_owned);
            Some(Stream {
                url,
                kind,
                label,
                download_user_agent: None,
                mux_audio: None,
                hls: false,
                download_referer: None,
            })
        })
        .collect()
}

/// Full-length `<video source>` entries, skipping preview clips and Auto duplicates.
pub(crate) fn playable_video_sources(document: &Html, base: &Url) -> Vec<Stream> {
    select_attr(
        document,
        "video source",
        "src",
        base,
        Some(MediaKind::Video),
        Some("label"),
    )
    .into_iter()
    .filter(|s| {
        !s.url.path().contains("preview")
            && !s
                .label
                .as_deref()
                .is_some_and(|l| l.eq_ignore_ascii_case("auto"))
    })
    .collect()
}

fn links_by_extension(document: &Html, base: &Url, kinds: &HashSet<MediaKind>) -> Vec<Stream> {
    let Ok(sel) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    document
        .select(&sel)
        .filter_map(|el| {
            let href = el.value().attr("href")?;
            let url = resolve_url(base, href)?;
            let kind = infer_kind(&url, None);
            if kinds.contains(&kind) && has_known_extension(&url, kind) {
                Some(Stream {
                    url,
                    kind,
                    label: None,
                    download_user_agent: None,
                    mux_audio: None,
                    hls: false,
                    download_referer: None,
                })
            } else {
                None
            }
        })
        .collect()
}

fn has_known_extension(url: &Url, kind: MediaKind) -> bool {
    let path = url.path().to_ascii_lowercase();
    kind.file_extensions()
        .iter()
        .any(|ext| path.ends_with(&format!(".{ext}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::Quality;
    use url::Url;

    fn page_url() -> Url {
        Url::parse("https://example.com/watch/episode").unwrap()
    }

    #[test]
    fn finds_video_tags_and_sources() {
        let html = r#"
            <video src="/media/main.mp4"></video>
            <video><source src="../trailers/preview.webm" label="preview"></video>
        "#;
        let document = Html::parse_document(html);
        let streams = extract(
            &document,
            &page_url(),
            &ExtractOptions {
                kinds: vec![MediaKind::Video],
                quality: Quality::Best,
            },
        );
        assert_eq!(streams.len(), 2);
    }

    #[test]
    fn skips_preview_and_auto_sources() {
        let html = r#"
            <video>
                <source src="/preview.mp4" label="preview" />
                <source src="/360.mp4" label="Auto" />
                <source src="/720.mp4" label="720p" />
            </video>
        "#;
        let document = Html::parse_document(html);
        let sources = playable_video_sources(&document, &page_url());
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].label.as_deref(), Some("720p"));
    }

    #[test]
    fn respects_kind_filter() {
        let html = r#"<audio src="/song.mp3"></audio>"#;
        let document = Html::parse_document(html);
        let streams = extract(
            &document,
            &page_url(),
            &ExtractOptions {
                kinds: vec![MediaKind::Video],
                quality: Quality::Best,
            },
        );
        assert!(streams.is_empty());
    }
}
