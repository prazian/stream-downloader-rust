# stream-downloader

Download media **files** from page URLs (Rust library + CLI).

> **Disclaimer:** Educational demo only—not for unauthorized downloads. See [DISCLAIMER.md](DISCLAIMER.md).

## Setup

Run in order:

```bash
make install          # required — installs stream-dl to ~/.cargo/bin
make prefetch-tools   # optional — bundled ffmpeg (YouTube 720p+, HLS sites)
```

Then download:

```bash
stream-dl -u "https://www.youtube.com/watch?v=…" -o ~/Downloads
```

**ffmpeg:** `stream-dl` uses a bundled binary (`ffmpeg-sidecar`), not Homebrew or system ffmpeg. Skip `prefetch-tools` if you like — the first download that needs ffmpeg fetches it to `~/.ffmpeg-sidecar` automatically.

## Usage

```bash
stream-dl -u URL                    # best quality
stream-dl -u URL -q 1080p -o DIR    # specific quality
stream-dl -u URL --all              # every resolution
```

| Flag | Description |
|------|-------------|
| `-u`, `--url` | Page URL (required) |
| `-o`, `--out` | Output directory (default: `.`) |
| `-q`, `--quality` | e.g. `720p`, `1080p`, `4k` |
| `--all` | All resolutions (`-q` and `--all` exclude each other) |
| `--audio` | Audio only |

Site examples: [_docs/sites.md](_docs/sites.md)

## YouTube (how it works)

1. Fetch watch page → read innertube keys from HTML.
2. Call innertube player API as Android VR → get `googlevideo.com` URLs.
3. 360p and below: one progressive file. 720p+: separate video + audio, muxed with ffmpeg.

## Development

```bash
make test
make lint
```

**Layout:** `stream-downloader` (library), `stream-dl` (CLI). Sites register in `sites/registry.rs`; add a module + one `SitePlugin` row. Details in the source and `_docs/sites.md`.

## Release

Tags look like `V0.1`, `V0.2`, `V1.0`. The latest tag sets the next version (if none exists, the first release is `V0.1`).

```bash
make release-minor   # V0.1 → V0.2
make release-major   # V0.2 → V1.0
git push && git push origin V0.2   # use the tag printed by the command
```

Pushing a `V*.*` tag runs [`.github/workflows/release.yml`](.github/workflows/release.yml) — builds `stream-dl` and `prefetch-tools` for Linux, macOS, and Windows and publishes a GitHub release.

## License

MIT — [LICENSE](LICENSE)
