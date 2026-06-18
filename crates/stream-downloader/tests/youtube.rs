use stream_downloader::youtube;
use url::Url;

#[test]
fn youtube_video_id_from_watch_and_short_urls() {
    assert_eq!(
        youtube::video_id(
            &Url::parse("https://www.youtube.com/watch?v=p2xqGFEO9CY").unwrap()
        )
        .as_deref(),
        Some("p2xqGFEO9CY")
    );
    assert_eq!(
        youtube::video_id(&Url::parse("https://youtu.be/p2xqGFEO9CY").unwrap())
            .as_deref(),
        Some("p2xqGFEO9CY")
    );
}

#[test]
fn youtube_host_matching() {
    assert!(youtube::matches_host(
        &Url::parse("https://www.youtube.com/watch?v=x").unwrap()
    ));
}
