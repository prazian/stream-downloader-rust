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
        variants.push(MasterVariant {
            height,
            url: Url::parse(url_line.trim()).map_err(|e| Error::InvalidUrl(e.to_string()))?,
        });
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
}
