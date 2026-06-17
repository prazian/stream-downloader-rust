use stream_downloader::yandex;
use url::Url;

#[test]
fn yandex_preview_host_matching() {
    let url = Url::parse("https://yandex.ru/video/preview/18025183134392203214").unwrap();
    assert!(yandex::matches_host(&url));
    let url = Url::parse("https://yandex.com/video/preview/?filmId=42").unwrap();
    assert!(yandex::matches_host(&url));
    let url = Url::parse("https://frontend.vh.yandex.ru/player/vIsS3AJqE7Y4").unwrap();
    assert!(yandex::matches_host(&url));
}

#[test]
fn yandex_film_id_parsing() {
    let url = Url::parse("https://yandex.ru/video/preview/18025183134392203214").unwrap();
    assert_eq!(
        yandex::film_id(&url).as_deref(),
        Some("18025183134392203214")
    );
}
