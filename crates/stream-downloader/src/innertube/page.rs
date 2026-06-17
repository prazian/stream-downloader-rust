use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct PageKeys {
    pub api_key: String,
    pub visitor_data: String,
}

impl PageKeys {
    pub fn from_html(html: &str) -> Result<Self> {
        Ok(Self {
            api_key: extract_json_string(html, "INNERTUBE_API_KEY")?,
            visitor_data: extract_json_string(html, "VISITOR_DATA")?,
        })
    }
}

fn extract_json_string(html: &str, key: &str) -> Result<String> {
    let needle = format!("\"{key}\":\"");
    let start = html
        .find(&needle)
        .ok_or_else(|| Error::InvalidUrl(format!("page missing `{key}`")))?
        + needle.len();
    let rest = html
        .get(start..)
        .ok_or_else(|| Error::InvalidUrl("truncated page".into()))?;
    let end = rest
        .find('"')
        .ok_or_else(|| Error::InvalidUrl("truncated page".into()))?;
    Ok(rest[..end].to_owned())
}
