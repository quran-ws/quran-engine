#!/usr/bin/env bash
# Refreshes the web engine the Flutter package ships as assets.
#
# Inputs:  web/qvp.js, and the Wasm build of crates/qvp-ffi.
# Outputs: packages/flutter/qvp_flutter/assets/js/qvp.js
#          packages/flutter/qvp_flutter/assets/wasm/qvp_ffi.wasm
#
# Neither file is tracked. They are build outputs, like every other platform's engine
# binary. The pubspec declares them as assets, so `flutter analyze`, `flutter test` and
# `flutter build web` need them on disk: run this before those commands, and again after
# any change to the reference wrapper or the FFI crate. The release workflow builds them
# into the pub.dev package. scripts/check-parity.py fails when the qvp.js copy on disk
# drifts from web/qvp.js.
set -euo pipefail
cd "$(dirname "$0")/.."

ASSETS=packages/flutter/qvp_flutter/assets
WASM=target/wasm32-unknown-unknown/release/qvp_ffi.wasm

cargo build -p qvp-ffi --release --target wasm32-unknown-unknown

mkdir -p "$ASSETS/js" "$ASSETS/wasm"
cp web/qvp.js "$ASSETS/js/qvp.js"
cp "$WASM" "$ASSETS/wasm/qvp_ffi.wasm"

echo "flutter web engine synced:"
ls -l "$ASSETS/js/qvp.js" "$ASSETS/wasm/qvp_ffi.wasm"
