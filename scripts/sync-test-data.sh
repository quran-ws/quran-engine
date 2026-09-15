#!/usr/bin/env bash
# Fetch the test data that the gates need, into the gitignored paths the tests read.
#
#   dist/pages/   the built page data (NNN.qvp, atlas.qva, NNN.words.json) from the data
#                 release on GitHub; needed by the ABI test and every wrapper test.
#   pages/ index/ the source SVG bundle from the quran-svg-elements release (hafs-kfgqpc);
#                 needed by the identity gate.
#
# Environment:
#   QVP_DATA_TAG   data release tag (default data-v0.2.0)
#   QVP_SVG_TAG    quran-svg-elements release tag (default v1.1.1)
# Output: the directories above; a line per artifact saying fetched, kept or skipped.
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${QVP_DATA_TAG:-data-v0.2.0}"
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

svg_tag="${QVP_SVG_TAG:-v1.1.1}"
svg_asset="quran-svg-elements-hafs-kfgqpc.tar.gz"
svg_base="https://github.com/quran-ws/quran-svg-elements/releases/download/$svg_tag"

if [ -f pages/001.svg ] && [ -d index/by-page ] && [ "$(cat pages/.tag 2>/dev/null)" = "$svg_tag" ]; then
  echo "kept     pages/ and index/ ($svg_tag)"
else
  mkdir -p dist
  curl --fail --location --retry 3 --silent --show-error "$svg_base/$svg_asset" --output "dist/$svg_asset"
  curl --fail --location --retry 3 --silent --show-error "$svg_base/$svg_asset.sha256" --output "dist/$svg_asset.sha256"
  (cd dist && shasum -a 256 -c "$svg_asset.sha256" >/dev/null)
  rm -rf pages index
  tar -xzf "dist/$svg_asset" -C . --strip-components=1 --include='*/pages/*' --include='*/index/*' 2>/dev/null \
    || tar -xzf "dist/$svg_asset" -C . --strip-components=1 --wildcards '*/pages/*' '*/index/*'
  [ -f pages/001.svg ] || { echo "error: the bundle did not contain pages/001.svg" >&2; exit 1; }
  echo "$svg_tag" > pages/.tag
  echo "fetched  pages/ and index/ ($svg_tag, checksum verified)"
fi
