#!/usr/bin/env bash
# One-time move of the published page data from qvp.quran.ws to cdn.quran.ws/qvp/.
#
#   scripts/migrate-cdn-prefix.sh            # copy and verify, delete nothing
#   scripts/migrate-cdn-prefix.sh --delete   # also drop the old keys, after the verify
#
# Source and destination are separate buckets, so this streams each object through the
# machine running it; `x-amz-copy-source` only works inside one bucket. Every object is
# checked against the manifest afterwards, because a streamed copy can truncate where a
# server-side copy cannot.
#
# Prefer `scripts/publish-cdn.sh <version> --from-release`, which rebuilds the same tree from
# the signed release tarball and is therefore verified by construction. Use this script when
# rebuilding is not possible.
#
# Credentials: see scripts/cdn-put.sh. SRC_BUCKET defaults to the old bucket.
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/cdn-put.sh

DELETE=0
[ "${1:-}" = --delete ] && DELETE=1
SRC_BUCKET="${SRC_BUCKET:-qvp-pages}"
cdn_init
WORK=$(mktemp -d); trap 'rm -rf "$WORK"' EXIT

# 1. List the source. A version folder holds about 1,213 objects, so page through the
#    listing rather than assuming one response covers it.
echo "== listing r2://$SRC_BUCKET"
token=""; : > "$WORK/keys"
while :; do
  url="$ENDPOINT/$SRC_BUCKET?list-type=2"
  [ -n "$token" ] && url="$url&continuation-token=$(jq -rn --arg t "$token" '$t|@uri')"
  curl -sS --aws-sigv4 "aws:amz:auto:s3" \
    --user "$R2_ACCESS_KEY_ID:$R2_SECRET_ACCESS_KEY" "$url" > "$WORK/page.xml"
  # `tr` maps one character to one; it cannot split on a tag. Match the elements instead.
  grep -o '<Key>[^<]*</Key>' "$WORK/page.xml" | sed -E 's|</?Key>||g' >> "$WORK/keys"
  grep -q '<IsTruncated>true' "$WORK/page.xml" || break
  token=$(grep -o '<NextContinuationToken>[^<]*' "$WORK/page.xml" | sed 's|<[^>]*>||' | tail -1)
done
grep -E '^v[0-9]' "$WORK/keys" | sort > "$WORK/todo"
echo "   $(wc -l < "$WORK/todo" | tr -d ' ') objects under v*/"

# 2. Copy. The headers are set here rather than carried over, because R2 drops Cache-Control
#    on a copy and the whole cache contract depends on it.
move() {
  local key="$1" dst="qvp/$1" cache="public, max-age=31536000, immutable" f
  case "$key" in */manifest.json) cache="public, max-age=300" ;; esac
  f="$WORK/$(echo "$key" | tr / _)"
  curl -fsS --aws-sigv4 "aws:amz:auto:s3" \
    --user "$R2_ACCESS_KEY_ID:$R2_SECRET_ACCESS_KEY" \
    "$ENDPOINT/$SRC_BUCKET/$key" -o "$f"
  cdn_put "$dst" "$f" "$cache"
  rm -f "$f"
}
export -f move
export WORK SRC_BUCKET
echo "== copying to r2://$BUCKET/qvp/"
xargs -P 16 -I{} bash -c 'move "{}"' < "$WORK/todo"

# 3. Verify over HTTPS, against the manifest the release signed. This is the step that makes
#    the copy trustworthy; without it a short read is invisible.
for version in $(cut -d/ -f1 "$WORK/todo" | sort -u); do
  echo "== verifying qvp/$version"
  curl -fsS "https://$CDN_HOST/qvp/$version/manifest.json" -o "$WORK/m.json"
  fail=0
  while IFS=$'\t' read -r name want; do
    got=$(curl -fsS "https://$CDN_HOST/qvp/$version/$name" | { sha256 /dev/stdin; })
    [ "$want" = "$got" ] || { echo "   $name: want $want got $got" >&2; fail=1; }
  done < <(jq -r '.files[]|"\(.name)\t\(.sha256)"' "$WORK/m.json")
  [ "$fail" = 0 ] || { echo "verify failed for $version; nothing deleted" >&2; exit 1; }
  echo "   $(jq '.files|length' "$WORK/m.json") objects match the manifest"
  cdn_latest qvp "$version"
done

# 4. Delete the old keys, only on request and only after the verify passed.
[ "$DELETE" = 1 ] || { echo "== done; old keys kept (pass --delete to drop them)"; exit 0; }
echo "== deleting from r2://$SRC_BUCKET"
xargs -P 16 -I{} curl -sS -o /dev/null -X DELETE \
  --aws-sigv4 "aws:amz:auto:s3" --user "$R2_ACCESS_KEY_ID:$R2_SECRET_ACCESS_KEY" \
  "$ENDPOINT/$SRC_BUCKET/{}" < "$WORK/todo"
echo "== done"
