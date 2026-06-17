/// Innertube client identity (see yt-dlp `INNERTUBE_CLIENTS`).
#[derive(Debug, Clone, Copy)]
pub struct ClientConfig {
    pub name: &'static str,
    pub version: &'static str,
    pub user_agent: &'static str,
    pub device_make: &'static str,
    pub device_model: &'static str,
    pub android_sdk: u32,
    pub os_name: &'static str,
    pub os_version: &'static str,
}

impl ClientConfig {
    pub fn client_json(self) -> serde_json::Value {
        serde_json::json!({
            "clientName": self.name,
            "clientVersion": self.version,
            "deviceMake": self.device_make,
            "deviceModel": self.device_model,
            "androidSdkVersion": self.android_sdk,
            "userAgent": self.user_agent,
            "osName": self.os_name,
            "osVersion": self.os_version,
        })
    }
}
