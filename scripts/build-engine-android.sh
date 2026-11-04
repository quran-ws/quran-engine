#!/usr/bin/env bash
# Cross-compile the Rust engine for Android and place the static libs where the
# Android library (packages/android/qvp) and the .so where the Flutter plugin expect them.
# Builds with the cargo profile QVP_PROFILE names, `dist` by default. Shipped artifacts
# keep `dist`. A CI job that only needs a debug example APK sets QVP_PROFILE=release,
# which skips LTO and its intermediate objects for three targets.
# Needs: rustup targets (aarch64/x86_64/armv7 android), cargo-ndk, ANDROID_NDK_HOME.
set -euo pipefail
cd "$(dirname "$0")/.."
# The Android library compiles its JNI shim against a copy of the header. The copy is
# generated here, never edited, before any toolchain check, so syncing the header needs no
# Android toolchain. scripts/check-parity.py fails when the copy drifts from the source.
cp crates/qvp-ffi/include/qvp.h packages/android/qvp/src/main/cpp/qvp.h
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ]; then
  candidate="$ANDROID_HOME/ndk/27.2.12479018"
  [ -d "$candidate" ] && export ANDROID_NDK_HOME="$candidate"
fi
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME or install NDK 27.2.12479018 under ANDROID_HOME}"
rustup target add aarch64-linux-android x86_64-linux-android armv7-linux-androideabi >/dev/null
command -v cargo-ndk >/dev/null || cargo install cargo-ndk
profile="${QVP_PROFILE:-dist}"
cargo ndk -t arm64-v8a -t x86_64 -t armeabi-v7a -o dist/android/jniLibs build -p qvp-ffi --profile "$profile"
for pair in arm64-v8a:aarch64-linux-android x86_64:x86_64-linux-android armeabi-v7a:armv7-linux-androideabi; do
  abi=${pair%%:*}; triple=${pair##*:}
  mkdir -p "packages/android/qvp/prebuilt/$abi" "packages/flutter/qvp_flutter/android/src/main/jniLibs/$abi"
  cp "target/$triple/$profile/libqvp_ffi.a" "packages/android/qvp/prebuilt/$abi/"
  cp "dist/android/jniLibs/$abi/libqvp_ffi.so" "packages/flutter/qvp_flutter/android/src/main/jniLibs/$abi/"
done
echo "engine built for android ($profile profile): $(ls packages/android/qvp/prebuilt)"
