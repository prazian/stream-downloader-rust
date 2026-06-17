use crate::model::{MediaKind, PageInfo, Stream};
use std::path::{Path, PathBuf};

/// Resolved on-disk filename for a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputName {
    pub file_name: String,
}

impl OutputName {
    pub fn for_stream(page: &PageInfo, stream: &Stream) -> Self {
        let page_slug = slugify(&clean_title_for_filename(&page.title));
        let ext = if stream.mux_audio.is_some() {
            "mp4".to_owned()
        } else {
            extension_from_url(&stream.url, stream.kind)
        };

        let file_name = match stream.label.as_deref().filter(|s| !s.is_empty()) {
            Some(quality) => format!("{page_slug}-{}.{}", slugify(quality), ext),
            None => {
                let stem = segment_stem(&stream.url).unwrap_or_else(|| "stream".into());
                if stem.starts_with(&page_slug) || page_slug == "page" {
                    format!("{stem}.{ext}")
                } else {
                    format!("{page_slug}-{stem}.{ext}")
                }
            }
        };

        Self { file_name }
    }

    pub fn path_in(&self, dir: &Path) -> PathBuf {
        dir.join(&self.file_name)
    }
}

fn clean_title_for_filename(title: &str) -> String {
    let t = title.trim();
    t.strip_suffix(" - YouTube")
        .unwrap_or(t)
        .trim()
        .to_string()
}

fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    if out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() { "page".into() } else { out }
}

fn segment_stem(url: &url::Url) -> Option<String> {
    let segment = url.path_segments()?.next_back()?;
    let stem = Path::new(segment)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())?;
    Some(slugify(stem))
}

fn extension_from_url(url: &url::Url, kind: MediaKind) -> String {
    let path = url.path();
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|ext| kind.file_extensions().contains(&ext.as_str()))
        .unwrap_or_else(|| default_extension(kind).to_owned())
}

fn default_extension(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Video => "mp4",
        MediaKind::Audio => "mp3",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn page(title: &str) -> PageInfo {
        PageInfo {
            url: Url::parse("https://example.com/x").unwrap(),
            title: title.into(),
        }
    }

    fn stream(url: &str, label: Option<&str>) -> Stream {
        Stream {
            url: Url::parse(url).unwrap(),
            kind: MediaKind::Video,
            label: label.map(str::to_owned),
            download_user_agent: None,
            mux_audio: None,
            hls: false,
            download_referer: None,
        }
    }
    #[test]
    fn combines_page_title_and_source_name() {
        let name = OutputName::for_stream(
            &page("My Cool Video!"),
            &stream("https://cdn.example.com/trailer.webm", None),
        );
        assert_eq!(name.file_name, "my-cool-video-trailer.webm");
    }

    #[test]
    fn uses_label_when_present() {
        let name = OutputName::for_stream(
            &page("Episode 1"),
            &stream("https://cdn.example.com/v/1", Some("1080p")),
        );
        assert_eq!(name.file_name, "episode-1-1080p.mp4");
    }

    #[test]
    fn keeps_full_title_and_appends_quality() {
        let name = OutputName::for_stream(
            &page("Bombs vs Brains | Low Quality Version - YouTube"),
            &stream("https://cdn.example.com/v/1", Some("1080p")),
        );
        assert_eq!(
            name.file_name,
            "bombs-vs-brains-low-quality-version-1080p.mp4"
        );
    }

    #[test]
    fn slugify_strips_noise() {
        assert_eq!(slugify("  Hello---World!! "), "hello-world");
    }
}
