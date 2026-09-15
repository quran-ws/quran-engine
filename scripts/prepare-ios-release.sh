#!/usr/bin/env bash
# Build the iOS binary, attach it to a draft GitHub release, and stamp its immutable URL
# and SwiftPM checksum into the root package manifest. Run this once on the release branch
# before its final commit. The tag workflow verifies and publishes the prepared artifact.
set -euo pipefail
cd "$(dirname "$0")/.."

version="${1:?usage: scripts/prepare-ios-release.sh X.Y.Z}"
tag="v$version"
have="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[ "$version" = "$have" ] || {
  echo "error: requested version $version does not match Cargo.toml version $have" >&2
  exit 1
}

git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1 && {
  echo "error: tag $tag already exists" >&2
  exit 1
}

if gh api --paginate "repos/{owner}/{repo}/releases" \
  --jq ".[] | select(.tag_name == \"$tag\") | .id" | grep -q .; then
  echo "error: a release for $tag already exists" >&2
  exit 1
fi

scripts/build-engine-ios.sh
archive="dist/ios/QvpEngine.xcframework.zip"
checksum_file="$archive.checksum"
(
  cd packages/ios/QvpKit
  zip -qry ../../../"$archive" QvpEngine.xcframework
)
swift package compute-checksum "$archive" > "$checksum_file"
checksum="$(cat "$checksum_file")"

python3 - "$tag" "$checksum" <<'PY'
from pathlib import Path
import re
import sys

tag, checksum = sys.argv[1:]
manifest = Path("Package.swift")
text = manifest.read_text()
text, urls = re.subn(
    r"(releases/download/)v[^/]+(/QvpEngine\.xcframework\.zip)",
    rf"\g<1>{tag}\g<2>",
    text,
    count=1,
)
text, checksums = re.subn(
    r'(checksum: ")[0-9a-f]{64}(")',
    rf"\g<1>{checksum}\g<2>",
    text,
    count=1,
)
if urls != 1 or checksums != 1:
    raise SystemExit("error: Package.swift does not contain one release URL and checksum")
manifest.write_text(text)
PY

gh release create "$tag" --draft --target main --title "$tag" \
  --notes "Prepared iOS artifact for $tag. The tag release workflow publishes this draft." \
  "$archive" "$checksum_file"

echo "prepared $tag: $archive"
echo "SwiftPM checksum: $checksum"
