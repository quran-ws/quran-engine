#!/usr/bin/env bash
# Package a data release from a converted edition: VERSION.json, the upstream notice, the
# tarball and its checksum. Run after `qvp-convert batch` and the full identity gate.
#
# Usage: scripts/package-data.sh X.Y.Z [edition]
#   edition hafs-kfgqpc (default) reads dist/pages and pages/.tag, and is tagged data-vX.Y.Z.
#   edition hafs-qcf-v1-1405h reads dist/hafs-qcf-v1-1405h and QVP_SOURCE_VERSION, and is
#   tagged data-hafs-qcf-v1-1405h-vX.Y.Z.
# Inputs: the edition's directory (604 pages and sidecars, atlas.qva, generated surah-name
#         assets) and the quran-svg-elements tag it was converted from.
#         Optional: ornaments.qvo, built with --redistributable, and QVP_ASSETS_COMMIT. The
#         script refuses a set with any ornament whose licence is not confirmed.
# Output: dist/quran-engine-pages-<edition>.tar.gz and .sha256, ready for
#         `gh release create <tag>`.
set -euo pipefail
cd "$(dirname "$0")/.."
v="${1:?usage: scripts/package-data.sh X.Y.Z [edition]}"
edition="${2:-hafs-kfgqpc}"
case "$edition" in
  hafs-kfgqpc) src=dist/pages; tag="data-v$v"; source_version="$(cat pages/.tag 2>/dev/null || echo unknown)" ;;
  hafs-qcf-v1-1405h) src="dist/$edition"; tag="data-$edition-v$v"; source_version="${QVP_SOURCE_VERSION:?set QVP_SOURCE_VERSION to the quran-svg-elements tag}" ;;
  *) echo "error: unknown edition $edition" >&2; exit 2 ;;
esac
name="quran-engine-pages-$edition"
[ -f "$src/atlas.qva" ] || { echo "error: $src is missing; run the batch converter first" >&2; exit 1; }
count="$(find "$src" -maxdepth 1 -type f -name '*.qvp' | wc -l | tr -d ' ')"
[ "$count" = 604 ] || { echo "error: expected 604 pages in $src, found $count" >&2; exit 1; }
sidecars="$(find "$src" -maxdepth 1 -type f -name '*.words.json' | wc -l | tr -d ' ')"
[ "$sidecars" = 604 ] || { echo "error: expected 604 word sidecars in $src, found $sidecars" >&2; exit 1; }
[ -d "$src/surah-names/qvp" ] && [ -d "$src/surah-names/svg" ] || {
  echo "error: $src/surah-names is incomplete; run the batch converter first" >&2
  exit 1
}
surah_names="$(find "$src/surah-names/qvp" -maxdepth 1 -type f -name '[0-9][0-9][0-9].qvp' | wc -l | tr -d ' ')"
surah_svgs="$(find "$src/surah-names/svg" -maxdepth 1 -type f -name '[0-9][0-9][0-9].svg' | wc -l | tr -d ' ')"
[ "$surah_names" = 114 ] || { echo "error: expected 114 QVP surah names, found $surah_names" >&2; exit 1; }
[ "$surah_svgs" = 114 ] || { echo "error: expected 114 SVG surah names, found $surah_svgs" >&2; exit 1; }
for number in {001..114}; do
  [ -f "$src/surah-names/qvp/$number.qvp" ] || { echo "error: missing surah-names/qvp/$number.qvp" >&2; exit 1; }
  [ -f "$src/surah-names/svg/$number.svg" ] || { echo "error: missing surah-names/svg/$number.svg" >&2; exit 1; }
done
for asset in qvp/all.qvp svg/all.svg surah-names.woff2 surah-names.css map.json; do
  [ -f "$src/surah-names/$asset" ] || { echo "error: missing surah-names/$asset" >&2; exit 1; }
done

# The ornament set is optional: a release without one is still a page release. When it is
# present it must be the redistributable build, and its source commit is recorded.
ornaments_commit="${QVP_ASSETS_COMMIT:-}"
if [ -f "$src/ornaments.qvo" ]; then
  [ -n "$ornaments_commit" ] || { echo "error: $src/ornaments.qvo is present; set QVP_ASSETS_COMMIT to the quran-assets commit it was built from" >&2; exit 1; }
  cargo build -q -p qvp-convert --release
  info="$(target/release/qvp-convert ornaments-info "$src/ornaments.qvo")" || {
    echo "error: $src/ornaments.qvo does not decode" >&2
    exit 1
  }
  last="$(printf '%s\n' "$info" | tail -n 1)"
  if [[ ! "$last" =~ ^redistributable:\ ([0-9]+)\ of\ ([0-9]+)$ ]]; then
    echo "error: unexpected ornaments-info output: $last" >&2
    exit 1
  fi
  if [ "${BASH_REMATCH[1]}" != "${BASH_REMATCH[2]}" ]; then
    echo "error: $src/ornaments.qvo has ${BASH_REMATCH[1]} redistributable ornaments of ${BASH_REMATCH[2]}; build it with --redistributable" >&2
    exit 1
  fi
fi

stage="$(mktemp -d)/$name"; trap 'rm -rf "$(dirname "$stage")"' EXIT
mkdir -p "$stage/surah-names"
if [ -f "$src/ornaments.qvo" ]; then
  cp "$src/ornaments.qvo" "$stage/"
fi
cp "$src"/*.qvp "$src"/*.words.json "$src/atlas.qva" "$src/atlas.json" "$stage/"
cp -R "$src/surah-names/." "$stage/surah-names/"
python3 - "$stage" "$v" "$source_version" "$(git rev-parse HEAD)" "$ornaments_commit" "$name" "$edition" <<'PY'
import hashlib, json, pathlib, sys, datetime
stage, v, svg_tag, commit, assets_commit, name, edition = sys.argv[1:8]
root = pathlib.Path(stage)
files = sorted(p for p in root.rglob("*") if p.is_file())
version = {
    "name": name, "edition": edition, "version": v,
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
    "ornaments": ({"source": "quran-ws/quran-assets", "source_commit": assets_commit, "redistributable_only": True}
                  if (root / "ornaments.qvo").exists() else None),
    "files": {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
}
(root / "VERSION.json").write_text(json.dumps(version, indent=1) + "\n")
PY
awk '/^## Third-party/,0' LICENSE > "$stage/NOTICE.txt" 2>/dev/null || cp LICENSE "$stage/NOTICE.txt"
mkdir -p dist
tar -czf "dist/$name.tar.gz" -C "$(dirname "$stage")" "$name"
(cd dist && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256")
extra=""; [ -f "$src/ornaments.qvo" ] && extra=" $src/ornaments.qvo"
echo "packaged dist/$name.tar.gz ($count pages, $sidecars word sidecars, $surah_names surah names, data version $v); next: gh release create $tag dist/$name.tar.gz dist/$name.tar.gz.sha256$extra"
