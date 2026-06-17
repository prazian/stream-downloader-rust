use url::Url;

/// Companion audio track to mux with a video-only file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuxPart {
    pub url: Url,
    pub download_user_agent: Option<&'static str>,
}

/// Media kind filter for discovery. Audio is reserved for a later phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind {
    Video,
    Audio,
}

impl MediaKind {
    pub const ALL: &'static [MediaKind] = &[MediaKind::Video, MediaKind::Audio];

    pub fn file_extensions(self) -> &'static [&'static str] {
        match self {
            MediaKind::Video => &["mp4", "webm", "mkv", "m4v", "mov"],
            MediaKind::Audio => &["mp3", "m4a", "aac", "ogg", "wav", "flac"],
        }
    }
}

/// Metadata fetched from the source page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    pub url: Url,
    pub title: String,
}

/// HTML body plus page metadata from a single fetch.
#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub info: PageInfo,
    pub html: String,
}

/// A direct URL to a media file on a page.
///
/// Sites rarely expose a plain `<video src>`; they embed file URLs in JSON or
/// attributes. The browser downloads the file progressively — this is not live streaming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub url: Url,
    pub kind: MediaKind,
    /// Optional label from HTML (`<source label>` or nearby text).
    pub label: Option<String>,
    /// Per-file User-Agent when the CDN expects a specific client (e.g. YouTube).
    pub download_user_agent: Option<&'static str>,
    /// When set, download and mux with this audio (1080p+ YouTube, etc.).
    pub mux_audio: Option<MuxPart>,
}
