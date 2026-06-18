#!/usr/bin/env bash
set -euo pipefail

kind="${1:-}"
if [[ "$kind" != "minor" && "$kind" != "major" ]]; then
  echo "usage: $0 minor|major" >&2
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "error: working tree is not clean" >&2
  exit 1
fi

cd "$(dirname "$0")/.."

latest="$(git tag -l 'V[0-9]*.[0-9]*' --sort=-v:refname | head -1 || true)"
if [[ -z "$latest" ]]; then
  major=0
  minor=0
else
  ver="${latest#V}"
  major="${ver%%.*}"
  minor="${ver#*.}"
  minor="${minor%%.*}"
fi

case "$kind" in
  minor) minor=$((minor + 1)) ;;
  major)
    major=$((major + 1))
    minor=0
    ;;
esac

tag="V${major}.${minor}"
version="${major}.${minor}.0"

if git rev-parse "$tag" >/dev/null 2>&1; then
  echo "error: tag $tag already exists" >&2
  exit 1
fi

perl -pi -e 's/^version = ".*"/version = "'"$version"'"/' Cargo.toml

if ! git diff --quiet Cargo.toml; then
  git add Cargo.toml
  git commit -m "Release $tag"
fi

git tag -a "$tag" -m "Release $tag"
echo "Tagged $tag. Push: git push && git push origin $tag"
