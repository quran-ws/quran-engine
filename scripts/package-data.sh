#!/usr/bin/env bash
# Package a data release from dist/pages: VERSION.json, the upstream notice, the tarball
# and its checksum. Run after `qvp-convert batch` and the full identity gate.
#
# Usage: scripts/package-data.sh X.Y.Z
# Inputs: dist/pages/ (604 pages, atlas.qva), pages/.tag (the quran-svg-elements tag).
# Output: dist/quran-engine-pages-hafs-kfgqpc.tar.gz and .sha256, ready for
#         `gh release create data-vX.Y.Z`.
set -euo pipefail
cd "$(dirname "$0")/.."
v="${1:?usage: scripts/package-data.sh X.Y.Z}"
name="quran-engine-pages-hafs-kfgqpc"
[ -f dist/pages/atlas.qva ] || { echo "error: dist/pages is missing; run the batch converter first" >&2; exit 1; }
count="$(ls dist/pages/*.qvp | wc -l | tr -d ' ')"
[ "$count" = 604 ] || { echo "error: expected 604 pages in dist/pages, found $count" >&2; exit 1; }

stage="$(mktemp -d)/$name"; trap 'rm -rf "$(dirname "$stage")"' EXIT
mkdir -p "$stage"; cp dist/pages/* "$stage/"
python3 - "$stage" "$v" "$(cat pages/.tag 2>/dev/null || echo unknown)" "$(git rev-parse HEAD)" <<'PY'
import hashlib, json, pathlib, sys, datetime
stage, v, svg_tag, commit = sys.argv[1:5]
root = pathlib.Path(stage)
files = sorted(p for p in root.iterdir() if p.is_file())
version = {
    "name": "quran-engine-pages-hafs-kfgqpc", "version": v,
    "source": "quran-ws/quran-svg-elements", "source_version": svg_tag,
    "engine_commit": commit, "format_version": 1,
    "generated_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "pages": sum(1 for p in files if p.suffix == ".qvp"),
    "files": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
}
(root / "VERSION.json").write_text(json.dumps(version, indent=1) + "\n")
PY
awk '/^## Third-party/,0' LICENSE > "$stage/NOTICE.txt" 2>/dev/null || cp LICENSE "$stage/NOTICE.txt"
mkdir -p dist
tar -czf "dist/$name.tar.gz" -C "$(dirname "$stage")" "$name"
(cd dist && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
echo "packaged dist/$name.tar.gz ($count pages, data version $v); next: gh release create data-v$v dist/$name.tar.gz dist/$name.tar.gz.sha256"
