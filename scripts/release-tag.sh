#!/usr/bin/env bash
# Bump version from the latest V{major}.{minor} tag, update Cargo.toml, commit, and tag.
# Push the commit and tag yourself to trigger the GitHub release workflow.
set -euo pipefail

kind="${1:-}"
if [[ "$kind" != "minor" && "$kind" != "major" ]]; then
  echo "usage: $0 minor|major" >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "error: working tree is not clean; commit or stash changes first" >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

latest="$(git tag -l 'V[0-9]*.[0-9]*' --sort=-v:refname | head -1 || true)"
if [[ -z "$latest" ]]; then
  major=0
  minor=0
else
  ver="${latest#V}"
  major="${ver%%.*}"
  rest="${ver#*.}"
  minor="${rest%%.*}"
fi

case "$kind" in
  minor) minor=$((minor + 1)) ;;
  major)
    major=$((major + 1))
    minor=0
    ;;
esac

tag="V${major}.${minor}"
cargo_version="${major}.${minor}.0"

for manifest in crates/stream-dl/Cargo.toml crates/stream-downloader/Cargo.toml; do
  perl -pi -e 's/^version = ".*"/version = "'"$cargo_version"'"/' "$manifest"
done

git add crates/stream-dl/Cargo.toml crates/stream-downloader/Cargo.toml
git commit -m "Release ${tag}"
git tag -a "${tag}" -m "Release ${tag}"

cat <<EOF
Created release ${tag} (crate version ${cargo_version}).

Push to trigger GitHub Actions:
  git push && git push origin ${tag}
EOF
