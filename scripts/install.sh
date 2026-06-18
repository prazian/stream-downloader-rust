#!/usr/bin/env bash
# Install stream-dl and prefetch-tools from the latest GitHub release.
set -euo pipefail

REPO="${STREAM_DL_REPO:-prazian/stream-downloader-rust}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    target=x86_64-unknown-linux-gnu
    ext=tar.gz
    ;;
  Darwin-arm64)
    target=aarch64-apple-darwin
    ext=tar.gz
    ;;
  Darwin-x86_64)
    target=x86_64-apple-darwin
    ext=tar.gz
    ;;
  *)
    echo "error: unsupported platform $(uname -s) $(uname -m)" >&2
    echo "Download manually from https://github.com/${REPO}/releases" >&2
    exit 1
    ;;
esac

asset="stream-dl-${target}.${ext}"
api="https://api.github.com/repos/${REPO}/releases/latest"

if ! tag="$(curl -fsSL "$api" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")"; then
  echo "error: could not find a release at https://github.com/${REPO}/releases" >&2
  exit 1
fi

url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

echo "Installing ${tag} (${target}) to ${INSTALL_DIR}"
mkdir -p "$INSTALL_DIR"
curl -fsSL "$url" -o "${tmpdir}/${asset}"

if [[ "$ext" == tar.gz ]]; then
  tar -xzf "${tmpdir}/${asset}" -C "$tmpdir"
  install -m 755 "${tmpdir}/stream-dl" "${tmpdir}/prefetch-tools" "$INSTALL_DIR/"
fi

echo "Installed ${tag} to ${INSTALL_DIR}"
case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "Add to PATH: export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
