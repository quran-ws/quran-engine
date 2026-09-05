#!/usr/bin/env bash
# Copy the demo's page subset (001–021, 440–445, 582, 604: .qvp + .words.json, and atlas.qva)
# from the converter output into Demo/pages/, which the app bundles as a folder reference.
# Demo/pages/ is a build artefact (gitignored); the Swift package ships no data.
set -euo pipefail
cd "$(dirname "$0")"
SRC="${QVP_PAGES:-../../../dist/pages}"
if [ ! -f "$SRC/atlas.qva" ]; then
  echo "error: no page data at $SRC — run from the repo root:" >&2
  echo "  cargo run -p qvp-convert --release -- batch pages dist/pages" >&2
  exit 1
fi
mkdir -p pages
for n in $(seq 1 21) $(seq 440 445) 582 604; do
  p=$(printf '%03d' "$n")
  cp "$SRC/$p.qvp" "$SRC/$p.words.json" pages/
done
cp "$SRC/atlas.qva" pages/
echo "demo pages synced: $(ls pages | wc -l | tr -d ' ') files from $SRC"
