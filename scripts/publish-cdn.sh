#!/usr/bin/env bash
# Publish one immutable QVP data release to cdn.quran.ws (Cloudflare R2).
#
# The release tag selects the edition family. The default V4 line remains under `qvp/`;
# QCF V1 1405H has its own tag and folder, so equal version numbers never collide:
#
#   data-v0.4.0                         -> qvp/v0.4.0/
#   data-hafs-qcf-v1-1405h-v0.1.0      -> qvp/hafs-qcf-v1-1405h/v0.1.0/
#
# Usage:
#   scripts/publish-cdn.sh <tag> --from-release
#   scripts/publish-cdn.sh <tag> --from-release --stage-only
#   scripts/publish-cdn.sh <tag> --source-dir <extracted-package> --stage-only
#
# Individual files are stored raw. Edge compression negotiates the transfer encoding. The
# whole-edition bundle is an opaque brotli file with no Content-Encoding; clients decode it.
# Versions are never overwritten. `latest.json` moves independently within each edition family.
#
# Needs: Python 3.12+, curl >= 7.75, brotli, jq, and gh for --from-release.
# Credentials for live publishing are documented in scripts/cdn-put.sh.
set -euo pipefail
cd "$(dirname "$0")/.."

usage='usage: publish-cdn.sh <release-tag> [--from-release] [--stage-only] [--source-dir PATH]'
RELEASE="${1:?$usage}"
shift
FROM_RELEASE=0
STAGE_ONLY=0
SOURCE_OVERRIDE=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --from-release)
      FROM_RELEASE=1
      ;;
    --stage-only)
      STAGE_ONLY=1
      ;;
    --source-dir)
      shift
      [ "$#" -gt 0 ] || { echo "$usage" >&2; exit 2; }
      SOURCE_OVERRIDE="$1"
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
  shift
done
[ "$FROM_RELEASE" = 0 ] || [ -z "$SOURCE_OVERRIDE" ] || {
  echo "--from-release and --source-dir are mutually exclusive" >&2
  exit 2
}

# shellcheck source=scripts/cdn-put.sh
. scripts/cdn-put.sh

PROFILE="$(python scripts/qvp-data-release.py profile "$RELEASE")"
VERSION="$(jq -er .version <<< "$PROFILE")"
FAMILY="$(jq -er .family <<< "$PROFILE")"
ARCHIVE="$(jq -er .archive <<< "$PROFILE")"
BUNDLE="$(jq -er .bundle <<< "$PROFILE")"
SRC="${SOURCE_OVERRIDE:-$(jq -er .source_dir <<< "$PROFILE")}"
STAGE="$(jq -er .stage_dir <<< "$PROFILE")"
PREFIX="$FAMILY/$VERSION"
QUALITY="${QVP_BROTLI_QUALITY:-11}"
case "$QUALITY" in
  0|1|2|3|4|5|6|7|8|9|10|11) ;;
  *) echo "QVP_BROTLI_QUALITY must be an integer from 0 through 11" >&2; exit 2 ;;
esac

# The GitHub archive is canonical. Its checksum is verified before a path-safe extraction.
if [ "$FROM_RELEASE" = 1 ]; then
  TAR="dist/$ARCHIVE"
  mkdir -p dist
  gh release download "$RELEASE" --repo quran-ws/quran-engine --clobber -D dist \
    -p "$ARCHIVE" -p "$ARCHIVE.sha256"
  echo "== verifying $TAR"
  want="$(awk 'NF {print $1; exit}' "$TAR.sha256")"
  got="$(sha256 "$TAR")"
  [ "$want" = "$got" ] || {
    echo "sha256 mismatch: want $want got $got" >&2
    exit 1
  }
  rm -rf "$SRC" "$SRC.tmp"
  python scripts/qvp-data-release.py extract "$RELEASE" "$TAR" "$SRC" >/dev/null
fi

# The helper owns identity, complete inventory, path safety and every digest in VERSION.json.
echo "== staging $PREFIX from $SRC"
rm -rf "$STAGE"
mkdir -p "$STAGE"
python scripts/qvp-data-release.py inventory \
  "$RELEASE" "$SRC" "$STAGE/.files.tsv" >/dev/null

# Build the canonical inventory order with normalized ustar metadata. Extraction time,
# host uid/gid and file mode must not change an immutable bundle's identity.
echo "== building $BUNDLE (solid brotli quality $QUALITY)"
python scripts/qvp-data-release.py bundle \
  "$RELEASE" "$SRC" "$STAGE/.files.tsv" "$STAGE/.tar.tmp" >/dev/null
brotli -q "$QUALITY" -c "$STAGE/.tar.tmp" > "$STAGE/$BUNDLE.tmp"
mv "$STAGE/$BUNDLE.tmp" "$STAGE/$BUNDLE"
bundle_bytes="$(wc -c < "$STAGE/$BUNDLE" | tr -d ' ')"
bundle_sha="$(sha256 "$STAGE/$BUNDLE")"
tar_bytes="$(wc -c < "$STAGE/.tar.tmp" | tr -d ' ')"
tar_sha="$(sha256 "$STAGE/.tar.tmp")"
rm -f "$STAGE/.tar.tmp"
printf '   %.1f MB\n' "$(awk -v bytes="$bundle_bytes" 'BEGIN {print bytes / 1048576}')"

# The manifest carries VERSION.json verbatim and the exact objects a client may install.
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
count="$(wc -l < "$STAGE/.files.tsv" | tr -d ' ')"
echo "   $count objects + manifest, $(du -sh "$SRC" | cut -f1) raw"

if [ "$STAGE_ONLY" = 1 ]; then
  echo "== staged only: $STAGE"
  exit 0
fi

cdn_init
cdn_guard "$PREFIX" || exit 1

echo "== uploading to r2://$BUCKET/$PREFIX"
export PREFIX SRC
# shellcheck disable=SC2016
cut -f1 "$STAGE/.files.tsv" | xargs -P 16 -I{} bash -c 'cdn_put "$PREFIX/{}" "$SRC/{}"'
cdn_put "$PREFIX/$BUNDLE" "$STAGE/$BUNDLE"
cdn_put "$PREFIX/manifest.json" "$STAGE/manifest.json" "public, max-age=300"
cdn_latest "$FAMILY" "$VERSION"

echo "== published https://$CDN_HOST/$PREFIX/manifest.json"
