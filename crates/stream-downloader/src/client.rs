pub(crate) const BROWSER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// GET with browser User-Agent and optional Referer (CDN hotlink checks).
pub(crate) fn browser_get(
    client: &reqwest::Client,
    url: impl reqwest::IntoUrl,
    referer: &str,
) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header(reqwest::header::REFERER, referer)
        .header(reqwest::header::USER_AGENT, BROWSER_UA)
}

pub(crate) fn build() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(BROWSER_UA)
        .cookie_store(true)
        .pool_max_idle_per_host(16)
        .tcp_nodelay(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .expect("reqwest client")
}
