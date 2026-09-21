#!/usr/bin/env bash
# Package a data release from dist/pages: VERSION.json, the upstream notice, the tarball
# and its checksum. Run after `qvp-convert batch` and the full identity gate.
#
# Usage: scripts/package-data.sh X.Y.Z
# Inputs: dist/pages/ (604 pages and sidecars, atlas.qva, generated surah-name assets),
#         pages/.tag (the quran-svg-elements tag).
# Output: dist/quran-engine-pages-hafs-kfgqpc.tar.gz and .sha256, ready for
#         `gh release create data-vX.Y.Z`.
set -euo pipefail
cd "$(dirname "$0")/.."
v="${1:?usage: scripts/package-data.sh X.Y.Z}"
name="quran-engine-pages-hafs-kfgqpc"
[ -f dist/pages/atlas.qva ] || { echo "error: dist/pages is missing; run the batch converter first" >&2; exit 1; }
count="$(find dist/pages -maxdepth 1 -type f -name '*.qvp' | wc -l | tr -d ' ')"
[ "$count" = 604 ] || { echo "error: expected 604 pages in dist/pages, found $count" >&2; exit 1; }
sidecars="$(find dist/pages -maxdepth 1 -type f -name '*.words.json' | wc -l | tr -d ' ')"
[ "$sidecars" = 604 ] || { echo "error: expected 604 word sidecars in dist/pages, found $sidecars" >&2; exit 1; }
[ -d dist/pages/surah-names/qvp ] && [ -d dist/pages/surah-names/svg ] || {
  echo "error: dist/pages/surah-names is incomplete; run the batch converter first" >&2
  exit 1
}
surah_names="$(find dist/pages/surah-names/qvp -maxdepth 1 -type f -name '[0-9][0-9][0-9].qvp' | wc -l | tr -d ' ')"
surah_svgs="$(find dist/pages/surah-names/svg -maxdepth 1 -type f -name '[0-9][0-9][0-9].svg' | wc -l | tr -d ' ')"
[ "$surah_names" = 114 ] || { echo "error: expected 114 QVP surah names, found $surah_names" >&2; exit 1; }
[ "$surah_svgs" = 114 ] || { echo "error: expected 114 SVG surah names, found $surah_svgs" >&2; exit 1; }
for number in {001..114}; do
  [ -f "dist/pages/surah-names/qvp/$number.qvp" ] || { echo "error: missing surah-names/qvp/$number.qvp" >&2; exit 1; }
  [ -f "dist/pages/surah-names/svg/$number.svg" ] || { echo "error: missing surah-names/svg/$number.svg" >&2; exit 1; }
done
for asset in qvp/all.qvp svg/all.svg surah-names.woff2 surah-names.css map.json; do
  [ -f "dist/pages/surah-names/$asset" ] || { echo "error: missing surah-names/$asset" >&2; exit 1; }
done

stage="$(mktemp -d)/$name"; trap 'rm -rf "$(dirname "$stage")"' EXIT
mkdir -p "$stage/surah-names"
cp dist/pages/*.qvp dist/pages/*.words.json dist/pages/atlas.qva dist/pages/atlas.json "$stage/"
cp -R dist/pages/surah-names/. "$stage/surah-names/"
python3 - "$stage" "$v" "$(cat pages/.tag 2>/dev/null || echo unknown)" "$(git rev-parse HEAD)" <<'PY'
import hashlib, json, pathlib, sys, datetime
stage, v, svg_tag, commit = sys.argv[1:5]
root = pathlib.Path(stage)
files = sorted(p for p in root.rglob("*") if p.is_file())
version = {
    "name": "quran-engine-pages-hafs-kfgqpc", "version": v,
    "source": "quran-ws/quran-svg-elements", "source_version": svg_tag,
    "engine_commit": commit, "format_version": 1,
    "generated_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "pages": sum(1 for p in files if p.parent == root and p.suffix == ".qvp"),
    "surah_names": sum(1 for p in files if p.parent.name == "qvp" and p.stem.isdigit() and p.suffix == ".qvp"),
    "surah_name_formats": ["qvp", "svg", "woff2"],
    "surah_name_assets": {
        "individual": {kind: sum(1 for p in files if p.parent.name == kind and p.stem.isdigit()) for kind in ("qvp", "svg")},
        "combined": ["surah-names/qvp/all.qvp", "surah-names/svg/all.svg"],
        "font": "surah-names/surah-names.woff2",
    },
    "files": {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
}
(root / "VERSION.json").write_text(json.dumps(version, indent=1) + "\n")
PY
awk '/^## Third-party/,0' LICENSE > "$stage/NOTICE.txt" 2>/dev/null || cp LICENSE "$stage/NOTICE.txt"
mkdir -p dist
tar -czf "dist/$name.tar.gz" -C "$(dirname "$stage")" "$name"
(cd dist && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
echo "packaged dist/$name.tar.gz ($count pages, $sidecars word sidecars, $surah_names surah names, data version $v); next: gh release create data-v$v dist/$name.tar.gz dist/$name.tar.gz.sha256"
