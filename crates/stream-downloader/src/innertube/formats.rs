use crate::extract::ExtractOptions;
use crate::model::{MediaKind, MuxPart, Stream};
use crate::quality::Quality;
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use url::Url;

pub fn select_for_kinds(
    response: &PlayerResponse,
    options: &ExtractOptions,
    user_agent: &'static str,
) -> Vec<Stream> {
    let kinds: HashSet<_> = options.kinds.iter().copied().collect();
    let Some(sd) = response.streaming_data.as_ref() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    if kinds.contains(&MediaKind::Video) {
        out.extend(select_videos(sd, options.quality, user_agent));
    }
    if kinds.contains(&MediaKind::Audio)
        && let Some(s) = best_audio(sd, user_agent)
    {
        out.push(s);
    }
    out
}

fn select_videos(sd: &StreamingData, quality: Quality, ua: &'static str) -> Vec<Stream> {
    let by_height = video_by_height(sd, ua);
    let pick = quality.pick(by_height.keys());
    pick.into_iter()
        .filter_map(|h| by_height.get(&h).cloned())
        .collect()
}

fn video_by_height(sd: &StreamingData, ua: &'static str) -> BTreeMap<u32, Stream> {
    let audio = best_audio_part(sd, ua);
    let mut map = BTreeMap::new();
    for f in all_formats(sd).filter(|f| f.url.is_some() && is_muxed(f)) {
        let h = f.height.unwrap_or(0);
        if let Some(s) = stream_from(f, MediaKind::Video, ua, None) {
            map.entry(h).or_insert(s);
        }
    }
    if let Some(a) = audio {
        for f in all_formats(sd).filter(|f| f.url.is_some() && is_video_only(f)) {
            let h = f.height.unwrap_or(0);
            if let Some(s) = stream_from(f, MediaKind::Video, ua, Some(a.clone())) {
                map.insert(h, s);
            }
        }
    }
    map
}

fn best_audio_part(sd: &StreamingData, ua: &'static str) -> Option<MuxPart> {
    best_audio(sd, ua).map(|s| MuxPart {
        url: s.url,
        download_user_agent: s.download_user_agent,
    })
}

fn best_audio(sd: &StreamingData, ua: &'static str) -> Option<Stream> {
    all_formats(sd)
        .filter(|f| f.url.is_some() && is_audio(f))
        .max_by_key(|f| f.bitrate.unwrap_or(0))
        .and_then(|f| stream_from(f, MediaKind::Audio, ua, None))
}

fn stream_from(
    f: &Format,
    kind: MediaKind,
    ua: &'static str,
    mux: Option<MuxPart>,
) -> Option<Stream> {
    Some(Stream {
        url: Url::parse(f.url.as_ref()?).ok()?,
        kind,
        label: f
            .quality_label
            .clone()
            .or_else(|| f.itag.map(|t| t.to_string())),
        download_user_agent: Some(ua),
        mux_audio: mux,
        hls: false,
        download_referer: None,
    })
}

fn all_formats(sd: &StreamingData) -> impl Iterator<Item = &Format> {
    sd.formats
        .iter()
        .flatten()
        .chain(sd.adaptive_formats.iter().flatten())
}

fn is_muxed(f: &Format) -> bool {
    f.mime_type.as_deref().is_some_and(|m| {
        m.starts_with("video/") && (m.contains("mp4a") || f.audio_channels.is_some())
    })
}

fn is_video_only(f: &Format) -> bool {
    f.mime_type.as_deref().is_some_and(|m| {
        m.starts_with("video/") && !m.contains("mp4a") && f.audio_channels.is_none()
    })
}

fn is_audio(f: &Format) -> bool {
    f.mime_type
        .as_deref()
        .is_some_and(|m| m.starts_with("audio/"))
}

#[derive(Debug, Deserialize)]
pub struct PlayerResponse {
    #[serde(rename = "playabilityStatus")]
    pub playability: Playability,
    #[serde(rename = "streamingData")]
    pub streaming_data: Option<StreamingData>,
}

#[derive(Debug, Deserialize)]
pub struct Playability {
    pub status: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamingData {
    pub formats: Option<Vec<Format>>,
    #[serde(rename = "adaptiveFormats")]
    pub adaptive_formats: Option<Vec<Format>>,
}

#[derive(Debug, Deserialize)]
pub struct Format {
    pub url: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    #[serde(rename = "qualityLabel")]
    pub quality_label: Option<String>,
    pub itag: Option<u32>,
    pub height: Option<u32>,
    pub bitrate: Option<u32>,
    #[serde(rename = "audioChannels")]
    pub audio_channels: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::ExtractOptions;

    const UA: &str = "test-ua";

    fn response() -> PlayerResponse {
        serde_json::from_str(include_str!("../../tests/fixtures/youtube_innertube.json")).unwrap()
    }

    fn opts(quality: Quality) -> ExtractOptions {
        ExtractOptions {
            kinds: vec![MediaKind::Video],
            quality,
        }
    }

    #[test]
    fn best_picks_highest_with_mux() {
        let streams = select_for_kinds(&response(), &opts(Quality::Best), UA);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].label.as_deref(), Some("1080p"));
        assert!(streams[0].mux_audio.is_some());
    }

    #[test]
    fn all_returns_every_height() {
        let streams = select_for_kinds(&response(), &opts(Quality::All), UA);
        assert_eq!(streams.len(), 3); // 360 muxed, 720, 1080 adaptive
    }

    #[test]
    fn specific_height() {
        let streams = select_for_kinds(&response(), &opts(Quality::Height(720)), UA);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].label.as_deref(), Some("720p"));
    }
}
