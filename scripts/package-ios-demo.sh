#!/usr/bin/env bash
# One file to hand an iOS developer: the SDK, the prebuilt engine, the complete mushaf and the
# demo app. They need Xcode and nothing else — no Rust, no page-data pipeline, no repo access.
#
# Layout inside the zip (Demo and QvpKit stay siblings, as the demo project expects):
#   README.md  QvpKit/  Demo/(+pages)  docs/{API.md,qvp.h,ios.md}
#
# Needs: scripts/build-engine-ios.sh run first (for QvpEngine.xcframework).
# Page data is fetched by Demo/sync-pages.sh — which needs `gh auth login` while the release is private.
# Output: dist/qvp-ios-demo.zip
set -euo pipefail
cd "$(dirname "$0")/.."

[ -d packages/ios/QvpKit/QvpEngine.xcframework ] || { echo "error: no QvpEngine.xcframework — run scripts/build-engine-ios.sh" >&2; exit 1; }
packages/ios/Demo/sync-pages.sh

STAGE=$(mktemp -d)/qvp-ios-demo
trap 'rm -rf "$(dirname "$STAGE")"' EXIT
mkdir -p "$STAGE/docs"

rsync -a --exclude 'build/' --exclude '.build/' --exclude '.swiftpm/' --exclude 'xcuserdata/' \
      --exclude '.DS_Store' packages/ios/QvpKit "$STAGE/"
rsync -a --exclude 'build/' --exclude 'xcuserdata/' --exclude '.DS_Store' packages/ios/Demo "$STAGE/"
cp docs/SHARE-IOS.md "$STAGE/README.md"
cp docs/API.md "$STAGE/docs/API.md"
cp crates/qvp-ffi/include/qvp.h "$STAGE/docs/qvp.h"
cp packages/ios/README.md "$STAGE/docs/ios.md"

[ -f "$STAGE/Demo/pages/atlas.qva" ] || { echo "error: no page data staged — Demo/sync-pages.sh did not run" >&2; exit 1; }
[ -d "$STAGE/QvpKit/QvpEngine.xcframework" ] || { echo "error: the engine did not make it into the bundle" >&2; exit 1; }

OUT=$PWD/dist/qvp-ios-demo.zip
mkdir -p dist; rm -f "$OUT"
(cd "$(dirname "$STAGE")" && zip -qr "$OUT" qvp-ios-demo)
echo "packaged: dist/qvp-ios-demo.zip ($(du -h "$OUT" | cut -f1)) — $(find "$STAGE/Demo/pages" -name '*.qvp' | wc -l | tr -d ' ') pages, engine + SDK + docs"
