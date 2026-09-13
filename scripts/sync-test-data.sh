#!/usr/bin/env bash
# Fetch the test data that the gates need, into the gitignored paths the tests read.
#
#   dist/pages/   the built page data (NNN.qvp, atlas.qva, NNN.words.json) from the data
#                 release on GitHub; needed by the ABI test and every wrapper test.
#   pages/ index/ the source SVG bundle (quran-svg, hafs-kfgqpc); needed by the identity
#                 gate. It is not published as a release asset yet, so it is fetched only
#                 when QVP_SVG_BUNDLE_URL points at a tarball with pages/ and index/ inside.
#
# Environment:
#   QVP_DATA_TAG        data release tag (default v0.1.0)
#   QVP_SVG_BUNDLE_URL  URL of the source bundle tarball (optional)
# Output: the directories above; a line per artifact saying fetched, kept or skipped.
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${QVP_DATA_TAG:-v0.1.0}"
asset="quran-engine-pages-hafs-kfgqpc.tar.gz"
base="https://github.com/quran-ws/quran-engine/releases/download/$tag"

if [ -f dist/pages/atlas.qva ] && [ "$(ls dist/pages/*.qvp 2>/dev/null | wc -l)" -ge 604 ]; then
  echo "kept     dist/pages (604 pages present)"
else
  mkdir -p dist
  curl --fail --location --retry 3 --silent --show-error "$base/$asset" --output "dist/$asset"
  curl --fail --location --retry 3 --silent --show-error "$base/$asset.sha256" --output "dist/$asset.sha256"
  (cd dist && shasum -a 256 -c "$asset.sha256" >/dev/null)
  rm -rf dist/pages && mkdir -p dist/pages
  tar -xzf "dist/$asset" -C dist/pages --strip-components=1
  echo "fetched  dist/pages ($tag, checksum verified)"
fi

if [ -f pages/001.svg ] && [ -d index/by-page ]; then
  echo "kept     pages/ and index/"
elif [ -n "${QVP_SVG_BUNDLE_URL:-}" ]; then
  tmp="$(mktemp -t qvp-svg.XXXXXX)"
  trap 'rm -f "$tmp"' EXIT
  curl --fail --location --retry 3 --silent --show-error "$QVP_SVG_BUNDLE_URL" --output "$tmp"
  rm -rf pages index
  tar -xzf "$tmp" -C .
  [ -f pages/001.svg ] || { echo "error: the bundle did not contain pages/001.svg" >&2; exit 1; }
  echo "fetched  pages/ and index/ (source bundle)"
else
  echo "skipped  pages/ and index/: set QVP_SVG_BUNDLE_URL to fetch the source bundle; the identity gate skips without it"
fi
