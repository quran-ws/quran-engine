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
#   scripts/publish-cdn.sh v0.1.0                 # stage from dist/pages, upload
#   scripts/publish-cdn.sh v0.1.0 --from-release  # fetch the release tarball instead
#   scripts/publish-cdn.sh v0.1.0 --stage-only    # build the tree, upload nothing
#
# Two shapes of the same data go up. The per-page objects are what a reader fetches — it
# wants page 42, not 92 MB. The bundle is for "download the whole mushaf": one request, and
# solid brotli across all 604 pages exploits the redundancy between them (26 MB against
# 41.8 MB for the same pages fetched individually). gzip cannot do this — its 32 KB window
# never sees two pages at once.
#
# The bundle is an opaque brotli file: no Content-Encoding, so every client receives the same
# 26 MB and decodes it itself. Serving it as `Content-Encoding: br` was tried and reverted —
# the edge caches one normalised body, so whichever client filled the cache first decided
# what everyone got: a gzip client filling it first left every browser downloading 89 MB.
# iOS decodes this with COMPRESSION_BROTLI (iOS 15+). Browsers have no brotli decoder in
# JavaScript, so a web app should fetch pages individually — 20 of them arrive in 36 ms.
#
# Objects are stored RAW. Compression happens at the edge: a Cloudflare Compression Rule on
# cdn.quran.ws compresses application/octet-stream, negotiating zstd → brotli → gzip per
# client, which beats one fixed encoding (~37 MB brotli vs ~62 MB gzip for the mushaf). It
# also fails safely — lose the rule and clients get correct, larger bytes — and what arrives
# is the byte stream the manifest's sha256 covers.
#
# Needs: curl (>= 7.75, for --aws-sigv4), brotli, tar, sha256sum/shasum, jq;
# gh for --from-release. Credentials: see scripts/cdn-put.sh.
set -euo pipefail
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

. scripts/cdn-put.sh

FAMILY=qvp
RELEASE="$VERSION"           # the tag, for `gh release download`
VERSION="$(cdn_version "$VERSION")"   # the folder, without the data- prefix
PREFIX="$FAMILY/$VERSION"
SRC=dist/pages
STAGE="dist/cdn/$VERSION"

# 1. Source the files. The release tarball is the canonical artefact; dist/pages is what a
#    local `batch` just produced. Either way we verify nothing beyond what the release signs.
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
[ -f "$SRC/atlas.qva" ] || { echo "no page data in $SRC — run batch, or pass --from-release" >&2; exit 1; }

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
for f in "$SRC"/*.qvp "$SRC"/*.words.json "$SRC"/atlas.qva "$SRC"/atlas.json "$SRC"/VERSION.json; do
  printf '%s\t%s\t%s\n' "$(basename "$f")" "$(wc -c < "$f" | tr -d ' ')" "$(sha256 "$f")" >> "$STAGE/.files.tsv"
done
for f in $extras; do
  printf '%s\t%s\t%s\n' "$f" "$(wc -c < "$SRC/$f" | tr -d ' ')" "$(sha256 "$SRC/$f")" >> "$STAGE/.files.tsv"
done

# 2b. The solid bundle, named for the edition alone: the host says the format and the prefix
#     says the version, so anything more just repeats the URL. Built once per release; brotli -q 11 over ~92 MB takes a few minutes.
BUNDLE="hafs-kfgqpc.tar.br"
echo "== building $BUNDLE (solid brotli, this takes a few minutes)"
# COPYFILE_DISABLE: macOS tar otherwise stores extended attributes as AppleDouble "._name"
# members. `tar tf` on macOS hides them, but Linux, iOS and every JS untar see them — the
# archive would extract to 2,424 files, half of them junk, and differ from a CI build.
  # --format ustar keeps it to the portable header, with no pax extensions to parse.
( cd "$SRC" && COPYFILE_DISABLE=1 tar --format ustar -cf - $(ls *.qvp *.words.json atlas.qva atlas.json VERSION.json $extras) ) \
  | brotli -q 11 -c > "$STAGE/$BUNDLE"
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
cut -f1 "$STAGE/.files.tsv" | xargs -P 16 -I{} bash -c 'cdn_put "$PREFIX/{}" "$SRC/{}"'
cdn_put "$PREFIX/$BUNDLE" "$STAGE/$BUNDLE"
cdn_put "$PREFIX/manifest.json" "$STAGE/manifest.json" "public, max-age=300"
cdn_latest "$FAMILY" "$VERSION"

echo "== published https://$CDN_HOST/$PREFIX/manifest.json"
