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

REPO="${QVP_DATA_REPO:-quran-ws/quran-engine}"
TAG="${QVP_DATA_TAG:-v0.1.0}"
ASSET=quran-engine-pages-hafs-kfgqpc.tar.gz
BASE="https://github.com/$REPO/releases/download/$TAG"
SRC="${QVP_PAGES:-$ROOT/dist/pages}"

# 604 pages + 604 sidecars + atlas.qva
want=1209
have=$( (ls pages 2>/dev/null || true) | wc -l | tr -d ' ')
if [ "$have" -ge "$want" ] && [ -f pages/atlas.qva ] && [ -z "${QVP_FORCE_SYNC:-}" ]; then
  echo "demo pages already synced ($have files)"; exit 0
fi

if [ ! -f "$SRC/atlas.qva" ]; then
  echo "no page data at $SRC — fetching the $TAG release bundle"
  mkdir -p "$ROOT/dist"
  if [ ! -f "$ROOT/dist/$ASSET" ]; then
    # The repository is private, so an anonymous download 404s: prefer the authenticated gh CLI
    # and fall back to a plain URL (which works once the release is public).
    if command -v gh >/dev/null 2>&1 && gh release download "$TAG" -R "$REPO" -p "$ASSET*" -D "$ROOT/dist" --clobber 2>/dev/null; then
      :
    elif ! curl -fL --progress-bar -o "$ROOT/dist/$ASSET" "$BASE/$ASSET"; then
      rm -f "$ROOT/dist/$ASSET"
      echo "error: could not download $ASSET from the $TAG release of $REPO." >&2
      echo "  The release is private: sign in with 'gh auth login' (needs read access), or put the" >&2
      echo "  unpacked page data somewhere and point QVP_PAGES at it." >&2
      exit 1
    fi
  fi
  [ -f "$ROOT/dist/$ASSET.sha256" ] || curl -fsL -o "$ROOT/dist/$ASSET.sha256" "$BASE/$ASSET.sha256" || true
  if [ -s "$ROOT/dist/$ASSET.sha256" ]; then
    (cd "$ROOT/dist" && shasum -a 256 -c "$ASSET.sha256" >/dev/null) || { echo "error: $ASSET failed its checksum" >&2; exit 1; }
  else
    echo "warning: no checksum file alongside $ASSET — skipping verification" >&2
  fi
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
