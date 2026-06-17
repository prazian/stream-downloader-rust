use crate::error::{Error, Result};
use crate::innertube::config::ClientConfig;
use crate::innertube::formats::PlayerResponse;
use crate::innertube::PageKeys;

pub async fn player(
    http: &reqwest::Client,
    keys: &PageKeys,
    client: ClientConfig,
    video_id: &str,
) -> Result<PlayerResponse> {
    let body = serde_json::json!({
        "context": { "client": client.client_json() },
        "videoId": video_id,
    });

    let response = http
        .post(format!(
            "https://www.youtube.com/youtubei/v1/player?key={}",
            keys.api_key
        ))
        .header("Content-Type", "application/json")
        .header("User-Agent", client.user_agent)
        .header("X-Goog-Visitor-Id", &keys.visitor_data)
        .header("Origin", "https://www.youtube.com")
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    let data = response.json::<PlayerResponse>().await?;
    if data.playability.status.as_deref() != Some("OK") {
        let reason = data
            .playability
            .reason
            .unwrap_or_else(|| "video unavailable".into());
        return Err(Error::InvalidUrl(reason));
    }
    Ok(data)
}
