#!/usr/bin/env bash
# Copy the 29-page example set (docs/EXAMPLE-APP.md) into a platform's example app.
#
# Usage: scripts/sync-example-data.sh android|flutter|react-native|ios|all
# Input: dist/pages/ (run scripts/sync-test-data.sh first). Output: the example's gitignored
# assets directory; iOS delegates to example/sync-pages.sh, which bundles all 604 pages.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f dist/pages/atlas.qva ] || { echo "error: dist/pages is missing; run scripts/sync-test-data.sh" >&2; exit 1; }

pages="$(seq -f '%03g' 1 21) $(seq -f '%03g' 440 445) 582 604"

copy() { # destination directory
  mkdir -p "$1"
  for p in $pages; do cp "dist/pages/$p.qvp" "dist/pages/$p.words.json" "$1/"; done
  cp dist/pages/atlas.qva "$1/"
  echo "synced   $1 (29 pages + atlas)"
}

for platform in "${@:-all}"; do
  case "$platform" in
    android) copy packages/android/example/src/main/assets/pages ;;
    flutter) copy packages/flutter/qvp_flutter/example/assets/pages ;;
    react-native) copy packages/react-native/example/android/app/src/main/assets/pages ;;
    ios) packages/ios/example/sync-pages.sh ;;
    all) "$0" android flutter react-native ios ;;
    *) echo "unknown platform: $platform" >&2; exit 2 ;;
  esac
done
