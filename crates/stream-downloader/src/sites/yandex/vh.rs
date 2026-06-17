use crate::error::{Error, Result};
use crate::extract::ExtractOptions;
use crate::model::{MediaKind, Stream};
use crate::quality::Quality;
use serde::Deserialize;
use url::Url;

pub const REFERER: &str = "https://frontend.vh.yandex.ru/";

pub fn content_id(url: &Url) -> Option<String> {
    if url.host_str() == Some("frontend.vh.yandex.ru") {
        return url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .filter(|id| is_content_id(id))
            .map(str::to_owned);
    }
    url.query_pairs()
        .find(|(k, _)| k == "stream_id")
        .map(|(_, v)| v.into_owned())
        .filter(|id| is_content_id(id))
}

fn is_content_id(id: &str) -> bool {
    (id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit()))
        || ((9..=13).contains(&id.len())
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

pub async fn extract(
    client: &reqwest::Client,
    content_id: &str,
    title_hint: Option<&str>,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    if !options.kinds.contains(&MediaKind::Video) {
        return Ok(Vec::new());
    }

    let content = fetch_content(client, content_id).await?;
    let title = title_hint
        .map(str::to_owned)
        .or_else(|| content.title.clone())
        .or_else(|| content.computed_title.clone());

    let mut candidates = Vec::new();
    for stream in content.streams.iter().flatten() {
        if let Some(url) = stream.url.as_deref() {
            candidates.push((url.to_owned(), true));
        }
    }
    if let Some(url) = content.content_url.as_deref() {
        candidates.push((url.to_owned(), false));
    }

    let mut streams: Vec<Stream> = candidates
        .into_iter()
        .filter(|(url, _)| url.contains(".m3u8"))
        .map(|(url, signed)| Stream {
            url: Url::parse(&url).expect("vh url"),
            kind: MediaKind::Video,
            label: Some(if signed { "best".into() } else { "auto".into() }),
            download_user_agent: None,
            mux_audio: None,
            hls: true,
            download_referer: Some(REFERER),
        })
        .collect();

    if streams.is_empty() {
        return Err(Error::NoStreamsFound);
    }
    streams.sort_by_key(|b| std::cmp::Reverse(rank(b)));
    if let Some(title) = title {
        for s in &mut streams {
            s.label = Some(title.clone());
        }
    }
    Ok(match options.quality {
        Quality::All => streams,
        Quality::Best | Quality::Height(_) => vec![streams.remove(0)],
    })
}

fn rank(stream: &Stream) -> u8 {
    if stream.url.as_str().contains("ysign1=") {
        2
    } else {
        1
    }
}

async fn fetch_content(client: &reqwest::Client, content_id: &str) -> Result<VhContent> {
    let url = format!(
        "https://frontend.vh.yandex.ru/v23/player/{content_id}.json?stream_options=hires&disable_trackings=1"
    );
    let body: VhResponse = client.get(url).send().await?.error_for_status()?.json().await?;
    body.content.ok_or(Error::NoStreamsFound)
}

#[derive(Debug, Deserialize)]
struct VhResponse {
    content: Option<VhContent>,
}

#[derive(Debug, Deserialize)]
struct VhContent {
    computed_title: Option<String>,
    content_url: Option<String>,
    title: Option<String>,
    streams: Option<Vec<VhStream>>,
}

#[derive(Debug, Deserialize)]
struct VhStream {
    url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_content_ids() {
        let url = Url::parse("https://frontend.vh.yandex.ru/player/vIsS3AJqE7Y4").unwrap();
        assert_eq!(content_id(&url).as_deref(), Some("vIsS3AJqE7Y4"));
        let url = Url::parse("https://yandex.ru/portal/video?stream_id=4dbb36ec4e0526d58f9f2dc8f0ecf374")
            .unwrap();
        assert_eq!(
            content_id(&url).as_deref(),
            Some("4dbb36ec4e0526d58f9f2dc8f0ecf374")
        );
    }

    #[test]
    fn picks_best_signed_stream_from_fixture() {
        let body: VhResponse =
            serde_json::from_str(include_str!("../../../tests/fixtures/yandex_vh_player.json"))
                .unwrap();
        let content = body.content.unwrap();
        let signed = content.streams.as_ref().unwrap()[0].url.clone().unwrap();
        let stream = Stream {
            url: Url::parse(&signed).unwrap(),
            kind: MediaKind::Video,
            label: Some("best".into()),
            download_user_agent: None,
            mux_audio: None,
            hls: true,
            download_referer: Some(REFERER),
        };
        assert!(stream.url.as_str().contains("ysign1="));
    }
}
