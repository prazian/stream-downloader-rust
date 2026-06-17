use crate::extract::profile::JsonMapRule;
use crate::extract::{ExtractOptions, infer_kind};
use crate::model::{MediaKind, Stream};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

const CIPHER_KEYS: &[&str] = &["signatureCipher", "cipher"];

pub fn apply_rule(html: &str, rule: &JsonMapRule, options: &ExtractOptions) -> Vec<Stream> {
    let Some(json) = extract_embedded_json(html, rule.marker) else {
        return Vec::new();
    };
    let Ok(root) = serde_json::from_str::<Value>(&json) else {
        return Vec::new();
    };

    let kinds: HashSet<_> = options.kinds.iter().copied().collect();
    let mut streams = Vec::new();

    for path in rule.item_paths {
        let Some(items) = value_at_path(&root, path).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            let Some(url) = file_url_from_item(item, rule.url_key) else {
                continue;
            };
            let Ok(parsed) = Url::parse(&url) else {
                continue;
            };
            let kind = rule
                .mime_key
                .and_then(|key| item.get(key))
                .and_then(Value::as_str)
                .map(mime_kind)
                .unwrap_or_else(|| infer_kind(&parsed, None));
            if !kinds.contains(&kind) {
                continue;
            }
            streams.push(Stream {
                url: parsed,
                kind,
                label: label_from_item(item, rule.label_key, rule.alt_label_key),
                download_user_agent: None,
                mux_audio: None,
                hls: false,
                download_referer: None,
            });
        }
    }

    streams
}

fn file_url_from_item(item: &Value, url_key: &str) -> Option<String> {
    if let Some(url) = item.get(url_key).and_then(Value::as_str) {
        return Some(url.to_owned());
    }
    for key in CIPHER_KEYS {
        let Some(cipher) = item.get(*key).and_then(Value::as_str) else {
            continue;
        };
        if let Some(url) = parse_cipher_url(cipher) {
            return Some(url);
        }
    }
    None
}

fn label_from_item(
    item: &Value,
    label_key: Option<&str>,
    alt_label_key: Option<&str>,
) -> Option<String> {
    label_key
        .and_then(|key| item.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            alt_label_key.and_then(|key| item.get(key)).map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string())
            })
        })
}

fn mime_kind(mime_type: &str) -> MediaKind {
    if mime_type
        .split(';')
        .next()
        .is_some_and(|mime| mime.starts_with("audio/"))
    {
        MediaKind::Audio
    } else {
        MediaKind::Video
    }
}

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

pub fn extract_embedded_json(html: &str, marker: &str) -> Option<String> {
    let needle = format!("{marker} = ");
    let start = html.find(&needle)? + needle.len();
    let rest = html.get(start..)?.trim_start();
    extract_json_object(rest)
}

fn extract_json_object(input: &str) -> Option<String> {
    if !input.starts_with('{') {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return input.get(..=index).map(str::to_owned);
                }
            }
            _ => {}
        }
    }

    None
}

fn parse_cipher_url(cipher: &str) -> Option<String> {
    url::form_urlencoded::parse(cipher.as_bytes())
        .find(|(key, _)| key == "url")
        .map(|(_, value)| value.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulls_object_after_marker() {
        let html = r#"var ytInitialPlayerResponse = {"streamingData":{"formats":[]}};"#;
        let json = extract_embedded_json(html, "ytInitialPlayerResponse").unwrap();
        assert!(json.contains("streamingData"));
    }
}
