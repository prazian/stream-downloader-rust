//! YouTube site plugin — config only; logic lives in `innertube` + `merge`.

use crate::error::Result;
use crate::extract::ExtractOptions;
use crate::innertube::{self, ClientConfig};
use crate::model::{FetchedPage, Stream};
use url::Url;

const USER_AGENT: &str = "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip";

const CLIENT: ClientConfig = ClientConfig {
    name: "ANDROID_VR",
    version: "1.65.10",
    user_agent: USER_AGENT,
    device_make: "Oculus",
    device_model: "Quest 3",
    android_sdk: 32,
    os_name: "Android",
    os_version: "12L",
};

pub fn matches_host(url: &Url) -> bool {
    url.host_str().is_some_and(|h| {
        h == "youtu.be" || h.ends_with("youtube.com") || h.ends_with("youtube-nocookie.com")
    })
}

pub fn video_id(url: &Url) -> Option<String> {
    if url.host_str() == Some("youtu.be") {
        return url
            .path_segments()
            .and_then(|mut s| s.next())
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
    }
    url.query_pairs()
        .find(|(k, _)| k == "v")
        .map(|(_, v)| v.into_owned())
        .filter(|id| !id.is_empty())
}

pub async fn extract(
    client: &reqwest::Client,
    page: &FetchedPage,
    options: &ExtractOptions,
) -> Result<Vec<Stream>> {
    let id = video_id(&page.info.url)
        .ok_or_else(|| crate::error::Error::InvalidUrl("missing YouTube video id".into()))?;
    let keys = innertube::PageKeys::from_html(&page.html)?;
    let response = innertube::player(client, &keys, CLIENT, &id).await?;
    Ok(innertube::select_for_kinds(&response, options, USER_AGENT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_watch_and_short_urls() {
        assert_eq!(
            video_id(&Url::parse("https://www.youtube.com/watch?v=abc").unwrap()).as_deref(),
            Some("abc")
        );
        assert_eq!(
            video_id(&Url::parse("https://youtu.be/abc").unwrap()).as_deref(),
            Some("abc")
        );
    }
}
