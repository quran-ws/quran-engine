#!/usr/bin/env bash
# Every version string in the repository must equal the workspace version in Cargo.toml.
# Input: the manifests listed below. Output: one line per manifest; exit 1 on any mismatch.
set -euo pipefail
cd "$(dirname "$0")/.."

want="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[ -n "$want" ] || { echo "error: no workspace version in Cargo.toml" >&2; exit 1; }
status=0

check() { # label, found
  if [ "$2" = "$want" ]; then printf 'ok       %-52s %s\n' "$1" "$2"
  else printf 'MISMATCH %-52s %s (want %s)\n' "$1" "$2" "$want"; status=1; fi
}

check package.json "$(python3 -c 'import json;print(json.load(open("package.json"))["version"])')"
check packages/react-native/qvp-react-native/package.json "$(python3 -c 'import json;print(json.load(open("packages/react-native/qvp-react-native/package.json"))["version"])')"
check packages/flutter/qvp_flutter/pubspec.yaml "$(sed -n 's/^version: //p' packages/flutter/qvp_flutter/pubspec.yaml)"
check packages/flutter/qvp_flutter/android/build.gradle "$(sed -n 's/^version = "\(.*\)"$/\1/p' packages/flutter/qvp_flutter/android/build.gradle)"

echo "workspace version: $want"
exit $status
