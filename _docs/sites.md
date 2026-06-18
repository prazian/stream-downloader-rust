# Supported sites

## Yandex Video

Preview pages embed upstream players (VK, Rutube, YouTube, OK.ru); native VH URLs use the Yandex player API and HLS on `strm.yandex.ru`. HLS downloads use ffmpeg (same bundled binary as YouTube HD mux).

## OK.ru

Embed page metadata with per-quality progressive MP4 URLs. Large files use parallel range download.

## okX

Not OK.ru — KTPlayer pages expose HLS via a `<source>` gate URL. Max quality depends on the page (often 720p).

## PH

`flashvars_*` embed JSON with `mediaDefinitions`; remote entries resolve through `get_media`. Progressive MP4 preferred; falls back to HLS. CDN URLs are re-signed before download.

## XN / XVid

Shared `html5player` embed: low qualities as progressive MP4, 480p–1080p via HLS master (ffmpeg). Works on `xvideos.com` and regional XNXX domains. Page is re-fetched before download for fresh signed CDN URLs.

## Vimeo

Vimeo’s public player config is often blocked; uses the macOS app API (`api.vimeo.com`) for progressive MP4. Redirect links expire — refreshed before download. Unlisted/private embeds may need the `h` hash from the page URL or `og:video` meta.

## Examples

```bash
# Yandex
stream-dl -u "https://yandex.ru/video/preview/<video-id>" -o ~/Downloads
stream-dl -u "https://yandex.ru/portal/video?stream_id=<stream-id>" -o ~/Downloads
stream-dl -u "https://frontend.vh.yandex.ru/player/<video-id>" -o ~/Downloads

# OK.ru
stream-dl -u "https://ok.ru/video/<video-id>" -q 1080p -o ~/Downloads

# okX
stream-dl -u "https://ok.xxx/video/<video-id>/" -q 720p -o ~/Downloads

# PH
stream-dl -u "https://www.pornhub.com/view_video.php?viewkey=<video-key>" -q 1080p -o ~/Downloads

# XN / XVid
stream-dl -u "https://www.xnxx.com/video-1cpbmdea/<video-slug>" -q 1080p -o ~/Downloads

# Vimeo
stream-dl -u "https://vimeo.com/<video-id>?fl=wc" -q 1080p -o ~/Downloads
```
