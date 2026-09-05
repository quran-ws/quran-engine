#!/usr/bin/env bash
# Bundle the iOS wrapper + demo into one zip an iOS developer can open without Rust or the data
# pipeline: packages/ios (Swift package with the built QvpEngine.xcframework, the demo project with
# its synced pages/ assets, README) plus docs/API.md and the C header.
# Needs: scripts/build-engine-ios.sh run first, and dist/pages from the converter.
# Output: dist/qvp-ios-demo.zip
set -euo pipefail
cd "$(dirname "$0")/.."

[ -d packages/ios/QvpKit/QvpEngine.xcframework ] || { echo "error: no QvpEngine.xcframework — run scripts/build-engine-ios.sh" >&2; exit 1; }
packages/ios/Demo/sync-pages.sh

OUT=dist/qvp-ios-demo.zip
mkdir -p dist
rm -f "$OUT"
zip -qr "$OUT" packages/ios docs/API.md crates/qvp-ffi/include/qvp.h \
  -x "packages/ios/Demo/build/*" "packages/ios/QvpKit/.build/*" "packages/ios/QvpKit/.swiftpm/*" "*/xcuserdata/*" "*.DS_Store"
echo "packaged: $OUT ($(du -h "$OUT" | cut -f1))"
