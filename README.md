# stream-downloader

Find and download media **files** delivered over HTTP (what players call a “stream” is a file URL).

```bash
make install
make prefetch-tools   # optional: warm ffmpeg cache before first HD download
```

## CLI

```bash
stream-dl -u URL [-o DIR]              # best quality (default)
stream-dl -u URL -q 720p               # specific resolution
stream-dl -u URL -q 1080p
stream-dl -u URL -q 4k
stream-dl -u URL --all                 # every available resolution
stream-dl -u URL --audio               # audio only
```

| Flag | Description |
|------|-------------|
| `-u`, `--url` | Page URL (required) |
| `-o`, `--out` | Output directory (default: current directory) |
| `-q`, `--quality` | `720p`, `1080p`, `4k`, etc. (conflicts with `--all`) |
| `--all` | Download all resolutions (conflicts with `-q`) |
| `--audio` | Download audio track only |

**Quality:** omit flags for **best** (highest available). `-q` and `--all` are mutually exclusive.

## Development

```bash
make help
make test
make lint
```

## Why two crates?

| Crate | Role |
|-------|------|
| **`stream-downloader`** | Library — discovery, naming, download, merge |
| **`stream-dl`** | CLI + `prefetch-tools` binary |

## Architecture

| Layer | Role |
|-------|------|
| `quality.rs` | `Best` / `All` / `Height` — shared quality selection |
| `extract/generic.rs` + `profiles.rs` | Simple sites (HTML/CSS/JSON rules) |
| `innertube/` | Shared YouTube-style player API client |
| `sites/youtube.rs` | YouTube innertube client |
| `sites/yandex/` | Yandex preview + VH player API |
| `merge.rs` | ffmpeg mux + HLS download (`ffmpeg-sidecar`) |

New complex sites: add `sites/foo.rs` and reuse `innertube` or `profiles` as fits.

## YouTube

**Discovery**

1. Fetch the watch page.
2. Read `INNERTUBE_API_KEY` and visitor data from the HTML.
3. Call YouTube’s innertube player API (`POST /youtubei/v1/player`) posing as the **Android VR** app.
4. Use the direct `googlevideo.com` URLs from that response (with the same User-Agent on download).

**What gets downloaded**

| Resolution | What YouTube gives us | What `stream-dl` does |
|------------|----------------------|------------------------|
| Low (e.g. 360p) | One file, video + audio together | Single HTTP download → `.mp4` |
| 720p, 1080p, 4K | Separate video and audio URLs | Download both → **ffmpeg** mux (`-c copy`) → one `.mp4` |

**ffmpeg:** provided by the **`ffmpeg-sidecar`** crate (bundled binary, downloaded to `~/.ffmpeg-sidecar` on first HD download). We do **not** call `yt-dlp` or your system `ffmpeg`. `make prefetch-tools` fetches it early; `make install-deps-mac` is optional and unused by the tool today.

## Yandex Video

```bash
stream-dl -u "https://yandex.ru/video/preview/18025183134392203214"
stream-dl -u "https://yandex.ru/portal/video?stream_id=4dbb36ec4e0526d58f9f2dc8f0ecf374"
stream-dl -u "https://frontend.vh.yandex.ru/player/vIsS3AJqE7Y4"
```

| URL type | What happens |
|----------|----------------|
| `/video/preview/{id}` | Parse embed JSON → follow upstream (VK, Rutube, YouTube, …) |
| `stream_id=` / `frontend.vh.yandex.ru` | Yandex VH player API → HLS on `strm.yandex.ru` |

HLS streams download via ffmpeg (same bundled binary as YouTube HD mux).

Upstream hosts from Yandex previews: **OK.ru**, VK, Rutube, YouTube, native Yandex VH.
