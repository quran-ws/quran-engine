#!/usr/bin/env bash
# Copy QvpKit into this package, for the pod: the Swift sources, and the release URL and
# checksum of the XCFramework the root Package.swift names.
#
# Usage: ios/vendor-qvpkit.sh (from the package directory, inside the quran-engine repository)
# Input:  ../../ios/QvpKit/Sources/QvpKit/*.swift and ../../../Package.swift
# Output: ios/vendor/QvpKit/*.swift and ios/vendor/QvpEngine.json (both gitignored)
#
# `npm pack` and `npm publish` run it (the prepack script), so the tarball holds the sources.
# qvp-react-native.podspec runs it at `pod install` inside the repository, so the pod builds
# the QvpKit sources of this checkout.
set -euo pipefail
cd "$(dirname "$0")/.."

kit=../../ios/QvpKit/Sources/QvpKit
manifest=../../../Package.swift
[ -d "$kit" ] || { echo "error: $kit is missing; run this script inside the quran-engine repository" >&2; exit 1; }

rm -rf ios/vendor/QvpKit
mkdir -p ios/vendor/QvpKit
cp "$kit"/*.swift ios/vendor/QvpKit/

python3 - "$manifest" ios/vendor/QvpEngine.json <<'PY'
import json
import pathlib
import re
import sys

text = pathlib.Path(sys.argv[1]).read_text()
url = re.search(r'url: "(https://[^"]+/QvpEngine\.xcframework\.zip)"', text)
checksum = re.search(r'checksum: "([0-9a-f]{64})"', text)
if not url or not checksum:
    raise SystemExit(f"error: {sys.argv[1]} has no XCFramework URL and checksum")
pathlib.Path(sys.argv[2]).write_text(json.dumps({"url": url.group(1), "sha256": checksum.group(1)}, indent=2) + "\n")
PY
echo "vendored $(find ios/vendor/QvpKit -name '*.swift' | wc -l | tr -d ' ') QvpKit sources and ios/vendor/QvpEngine.json" >&2
