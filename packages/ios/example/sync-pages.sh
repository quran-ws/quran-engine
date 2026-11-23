#!/usr/bin/env bash
# Fill example/pages/ with the complete mushaf (604 × .qvp + .words.json, plus atlas.qva, plus
# ornaments.qvo when the source has one), which the app bundles as a folder reference. example/pages/ is a build artefact (gitignored); the Swift
# package ships no data.
# The letter layers (NNN.qvl) come from $QVP_LETTERS, else dist/letters, when there are any.
# They are optional: without them the letter controls of the app are disabled. A layer
# attaches only to the page file it was built with (docs/EXAMPLE-APP.md).
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
TAG="${QVP_DATA_TAG:-data-v0.5.0}"
ASSET=quran-engine-pages-hafs-kfgqpc.tar.gz
BASE="https://github.com/$REPO/releases/download/$TAG"
SRC="${QVP_PAGES:-$ROOT/dist/pages}"
LETTERS="${QVP_LETTERS:-$ROOT/dist/letters}"

# The ornament set for the dress panel. It is optional: the demo works without it.
# dist/pages has it from scripts/sync-test-data.sh. The data tarball of an older release has
# the font markers only, so a set that this script downloads comes from its own release.
ORNAMENTS_TAG="${QVP_ORNAMENTS_TAG:-data-v0.7.0}"
copy_ornaments() {
  if [ -f "$1/.ornaments-tag" ] || [ -n "${QVP_PAGES:-}" ]; then
    if [ -f "$1/ornaments.qvo" ]; then cp "$1/ornaments.qvo" pages/; fi
  elif curl -fsL "https://github.com/$REPO/releases/download/$ORNAMENTS_TAG/ornaments.qvo" -o pages/ornaments.qvo.part; then
    mv pages/ornaments.qvo.part pages/ornaments.qvo
  else
    rm -f pages/ornaments.qvo.part
    if [ -f "$1/ornaments.qvo" ]; then cp "$1/ornaments.qvo" pages/; fi
  fi
}

# The letter layers. They are optional too. Copy them again on each run, so the bundle has
# the layers that $LETTERS has now and no others.
copy_letters() {
  rm -f pages/*.qvl
  if [ -d "$LETTERS" ]; then
    find "$LETTERS" -maxdepth 1 -name '[0-9][0-9][0-9].qvl' -exec cp {} pages/ \;
  fi
}

letter_count() { find pages -maxdepth 1 -name '*.qvl' | wc -l | tr -d ' '; }

# 604 pages + 604 sidecars + atlas.qva. The letter layers do not count.
want=1209
have=$( (ls pages 2>/dev/null || true) | grep -vc '\.qvl$' || true)
if [ "$have" -ge "$want" ] && [ -f pages/atlas.qva ] && [ -z "${QVP_FORCE_SYNC:-}" ] \
   && [ "$(cat pages/.tag 2>/dev/null)" = "$TAG" ]; then
  copy_ornaments "$SRC"
  copy_letters
  echo "demo pages already synced ($have files, $TAG, $(letter_count) letter layers)"; exit 0
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
  echo "$TAG" > "$SRC/.tag"
fi

rm -rf pages; mkdir -p pages
for n in $(seq 1 604); do
  p=$(printf '%03d' "$n")
  cp "$SRC/$p.qvp" pages/
  [ -f "$SRC/$p.words.json" ] && cp "$SRC/$p.words.json" pages/
done
cp "$SRC/atlas.qva" pages/
copy_ornaments "$SRC"
copy_letters
# Record where the bundle came from: the tag its source directory records, or "local"
# for page data built on this machine. A mismatch makes the next run re-sync.
cat "$SRC/.tag" 2>/dev/null > pages/.tag || echo local > pages/.tag
echo "demo pages synced: $(find pages -mindepth 1 -maxdepth 1 ! -name '.*' | wc -l | tr -d ' ') files from $SRC, $(letter_count) letter layers"
