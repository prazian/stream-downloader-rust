use crate::error::{Error, Result};
use crate::model::FetchedPage;
use serde_json::Value;
use url::Url;

const PLAYER_PRIORITY: &[&str] = &["yandex", "vh", "rutube", "youtube", "ok", "zen", "vk"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSource {
    pub url: Url,
    pub title: String,
}

pub fn film_id(url: &Url) -> Option<String> {
    if let Some(id) = url
        .query_pairs()
        .find(|(k, _)| k == "filmId")
        .map(|(_, v)| v.into_owned())
    {
        return Some(id);
    }
    let segment = url.path().rsplit('/').next()?;
    segment
        .chars()
        .all(|c| c.is_ascii_digit())
        .then(|| segment.to_owned())
        .filter(|s| !s.is_empty())
}

pub fn resolve(page: &FetchedPage) -> Result<ResolvedSource> {
    let film_id = film_id(&page.info.url)
        .ok_or_else(|| Error::InvalidUrl("missing Yandex film id".into()))?;
    let json = noframes_json(&page.html)?;
    let dups = json
        .pointer("/clips/dups")
        .and_then(Value::as_object)
        .filter(|m| !m.is_empty())
        .ok_or_else(|| Error::InvalidUrl("video unavailable or geo-restricted".into()))?;

    let entry = pick_dup(dups, &film_id).ok_or_else(|| Error::NoStreamsFound)?;
    let href = entry
        .pointer("/host/href")
        .or_else(|| entry.pointer("/player/videoUrl"))
        .and_then(Value::as_str)
        .ok_or_else(|| Error::NoStreamsFound)?;
    let title = entry
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(&page.info.title)
        .to_owned();
    let url = Url::parse(href).map_err(|e| Error::InvalidUrl(e.to_string()))?;
    Ok(ResolvedSource { url, title })
}

fn pick_dup<'a>(dups: &'a serde_json::Map<String, Value>, film_id: &str) -> Option<&'a Value> {
    if let Some(v) = dups.get(film_id) {
        return Some(v);
    }
    let mut best: Option<(&str, &Value, usize)> = None;
    for v in dups.values() {
        let player = v
            .pointer("/host/playerId")
            .or_else(|| v.pointer("/player/playerId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let rank = PLAYER_PRIORITY
            .iter()
            .position(|p| player.eq_ignore_ascii_case(p))
            .unwrap_or(PLAYER_PRIORITY.len());
        if best.is_none_or(|(_, _, r)| rank < r) {
            best = Some((player, v, rank));
        }
    }
    best.map(|(_, v, _)| v)
}

fn noframes_json(html: &str) -> Result<Value> {
    let start_tag = html
        .find("<noframes")
        .ok_or_else(|| Error::InvalidUrl("page missing Yandex embed data".into()))?;
    let json_start = html[start_tag..]
        .find('>')
        .map(|i| start_tag + i + 1)
        .ok_or_else(|| Error::InvalidUrl("truncated Yandex embed".into()))?;
    let json_end = html[json_start..]
        .find("</noframes>")
        .map(|i| json_start + i)
        .ok_or_else(|| Error::InvalidUrl("truncated Yandex embed".into()))?;
    serde_json::from_str(html.get(json_start..json_end).unwrap_or(""))
        .map_err(|e| Error::InvalidUrl(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PageInfo;

    #[test]
    fn parses_film_id_from_path_and_query() {
        let url = Url::parse("https://yandex.ru/video/preview/18025183134392203214").unwrap();
        assert_eq!(film_id(&url).as_deref(), Some("18025183134392203214"));
        let url = Url::parse("https://yandex.com/video/preview/?filmId=42").unwrap();
        assert_eq!(film_id(&url).as_deref(), Some("42"));
    }

    #[test]
    fn resolves_upstream_from_fixture() {
        let html = include_str!("../../../tests/fixtures/yandex_preview.html");
        let page = FetchedPage {
            info: PageInfo {
                url: Url::parse("https://yandex.ru/video/preview/18025183134392203214").unwrap(),
                title: "preview".into(),
            },
            html: html.to_string(),
        };
        let source = resolve(&page).unwrap();
        assert_eq!(source.url.host_str(), Some("vk.com"));
    }
}
