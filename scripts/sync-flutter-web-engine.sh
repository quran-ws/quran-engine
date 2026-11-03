#!/usr/bin/env bash
# Refreshes the web engine the Flutter package ships as assets.
#
# Inputs:  web/qvp.js, and the Wasm build of crates/qvp-ffi.
# Outputs: packages/flutter/qvp_flutter/assets/js/qvp.js
#          packages/flutter/qvp_flutter/assets/wasm/qvp_ffi.wasm
#
# Neither file is tracked. They are build outputs, like every other platform's engine
# binary. The pubspec declares their directories as assets, so a native build runs without
# them and a web build or test needs them on disk: run this before `flutter build web` or
# `flutter test`, and again after any change to the reference wrapper or the FFI crate.
# The release workflow puts the wasm its verify job built, and web/qvp.js, in the pub.dev
# package.
#
# Builds with the cargo profile QVP_PROFILE names, `dist` by default. Shipped artifacts
# keep `dist`. A CI job that only needs the example to build sets QVP_PROFILE=release.
set -euo pipefail
cd "$(dirname "$0")/.."

ASSETS=packages/flutter/qvp_flutter/assets
profile="${QVP_PROFILE:-dist}"
WASM="target/wasm32-unknown-unknown/$profile/qvp_ffi.wasm"

cargo build -p qvp-ffi --profile "$profile" --target wasm32-unknown-unknown

mkdir -p "$ASSETS/js" "$ASSETS/wasm"
cp web/qvp.js "$ASSETS/js/qvp.js"
cp "$WASM" "$ASSETS/wasm/qvp_ffi.wasm"

echo "flutter web engine synced:"
ls -l "$ASSETS/js/qvp.js" "$ASSETS/wasm/qvp_ffi.wasm"
