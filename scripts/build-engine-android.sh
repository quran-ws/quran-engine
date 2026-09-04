#!/usr/bin/env bash
# Cross-compile the Rust engine for Android and place the static libs where the
# Android library (packages/android/qvp) and the .so where the Flutter plugin expect them.
# Needs: rustup targets (aarch64/x86_64/armv7 android), cargo-ndk, ANDROID_NDK_HOME.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to your NDK (e.g. $ANDROID_HOME/ndk/27.2.12479018)}"
rustup target add aarch64-linux-android x86_64-linux-android armv7-linux-androideabi >/dev/null
command -v cargo-ndk >/dev/null || cargo install cargo-ndk
cargo ndk -t arm64-v8a -t x86_64 -t armeabi-v7a -o dist/android/jniLibs build -p qvp-ffi --release
for pair in arm64-v8a:aarch64-linux-android x86_64:x86_64-linux-android armeabi-v7a:armv7-linux-androideabi; do
  abi=${pair%%:*}; triple=${pair##*:}
  mkdir -p packages/android/qvp/prebuilt/$abi packages/flutter/qvp_flutter/android/src/main/jniLibs/$abi
  cp target/$triple/release/libqvp_ffi.a packages/android/qvp/prebuilt/$abi/
  cp dist/android/jniLibs/$abi/libqvp_ffi.so packages/flutter/qvp_flutter/android/src/main/jniLibs/$abi/
done
echo "engine built for android: $(ls packages/android/qvp/prebuilt)"
