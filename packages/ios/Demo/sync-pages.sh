#!/usr/bin/env bash
# Fill Demo/pages/ with the complete mushaf (604 × .qvp + .words.json, plus atlas.qva), which the
# app bundles as a folder reference. Demo/pages/ is a build artefact (gitignored); the Swift
# package ships no data.
#
# Source, in order:
#   $QVP_PAGES     — a directory of converter output (dist/pages) to use as-is
#   dist/pages     — already unpacked at the repo root
#   the release    — downloaded and cached in dist/ (no Rust, no source SVGs needed)
# Set QVP_DATA_TAG to pin a different release, QVP_FORCE_SYNC=1 to re-copy an up-to-date bundle.
set -euo pipefail
cd "$(dirname "$0")"
ROOT=$(cd ../../.. && pwd)

TAG="${QVP_DATA_TAG:-v0.1.0}"
ASSET=quran-engine-pages-hafs-kfgqpc.tar.gz
BASE="https://github.com/quranpedia/quran-engine/releases/download/$TAG"
SRC="${QVP_PAGES:-$ROOT/dist/pages}"

# 604 pages + 604 sidecars + atlas.qva
want=1209
have=$(ls pages 2>/dev/null | wc -l | tr -d ' ')
if [ "$have" -ge "$want" ] && [ -f pages/atlas.qva ] && [ -z "${QVP_FORCE_SYNC:-}" ]; then
  echo "demo pages already synced ($have files)"; exit 0
fi

if [ ! -f "$SRC/atlas.qva" ]; then
  echo "no page data at $SRC — fetching the $TAG release bundle"
  mkdir -p "$ROOT/dist"
  [ -f "$ROOT/dist/$ASSET" ] || curl -fL --progress-bar -o "$ROOT/dist/$ASSET" "$BASE/$ASSET"
  curl -fsL -o "$ROOT/dist/$ASSET.sha256" "$BASE/$ASSET.sha256"
  (cd "$ROOT/dist" && shasum -a 256 -c "$ASSET.sha256" >/dev/null) || { echo "error: $ASSET failed its checksum" >&2; exit 1; }
  SRC="$ROOT/dist/pages"
  rm -rf "$SRC"; mkdir -p "$SRC"
  tar xzf "$ROOT/dist/$ASSET" -C "$SRC" --strip-components=1
fi

rm -rf pages; mkdir -p pages
for n in $(seq 1 604); do
  p=$(printf '%03d' "$n")
  cp "$SRC/$p.qvp" pages/
  [ -f "$SRC/$p.words.json" ] && cp "$SRC/$p.words.json" pages/
done
cp "$SRC/atlas.qva" pages/
echo "demo pages synced: $(ls pages | wc -l | tr -d ' ') files from $SRC"
