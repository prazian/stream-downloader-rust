# stream-downloader

Download media files from a page URL.

> **Disclaimer:** For learning and authorized use only. Do not download content you are not allowed to access. See [DISCLAIMER.md](DISCLAIMER.md).

## Install

### From a release

No Rust or compiler needed. Pick one:

**macOS / Linux — install script**

```bash
curl -fsSL https://raw.githubusercontent.com/prazian/stream-downloader-rust/master/scripts/install.sh | bash
```

This installs `stream-dl` and `prefetch-tools` to `~/.local/bin`. If the command is not found afterward, add that directory to your `PATH`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

**Manual download**

1. Open [Releases](https://github.com/prazian/stream-downloader-rust/releases) and pick the archive for your system:
   - **Linux (x86_64):** `stream-dl-x86_64-unknown-linux-gnu.tar.gz`
   - **macOS (Apple Silicon):** `stream-dl-aarch64-apple-darwin.tar.gz`
   - **macOS (Intel):** `stream-dl-x86_64-apple-darwin.tar.gz`
   - **Windows:** `stream-dl-x86_64-pc-windows-msvc.zip`
2. Extract the archive.
3. Put `stream-dl` somewhere on your `PATH` (or run it from the folder you extracted).

**ffmpeg (YouTube 720p+ and some sites)**  
`stream-dl` downloads a bundled ffmpeg automatically the first time it is needed (`~/.ffmpeg-sidecar`). To fetch it up front, run `prefetch-tools` from the same install (included in every release archive).

### From source

For hacking on the project. Requires [Rust](https://rustup.rs/).

```bash
git clone https://github.com/prazian/stream-downloader-rust.git
cd stream-downloader-rust
make install          # installs stream-dl to ~/.cargo/bin
make prefetch-tools   # optional — fetch bundled ffmpeg early
```

## Usage

```bash
stream-dl -u "https://www.youtube.com/watch?v=…" -o ~/Downloads
stream-dl -u URL -q 1080p -o DIR
stream-dl -u URL --all
```

| Flag | Description |
|------|-------------|
| `-u`, `--url` | Page URL (required) |
| `-o`, `--out` | Output directory (default: current folder) |
| `-q`, `--quality` | e.g. `720p`, `1080p`, `4k` |
| `--all` | Every available resolution |
| `--audio` | Audio only |

Omit `-q` and `--all` to get the best quality available.

Example URLs: [_docs/sites.md](_docs/sites.md)

## For developers

```bash
make test
make lint
```

`stream-downloader` is the library; `stream-dl` is the CLI. New sites go in `sites/` and one row in `sites/registry.rs`.

**Releasing**

```bash
make release-minor   # e.g. V0.1 → V0.2
make release-major   # e.g. V0.2 → V1.0
git push && git push origin V0.2
```

Pushing a `V*.*` tag builds release binaries via GitHub Actions.

## License

MIT — [LICENSE](LICENSE)
