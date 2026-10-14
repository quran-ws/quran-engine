#!/usr/bin/env bash
# Fetch the test data that the gates need, into the gitignored paths the tests read.
#
#   dist/pages/   the built page data (NNN.qvp, atlas.qva, NNN.words.json) from the data
#                 release on GitHub; needed by the ABI test and every wrapper test.
#                 Each directory records its tag, and a tag that does not match is refetched.
#                 A locally built dist/pages records no tag, so set QVP_KEEP_PAGES=1 to keep it.
#   pages/ index/ the source SVG bundle from the quran-svg-elements release (hafs-kfgqpc);
#                 needed by the identity gate.
#
# Environment:
#   QVP_DATA_TAG   data release tag (default data-v0.4.0)
#   QVP_SVG_TAG    quran-svg-elements release tag (default v1.1.2)
#   QVP_KEEP_PAGES keep dist/pages as it is, whatever built it
# Output: the directories above; a line per artifact saying fetched, kept or skipped.
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${QVP_DATA_TAG:-data-v0.4.0}"
pages_fetched=""
asset="quran-engine-pages-hafs-kfgqpc.tar.gz"
base="https://github.com/quran-ws/quran-engine/releases/download/$tag"

if [ -n "${QVP_KEEP_PAGES:-}" ] && [ -f dist/pages/atlas.qva ]; then
  echo "skipped  dist/pages (QVP_KEEP_PAGES is set)"
elif [ -f dist/pages/atlas.qva ] && [ "$(ls dist/pages/*.qvp 2>/dev/null | wc -l)" -ge 604 ] \
   && [ "$(cat dist/pages/.tag 2>/dev/null)" = "$tag" ]; then
  echo "kept     dist/pages ($tag, 604 pages present)"
else
  mkdir -p dist
  curl --fail --location --retry 3 --silent --show-error "$base/$asset" --output "dist/$asset"
  curl --fail --location --retry 3 --silent --show-error "$base/$asset.sha256" --output "dist/$asset.sha256"
  (cd dist && shasum -a 256 -c "$asset.sha256" >/dev/null)
  rm -rf dist/pages && mkdir -p dist/pages
  tar -xzf "dist/$asset" -C dist/pages --strip-components=1
  echo "$tag" > dist/pages/.tag
  pages_fetched=1
  echo "fetched  dist/pages ($tag, checksum verified)"
fi

# The ornament set. A data tarball that carries it has its checksum verified with the
# tarball, so use that copy. An older release can have the set only as a separate asset,
# or have none. Only HTTP 404 means none. Any other failure stops the script.
if [ -n "${QVP_KEEP_PAGES:-}" ] && [ -f dist/pages/atlas.qva ]; then
  echo "skipped  dist/pages/ornaments.qvo (QVP_KEEP_PAGES is set)"
elif [ -f dist/pages/ornaments.qvo ]; then
  if [ -n "$pages_fetched" ]; then
    echo "fetched  dist/pages/ornaments.qvo ($tag, from the data tarball)"
  else
    echo "kept     dist/pages/ornaments.qvo ($tag)"
  fi
else
  part="dist/pages/ornaments.qvo.part"
  status="$(curl --location --retry 3 --silent --show-error --write-out '%{http_code}' \
    "$base/ornaments.qvo" --output "$part")" || {
    rm -f "$part"
    echo "error: could not fetch $base/ornaments.qvo" >&2
    exit 1
  }
  case "$status" in
    200) mv "$part" dist/pages/ornaments.qvo
         echo "fetched  dist/pages/ornaments.qvo ($tag)" ;;
    404) rm -f "$part"
         echo "skipped  dist/pages/ornaments.qvo ($tag has none)" ;;
    *)   rm -f "$part"
         echo "error: $base/ornaments.qvo returned HTTP $status" >&2
         exit 1 ;;
  esac
fi

svg_tag="${QVP_SVG_TAG:-v1.1.2}"
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
