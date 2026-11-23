#!/usr/bin/env bash
# Copy the 29-page example set (docs/EXAMPLE-APP.md) into a platform's example app.
#
# Usage: scripts/sync-example-data.sh android|flutter|react-native|ios|web|all
# Input: dist/pages/ (run scripts/sync-test-data.sh first), and dist/letters/ when it is
# there (docs/DATA.md): the letter layer NNN.qvl goes with its page. QVP_PAGES_DIR and
# QVP_LETTERS_DIR name other directories. A layer attaches only to the page file it was
# built with. Output: the example's gitignored assets directory; iOS and web take all 604
# pages, and iOS delegates to example/sync-pages.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
src="${QVP_PAGES_DIR:-dist/pages}"
letters="${QVP_LETTERS_DIR:-dist/letters}"
[ -f "$src/atlas.qva" ] || { echo "error: $src is missing; run scripts/sync-test-data.sh" >&2; exit 1; }

pages="$(seq -f '%03g' 1 21) $(seq -f '%03g' 440 445) 582 604"

copy() { # destination directory
  mkdir -p "$1"
  rm -f "$1"/*.qvl
  n=0
  for p in $pages; do
    cp "$src/$p.qvp" "$src/$p.words.json" "$1/"
    if [ -f "$letters/$p.qvl" ]; then cp "$letters/$p.qvl" "$1/"; n=$((n + 1)); fi
  done
  cp "$src/atlas.qva" "$1/"
  if [ -f "$src/ornaments.qvo" ]; then cp "$src/ornaments.qvo" "$1/"; fi
  echo "synced   $1 (29 pages + atlas, $n letter layers)"
}

copy_all() { # destination directory
  mkdir -p "$1"
  rm -f "$1"/*.qvl
  cp "$src"/*.qvp "$src"/*.words.json "$src/atlas.qva" "$1/"
  if [ -f "$src/ornaments.qvo" ]; then cp "$src/ornaments.qvo" "$1/"; fi
  n=0
  if [ -f "$letters/001.qvl" ]; then cp "$letters"/*.qvl "$1/"; n=$(find "$letters" -maxdepth 1 -name '*.qvl' | wc -l); fi
  cat "$src/.tag" 2>/dev/null > "$1/.tag" || echo local > "$1/.tag"
  echo "synced   $1 (604 pages + atlas, $n letter layers)"
}

for platform in "${@:-all}"; do
  case "$platform" in
    android) copy packages/android/example/src/main/assets/pages ;;
    flutter) copy packages/flutter/qvp_flutter/example/assets/pages ;;
    react-native)
      copy packages/react-native/example/android/app/src/main/assets/pages
      copy packages/react-native/example/ios/QvpExample/pages ;;
    ios)
      # sync-pages.sh works in its own directory, so it takes absolute paths
      abs() { case "$1" in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
      QVP_PAGES="$(abs "$src")" QVP_LETTERS="$(abs "$letters")" packages/ios/example/sync-pages.sh ;;
    web) copy_all web/pages ;;
    all) "$0" android flutter react-native ios web ;;
    *) echo "unknown platform: $platform" >&2; exit 2 ;;
  esac
done
