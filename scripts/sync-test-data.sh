#!/usr/bin/env bash
# Fetch the test data that the gates need, into the gitignored paths the tests read.
#
#   dist/pages/   the built page data (NNN.qvp, atlas.qva, NNN.words.json) from the data
#                 release on GitHub; needed by the ABI test and every wrapper test.
#                 Each directory records its tag, and a tag that does not match is refetched.
#                 A locally built dist/pages records no tag, so set QVP_KEEP_PAGES=1 to keep it.
#   pages/ index/ the words archive of the quran-svg-elements release (hafs-kfgqpc);
#                 needed by the identity gate.
#   pages-letters/ the letters archive of the same release: NNN.svg, index/ and
#                 schema/FORMAT.md. It unpacks to 614 MB. Only QVP_SYNC=letters fetches it.
#
# Environment:
#   QVP_DATA_TAG   data release tag (default data-v0.5.0)
#   QVP_ORNAMENTS_TAG  data release that dist/pages/ornaments.qvo comes from (default
#                  data-v0.7.0, the first with the ornaments of the mushafs). An ornament set
#                  does not depend on the page files, so its release can be newer than theirs.
#   QVP_SVG_TAG    quran-svg-elements release tag (default v2.0.0)
#   QVP_KEEP_PAGES keep dist/pages as it is, whatever built it
#   QVP_SYNC       what to fetch: `all` (default), `pages` or `letters`. `pages` fetches the
#                  data release and leaves pages/ and index/ alone. A job that reads dist/pages
#                  only, such as a wrapper build, has no use for the 440 MB the SVG bundle
#                  unpacks to. `letters` does what `all` does and then fetches pages-letters/.
#                  A tag before v2.0.0 has no letters archive.
# Output: the directories above; a line per artifact saying fetched, kept or skipped.
set -euo pipefail
cd "$(dirname "$0")/.."

sync="${QVP_SYNC:-all}"
case "$sync" in
  all|pages|letters) ;;
  *) echo "error: QVP_SYNC must be all, pages or letters, not '$sync'" >&2; exit 2 ;;
esac

svg_tag="${QVP_SVG_TAG:-v2.0.0}"
# Stop before a download when the letters archive cannot exist.
if [ "$sync" = letters ]; then
  case "$svg_tag" in
    v0|v0.*|v1|v1.*) echo "error: $svg_tag has no letters archive" >&2; exit 1 ;;
  esac
fi

tag="${QVP_DATA_TAG:-data-v0.5.0}"
asset="quran-engine-pages-hafs-kfgqpc.tar.gz"
base="https://github.com/quran-ws/quran-engine/releases/download/$tag"

if [ -n "${QVP_KEEP_PAGES:-}" ] && [ -f dist/pages/atlas.qva ]; then
  echo "skipped  dist/pages (QVP_KEEP_PAGES is set)"
elif [ -f dist/pages/atlas.qva ] && [ "$(find dist/pages -maxdepth 1 -name '*.qvp' 2>/dev/null | wc -l)" -ge 604 ] \
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
  echo "fetched  dist/pages ($tag, checksum verified)"
fi

# The ornament set, from its own release. It replaces the copy in the data tarball.
orn_tag="${QVP_ORNAMENTS_TAG:-data-v0.7.0}"
orn_url="https://github.com/quran-ws/quran-engine/releases/download/$orn_tag/ornaments.qvo"
if [ -n "${QVP_KEEP_PAGES:-}" ] && [ -f dist/pages/atlas.qva ]; then
  echo "skipped  dist/pages/ornaments.qvo (QVP_KEEP_PAGES is set)"
elif [ -f dist/pages/ornaments.qvo ] && [ "$(cat dist/pages/.ornaments-tag 2>/dev/null)" = "$orn_tag" ]; then
  echo "kept     dist/pages/ornaments.qvo ($orn_tag)"
else
  part="dist/pages/ornaments.qvo.part"
  curl --fail --location --retry 3 --silent --show-error "$orn_url" --output "$part" || {
    rm -f "$part"
    echo "error: could not fetch $orn_url" >&2
    exit 1
  }
  mv "$part" dist/pages/ornaments.qvo
  echo "$orn_tag" > dist/pages/.ornaments-tag
  echo "fetched  dist/pages/ornaments.qvo ($orn_tag)"
fi

if [ "$sync" = pages ]; then
  echo "skipped  pages/ and index/ (QVP_SYNC=pages)"
  exit 0
fi

# From v2.0.0 a release holds two archives, words and letters. The pages are built from
# the words archive. A release before v2.0.0 holds one archive, which has word pages.
case "$svg_tag" in
  v0|v0.*|v1|v1.*) svg_asset="quran-svg-elements-hafs-kfgqpc.tar.gz" ;;
  *)                svg_asset="quran-svg-elements-hafs-kfgqpc-words.tar.gz" ;;
esac
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

[ "$sync" = letters ] || exit 0

# The letters archive. Its pages/ goes to pages-letters/, its index/ to
# pages-letters/index/. A later step reads schema/FORMAT.md, so keep it too.
letters_asset="quran-svg-elements-hafs-kfgqpc-letters.tar.gz"
if [ -f pages-letters/001.svg ] && [ -d pages-letters/index/by-page ] \
   && [ "$(cat pages-letters/.tag 2>/dev/null)" = "$svg_tag" ]; then
  echo "kept     pages-letters/ ($svg_tag)"
else
  mkdir -p dist
  # The archive is large. Give the download up to one hour.
  curl --fail --location --retry 3 --max-time 3600 --silent --show-error \
    "$svg_base/$letters_asset" --output "dist/$letters_asset"
  curl --fail --location --retry 3 --silent --show-error \
    "$svg_base/$letters_asset.sha256" --output "dist/$letters_asset.sha256"
  (cd dist && shasum -a 256 -c "$letters_asset.sha256" >/dev/null)
  rm -rf pages-letters && mkdir -p pages-letters
  tar -xzf "dist/$letters_asset" -C pages-letters --strip-components=1 \
    --include='*/pages/*' --include='*/index/*' --include='*/schema/FORMAT.md' 2>/dev/null \
    || tar -xzf "dist/$letters_asset" -C pages-letters --strip-components=1 \
      --wildcards '*/pages/*' '*/index/*' '*/schema/FORMAT.md'
  [ -f pages-letters/pages/001.svg ] || {
    echo "error: the letters archive did not contain pages/001.svg" >&2; exit 1; }
  # Move the pages up one level, so that a page is pages-letters/NNN.svg.
  find pages-letters/pages -maxdepth 1 -name '*.svg' -exec mv {} pages-letters/ \;
  rmdir pages-letters/pages
  echo "$svg_tag" > pages-letters/.tag
  echo "fetched  pages-letters/ ($svg_tag, checksum verified)"
fi
