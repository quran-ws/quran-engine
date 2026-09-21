#!/usr/bin/env bash
# Publish one version of the page data to cdn.quran.ws/qvp/ (Cloudflare R2).
#
# The host carries every artefact of the stack, so the page data lives under a folder of its
# own; scripts/cdn-put.sh holds the layout and the upload itself.
#
# Versions are immutable: a path is written once and served forever, so the objects go up
# with `Cache-Control: immutable` and nothing ever overwrites them. A new build means a new
# version, never a rewrite of an old one.
#
#   scripts/publish-cdn.sh v0.1.0                 # publish an extracted package in dist/pages
#   scripts/publish-cdn.sh v0.1.0 --from-release  # fetch the release tarball instead
#   scripts/publish-cdn.sh v0.1.0 --from-release --stage-only  # stage, upload nothing
#
# Two shapes of the same data go up. The individual objects are what a reader fetches — it
# wants page 42 or one surah title, not the whole data set. The bundle is for "download the
# whole mushaf": one request, with solid brotli exploiting repetition across archive members.
#
# The bundle is an opaque brotli file: no Content-Encoding, so every client receives the same
# bytes and decodes them itself. Serving it as `Content-Encoding: br` was tried and reverted —
# the edge caches one normalised body, so whichever client filled the cache first decided
# what everyone got.
# iOS decodes this with COMPRESSION_BROTLI (iOS 15+). Browsers have no brotli decoder in
# JavaScript, so a web app should fetch pages individually — 20 of them arrive in 36 ms.
#
# Objects are stored RAW. Compression happens at the edge: a Cloudflare Compression Rule on
# cdn.quran.ws compresses application/octet-stream, negotiating zstd → brotli → gzip per
# client. It also fails safely — lose the rule and clients get correct, larger bytes — and
# what arrives
# is the byte stream the manifest's sha256 covers.
#
# Needs: curl (>= 7.75, for --aws-sigv4), brotli, tar, sha256sum/shasum, jq;
# gh for --from-release. Credentials: see scripts/cdn-put.sh.
set -euo pipefail
shopt -s nullglob
cd "$(dirname "$0")/.."

VERSION="${1:?usage: publish-cdn.sh <version> [--from-release] [--stage-only]}"; shift
FROM_RELEASE=0; STAGE_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --from-release) FROM_RELEASE=1 ;;
    --stage-only)   STAGE_ONLY=1 ;;
    *) echo "unknown flag: $arg" >&2; exit 2 ;;
  esac
done

# shellcheck source=scripts/cdn-put.sh
. scripts/cdn-put.sh

FAMILY=qvp
RELEASE="$VERSION"           # the tag, for `gh release download`
VERSION="$(cdn_version "$VERSION")"   # the folder, without the data- prefix
PREFIX="$FAMILY/$VERSION"
SRC=dist/pages
STAGE="dist/cdn/$VERSION"

# 1. Source the files. The release tarball is canonical; a local dist/pages must be an
#    extracted package, including VERSION.json. Either way we publish exactly the signed bytes.
if [ "$FROM_RELEASE" = 1 ]; then
  TAR=dist/quran-engine-pages-hafs-kfgqpc.tar.gz
  mkdir -p dist && rm -rf "$SRC"
  gh release download "$RELEASE" --repo quran-ws/quran-engine --clobber -D dist \
    -p 'quran-engine-pages-hafs-kfgqpc.tar.gz*'
  echo "== verifying $TAR"
  want=$(cut -d' ' -f1 < "$TAR.sha256"); got=$(sha256 "$TAR")
  [ "$want" = "$got" ] || { echo "sha256 mismatch: want $want got $got" >&2; exit 1; }
  mkdir -p "$SRC" && tar -xzf "$TAR" -C "$SRC" --strip-components=1
fi
if [ ! -f "$SRC/atlas.qva" ] || [ ! -f "$SRC/atlas.json" ]; then
  echo "no atlas data in $SRC — extract a packaged release, or pass --from-release" >&2
  exit 1
fi
[ -f "$SRC/VERSION.json" ] || {
  echo "no packaged VERSION.json in $SRC — run scripts/package-data.sh and publish the extracted release, or pass --from-release" >&2
  exit 1
}
pages=( "$SRC"/*.qvp )
sidecars=( "$SRC"/*.words.json )
[ "${#pages[@]}" = 604 ] || { echo "expected 604 QVP pages in $SRC, found ${#pages[@]}" >&2; exit 1; }
[ "${#sidecars[@]}" = 604 ] || { echo "expected 604 word sidecars in $SRC, found ${#sidecars[@]}" >&2; exit 1; }
surah_name_qvps=( "$SRC"/surah-names/qvp/[0-9][0-9][0-9].qvp )
surah_name_svgs=( "$SRC"/surah-names/svg/[0-9][0-9][0-9].svg )
[ "${#surah_name_qvps[@]}" = 114 ] || {
  echo "expected 114 QVP surah names, found ${#surah_name_qvps[@]}" >&2
  exit 1
}
[ "${#surah_name_svgs[@]}" = 114 ] || {
  echo "expected 114 SVG surah names, found ${#surah_name_svgs[@]}" >&2
  exit 1
}
for number in {001..114}; do
  [ -f "$SRC/surah-names/qvp/$number.qvp" ] || { echo "missing surah-names/qvp/$number.qvp" >&2; exit 1; }
  [ -f "$SRC/surah-names/svg/$number.svg" ] || { echo "missing surah-names/svg/$number.svg" >&2; exit 1; }
done
surah_name_assets=( "${surah_name_qvps[@]}" "$SRC"/surah-names/qvp/all.qvp "${surah_name_svgs[@]}" "$SRC"/surah-names/svg/all.svg "$SRC"/surah-names/surah-names.woff2 "$SRC"/surah-names/surah-names.css "$SRC"/surah-names/map.json )
for asset in "${surah_name_assets[@]}"; do
  [ -f "$asset" ] || { echo "missing $asset" >&2; exit 1; }
done

# 2. Stage: the manifest only. Objects upload straight from the source tree — nothing is
#    rewritten, so what we publish is byte-for-byte what the release signed.
echo "== staging $PREFIX"
rm -rf "$STAGE" && mkdir -p "$STAGE"
: > "$STAGE/.files.tsv"
# The text files a bundle carries have changed between data releases, so publish the ones
# this bundle has. The page data itself is always present and is checked above.
extras=""
for f in README.md NOTICE.txt; do
  if [ -f "$SRC/$f" ]; then extras="$extras $f"; fi
done
for f in "${pages[@]}" "${sidecars[@]}" "$SRC"/atlas.qva "$SRC"/atlas.json "$SRC"/VERSION.json "${surah_name_assets[@]}"; do
  name="${f#"$SRC/"}"
  printf '%s\t%s\t%s\n' "$name" "$(wc -c < "$f" | tr -d ' ')" "$(sha256 "$f")" >> "$STAGE/.files.tsv"
done
for f in $extras; do
  printf '%s\t%s\t%s\n' "$f" "$(wc -c < "$SRC/$f" | tr -d ' ')" "$(sha256 "$SRC/$f")" >> "$STAGE/.files.tsv"
done

# 2b. The solid bundle, named for the edition alone: the host says the format and the prefix
#     says the version, so anything more just repeats the URL. Built once per release; quality
#     11 over the complete data tree takes a few minutes.
BUNDLE="hafs-kfgqpc.tar.br"
echo "== building $BUNDLE (solid brotli, this takes a few minutes)"
# COPYFILE_DISABLE: macOS tar otherwise stores extended attributes as AppleDouble "._name"
# members. `tar tf` on macOS hides them, but Linux, iOS and every JS untar see them — the
# archive would contain one junk twin for every real file and differ from a CI build.
# --format ustar keeps it to the portable header, with no pax extensions to parse.
(
  cd "$SRC"
  files=( *.qvp *.words.json atlas.qva atlas.json VERSION.json surah-names/qvp/*.qvp surah-names/svg/*.svg surah-names/surah-names.woff2 surah-names/surah-names.css surah-names/map.json )
  for f in $extras; do files+=("$f"); done
  COPYFILE_DISABLE=1 tar --format ustar -cf - "${files[@]}"
) | brotli -q 11 -c > "$STAGE/$BUNDLE"
bundle_bytes=$(wc -c < "$STAGE/$BUNDLE" | tr -d ' ')
bundle_sha=$(sha256 "$STAGE/$BUNDLE")
# also record the archive inside, so a client can check what it decoded
brotli -dc "$STAGE/$BUNDLE" > "$STAGE/.tar.tmp"
tar_bytes=$(wc -c < "$STAGE/.tar.tmp" | tr -d ' ')
tar_sha=$(sha256 "$STAGE/.tar.tmp")
rm -f "$STAGE/.tar.tmp"
echo "   $(echo "scale=1; $bundle_bytes/1048576" | bc) MB"

# 3. A manifest so a service worker can prefetch a range of pages and verify what it got,
#    without 604 HEAD requests. Carries the data release's own VERSION.json verbatim.
jq -n --arg version "$VERSION" \
      --arg base "https://$CDN_HOST/$PREFIX/" \
      --arg bundle "$BUNDLE" --arg bbytes "$bundle_bytes" --arg bsha "$bundle_sha" \
      --arg tbytes "$tar_bytes" --arg tsha "$tar_sha" \
      --slurpfile release "$SRC/VERSION.json" \
      --rawfile tsv "$STAGE/.files.tsv" '
  {version: $version, base: $base, encoding: "identity", release: $release[0],
   bundle: {name: $bundle, bytes: ($bbytes|tonumber), sha256: $bsha,
            compression: "brotli — the client decodes it; no Content-Encoding is used",
            contains: "ustar archive of every file below",
            decoded: {bytes: ($tbytes|tonumber), sha256: $tsha}},
   files: ($tsv | rtrimstr("\n") | split("\n") | map(split("\t") |
     {name: .[0], bytes: (.[1]|tonumber), sha256: .[2]}))}
' > "$STAGE/manifest.json"
echo "   $(wc -l < "$STAGE/.files.tsv" | tr -d ' ') objects + manifest, $(du -sh "$SRC" | cut -f1) raw"

[ "$STAGE_ONLY" = 1 ] && { echo "== staged only: $STAGE"; exit 0; }

# 4. Upload. One object per file, straight from the source tree, never overwriting a version
#    that already exists. curl signs the S3 requests itself, so there is no SDK to install.
cdn_init
cdn_guard "$PREFIX" || exit 1

echo "== uploading to r2://$BUCKET/$PREFIX"
export PREFIX SRC
# The child shell expands the exported function and environment.
# shellcheck disable=SC2016
cut -f1 "$STAGE/.files.tsv" | xargs -P 16 -I{} bash -c 'cdn_put "$PREFIX/{}" "$SRC/{}"'
cdn_put "$PREFIX/$BUNDLE" "$STAGE/$BUNDLE"
cdn_put "$PREFIX/manifest.json" "$STAGE/manifest.json" "public, max-age=300"
cdn_latest "$FAMILY" "$VERSION"

echo "== published https://$CDN_HOST/$PREFIX/manifest.json"
