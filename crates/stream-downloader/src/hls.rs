//! HLS playlist parsing (download via [`crate::merge::download_hls`]).

use crate::error::{Error, Result};
use url::Url;

/// One entry from an HLS master playlist (`#EXT-X-STREAM-INF` + URL).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterVariant {
    pub height: u32,
    pub url: Url,
}

/// Parse variant URLs and heights from an HLS master playlist body.
pub fn parse_master(body: &str) -> Result<Vec<MasterVariant>> {
    parse_master_with_base(body, None)
}

/// Like [`parse_master`], but resolves relative variant URLs against `base`.
pub fn parse_master_with_base(body: &str, base: Option<&Url>) -> Result<Vec<MasterVariant>> {
    if !body.trim_start().starts_with("#EXTM3U") {
        return Err(Error::NoStreamsFound);
    }

    let mut variants = Vec::new();
    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") else {
            continue;
        };
        let height = resolution_height(attrs).unwrap_or(0);
        let Some(url_line) = lines.next() else {
            break;
        };
        if url_line.starts_with('#') {
            continue;
        }
        let trimmed = url_line.trim();
        let url = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            Url::parse(trimmed).map_err(|e| Error::InvalidUrl(e.to_string()))?
        } else if let Some(base) = base {
            base.join(trimmed)
                .map_err(|e| Error::InvalidUrl(e.to_string()))?
        } else {
            return Err(Error::InvalidUrl(format!(
                "relative HLS URL without base: {trimmed}"
            )));
        };
        variants.push(MasterVariant { height, url });
    }

    if variants.is_empty() {
        Err(Error::NoStreamsFound)
    } else {
        Ok(variants)
    }
}

fn resolution_height(attrs: &str) -> Option<u32> {
    let res = attrs.split("RESOLUTION=").nth(1)?;
    let res = res.split(&[','][..]).next()?;
    res.split('x').nth(1)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_master_variants() {
        let body = r#"#EXTM3U
#EXT-X-STREAM-INF:RESOLUTION=640x360
https://cdn.example.com/360.m3u8
#EXT-X-STREAM-INF:RESOLUTION=1280x720
https://cdn.example.com/720.m3u8
"#;
        let variants = parse_master(body).unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants[0].height, 360);
        assert_eq!(variants[1].height, 720);
    }

    #[test]
    fn resolves_relative_variant_urls() {
        let body = r#"#EXTM3U
#EXT-X-STREAM-INF:RESOLUTION=1280x720
hls-720p.m3u8
"#;
        let base = Url::parse("https://cdn.example.com/path/hls.m3u8").unwrap();
        let variants = parse_master_with_base(body, Some(&base)).unwrap();
        assert_eq!(variants.len(), 1);
        assert_eq!(
            variants[0].url.as_str(),
            "https://cdn.example.com/path/hls-720p.m3u8"
        );
    }
}
