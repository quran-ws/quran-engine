#!/usr/bin/env bash
# Verify that an AAR contains every supported ABI and 16 KiB-aligned 64-bit ELF segments.
set -euo pipefail

aar="${1:?usage: check-android-page-size.sh <aar>}"
[ -f "$aar" ] || { echo "AAR not found: $aar" >&2; exit 1; }

if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ]; then
  candidate="$ANDROID_HOME/ndk/27.2.12479018"
  [ -d "$candidate" ] && export ANDROID_NDK_HOME="$candidate"
fi
: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME or install NDK 27.2.12479018 under ANDROID_HOME}"

readelf="$(find "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt" -path '*/bin/llvm-readelf' -print -quit)"
[ -n "$readelf" ] || { echo "llvm-readelf not found under $ANDROID_NDK_HOME" >&2; exit 1; }

tmp="$(mktemp -d)"
trap 'find "$tmp" -type f -delete; rmdir "$tmp"' EXIT

for abi in arm64-v8a x86_64 armeabi-v7a; do
  library="$tmp/$abi.so"
  unzip -p "$aar" "jni/$abi/libqvp_jni.so" > "$library" || {
    echo "Missing jni/$abi/libqvp_jni.so in $aar" >&2
    exit 1
  }

  [ "$abi" = armeabi-v7a ] && continue
  alignments="$("$readelf" -lW "$library" | awk '$1 == "LOAD" { print $NF }')"
  [ -n "$alignments" ] || { echo "No ELF load segments in $abi/libqvp_jni.so" >&2; exit 1; }
  while IFS= read -r alignment; do
    if (( alignment < 0x4000 )); then
      echo "$abi/libqvp_jni.so has $alignment load alignment; expected at least 0x4000" >&2
      exit 1
    fi
  done <<< "$alignments"
  echo "$abi/libqvp_jni.so: 16 KiB ELF load alignment"
done
