#!/usr/bin/env bash
# Check that the Flutter package's dart:ffi bindings match the C header.
#
# packages/flutter/qvp_flutter/lib/src/bindings.dart is generated from
# crates/qvp-ffi/include/qvp.h by package:ffigen (tool/ffigen.dart in the package). This
# script generates a fresh copy into a temporary directory and compares it with the
# committed file, so a header change cannot drift from the Dart structs.
#
# Needs: Flutter (it brings the Dart SDK) and libclang (Xcode or the Command Line Tools on
# macOS, the libclang-dev package on Linux).
# Output: exit 0 when the two files agree; the diff and exit 1 when they differ.
set -euo pipefail
cd "$(dirname "$0")/../packages/flutter/qvp_flutter"

committed=lib/src/bindings.dart
lock=example/pubspec.lock
tmp="$(mktemp -d)"

# `flutter pub get` also resolves the example and may rewrite example/pubspec.lock. Keep a
# copy and put it back on exit, so the check leaves the tree as it found it.
if [ -f "$lock" ]; then cp "$lock" "$tmp/pubspec.lock"; fi
cleanup() {
  if [ -f "$tmp/pubspec.lock" ]; then cp "$tmp/pubspec.lock" "$lock"; fi
  rm -rf "$tmp"
}
trap cleanup EXIT

# The generator runs from the package, so it needs the package's dependencies. A checkout
# whose .dart_tool predates the ffigen dev dependency has no ffigen to run. Resolve the
# dependencies on every run: offline first, and the network only when the cache lacks a
# package. The output stays hidden unless both attempts fail.
if ! flutter pub get --offline >"$tmp/pub.log" 2>&1 &&
  ! flutter pub get >"$tmp/pub.log" 2>&1; then
  cat "$tmp/pub.log" >&2
  echo "FAIL flutter pub get did not resolve the package's dependencies" >&2
  echo "     Run it by hand: cd packages/flutter/qvp_flutter && flutter pub get" >&2
  exit 1
fi

# ffigen warns once for each opaque struct (QvpPage and the others have no definition by
# design). The warnings are noise here, so only errors reach the output.
if ! dart run tool/ffigen.dart "$tmp/bindings.dart" >"$tmp/log" 2>&1; then
  cat "$tmp/log" >&2
  echo "FAIL ffigen did not generate the bindings" >&2
  echo "     Check that libclang is installed, then run it by hand:" >&2
  echo "     cd packages/flutter/qvp_flutter && flutter pub get && dart run tool/ffigen.dart" >&2
  exit 1
fi

if ! diff -u "$committed" "$tmp/bindings.dart"; then
  echo "FAIL packages/flutter/qvp_flutter/$committed differs from the header." >&2
  echo "     Regenerate it: cd packages/flutter/qvp_flutter && dart run tool/ffigen.dart" >&2
  exit 1
fi
echo "ok   packages/flutter/qvp_flutter/$committed matches crates/qvp-ffi/include/qvp.h"
