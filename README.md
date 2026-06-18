# stream-downloader

Find and download media **files** delivered over HTTP (what players call a “stream” is a file URL).

> **Disclaimer:** This project is for **educational and technical demonstration** only—studying Rust, HTTP media extraction, and site-specific patterns. You must **not** use it for illegal activity or to download content you are not authorized to access. See [DISCLAIMER.md](DISCLAIMER.md) for the full terms.

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

```
fetch page → sites/registry (SitePlugin table) → quality → optional refresh → download
          ↘ extract/generic + profiles (fallback for simple HTML rules)
```

| Layer | Role |
|-------|------|
| `sites/registry.rs` | **`SITES` table** — one row per host family; drives extract + refresh |
| `sites/plugin.rs` | `SitePlugin` type (`matches`, `extract`, optional `refresh`) |
| `sites/common.rs` | Shared stream builders, HLS helpers, CDN refresh matching |
| `sites/patterns/` | Reusable embed parsers (e.g. `html5player` for XNXX/XVideos) |
| `extract/profiles.rs` | Declarative rules for trivial sites (CSS/JSON in HTML) |
| `innertube/` | YouTube-style player API client |

### Adding a site

1. **Trivial** (static URLs in HTML): add a `SiteProfile` row in `extract/profiles.rs`.
2. **Shared embed** (same JS player on many hosts): add a pattern under `sites/patterns/`, thin wrapper in `sites/foo.rs`.
3. **Custom API**: implement `matches_host` + `extract` in `sites/foo.rs`, register one line in `sites/registry.rs::SITES`.

No changes to `lib.rs` or `extract/engine.rs` needed. Set `refresh` only when CDN URLs expire (Pornhub, XNXX).

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

More supported sites: [_docs/sites.md](_docs/sites.md).

## License

MIT — see [LICENSE](LICENSE).
