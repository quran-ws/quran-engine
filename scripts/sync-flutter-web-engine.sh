#!/usr/bin/env bash
# Refreshes the web engine the Flutter package ships as assets.
#
# Inputs:  web/qvp.js, and the Wasm build of crates/qvp-ffi.
# Outputs: packages/flutter/qvp_flutter/assets/js/qvp.js
#          packages/flutter/qvp_flutter/assets/wasm/qvp_ffi.wasm
#
# Both files are committed, unlike every other platform's engine binary, because
# `flutter build web` reads them as package assets and a published package cannot run a
# script first. scripts/check-parity.py fails when the qvp.js copy drifts from web/qvp.js,
# so run this after any change to the reference wrapper or the FFI crate.
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
