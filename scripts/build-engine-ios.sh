#!/usr/bin/env bash
# Cross-compile the Rust engine for iOS and package it as an XCFramework for the Swift
# package (packages/ios/QvpKit). Static libs for device (arm64), simulator (arm64 + x86_64,
# lipo'd into one fat slice) and — so `swift test` / macOS hosts work too — the Mac (arm64).
# Output: packages/ios/QvpKit/QvpEngine.xcframework (a build artefact, gitignored).
# Needs: rustup (targets are added on demand) and Xcode command line tools.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-15.0}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

IOS_TARGETS=(aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios)
MAC_TARGETS=(aarch64-apple-darwin)
rustup target add "${IOS_TARGETS[@]}" "${MAC_TARGETS[@]}" >/dev/null

for t in "${IOS_TARGETS[@]}" "${MAC_TARGETS[@]}"; do
  echo "== cargo build -p qvp-ffi --release --target $t"
  cargo build -p qvp-ffi --release --target "$t"
done

OUT=dist/ios
rm -rf "$OUT"
mkdir -p "$OUT/sim" "$OUT/include"

# one fat simulator slice (arm64 + x86_64)
lipo -create \
  target/aarch64-apple-ios-sim/release/libqvp_ffi.a \
  target/x86_64-apple-ios/release/libqvp_ffi.a \
  -output "$OUT/sim/libqvp_ffi.a"

# headers + module map shared by every slice: `import QvpFFI` in Swift
cp crates/qvp-ffi/include/qvp.h "$OUT/include/qvp.h"
cat > "$OUT/include/module.modulemap" <<'EOF'
module QvpFFI {
    header "qvp.h"
    export *
}
EOF

XC=packages/ios/QvpKit/QvpEngine.xcframework
rm -rf "$XC"
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libqvp_ffi.a -headers "$OUT/include" \
  -library "$OUT/sim/libqvp_ffi.a"                        -headers "$OUT/include" \
  -library target/aarch64-apple-darwin/release/libqvp_ffi.a -headers "$OUT/include" \
  -output "$XC"

echo "engine packaged: $XC"
ls "$XC"
