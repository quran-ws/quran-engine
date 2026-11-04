#!/usr/bin/env bash
# Every version string in the repository must equal the workspace version in Cargo.toml, and
# every NDK version must equal QVP_NDK_VERSION in scripts/build-engine-android.sh.
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
check packages/flutter/qvp_flutter/ios/qvp_flutter.podspec "$(sed -n 's/.*s\.version *= *'"'"'\([^'"'"']*\)'"'"'.*/\1/p' packages/flutter/qvp_flutter/ios/qvp_flutter.podspec)"
check packages/flutter/qvp_flutter/macos/qvp_flutter.podspec "$(sed -n 's/.*s\.version *= *'"'"'\([^'"'"']*\)'"'"'.*/\1/p' packages/flutter/qvp_flutter/macos/qvp_flutter.podspec)"
check packages/android/build.gradle.kts/qvpVersion "$(sed -n 's/.*gradleProperty("qvpVersion").getOrElse("\([^"]*\)").*/\1/p' packages/android/build.gradle.kts)"
check packages/ios/example/project.yml "$(sed -n 's/^ *CFBundleShortVersionString: *//p' packages/ios/example/project.yml | tr -d '"')"
# [workspace.dependencies] holds exactly qvp-format and qvp-core. A different set means the
# line shape changed and the match below no longer sees it, or a crate was added or lost.
deps="$(sed -n 's/^\(qvp-[a-z]*\) = { path = .*version = "\(.*\)" }$/\1=\2/p' Cargo.toml)"
names="$(printf '%s\n' "$deps" | cut -d= -f1 | sort | tr '\n' ' ')"
if [ "$names" != "qvp-core qvp-format " ]; then
  echo "MISMATCH Cargo.toml [workspace.dependencies] lists '$names'; want qvp-core and qvp-format" ; status=1
fi
for dep in $deps; do
  check "Cargo.toml [workspace.dependencies] ${dep%%=*}" "${dep#*=}"
done
# Internal crates are declared once, in the workspace. A crate manifest must not carry its own version for one.
if grep -nE '^qvp-[a-z]+ = .*version = "' crates/*/Cargo.toml; then
  echo "MISMATCH a crate manifest declares an internal dependency version; use workspace = true" ; status=1
fi
check Package.swift/XCFramework "$(sed -n 's|.*releases/download/v\([^/]*\)/QvpEngine\.xcframework\.zip.*|\1|p' Package.swift)"

# The NDK version is defined once, as QVP_NDK_VERSION in scripts/build-engine-android.sh. Every
# other mention must equal it. Files read, all of them tracked:
#   - every build.gradle and build.gradle.kts under packages/ (the ndkVersion pins)
#   - every README.md under packages/
#   - packages/android/AGENTS.md, docs/MACOS.md and scripts/check-android-page-size.sh
# A full version (27.2.12479018) must match exactly. A bare major ("NDK 27") must match the major.
ndk="$(sed -n 's/^QVP_NDK_VERSION=//p' scripts/build-engine-android.sh)"
[ -n "$ndk" ] || { echo "error: no QVP_NDK_VERSION in scripts/build-engine-android.sh" >&2; exit 1; }
while IFS= read -r f; do
  found="$(grep -iE 'ndk' "$f" | grep -oE '[0-9]+\.[0-9]+\.[0-9]{5,}' | sort -u | tr '\n' ' ' || true)"
  majors="$(grep -oE 'NDK [0-9]+([^.0-9]|$)' "$f" | grep -oE '[0-9]+' | sort -u | tr '\n' ' ' || true)"
  [ -n "$found$majors" ] || continue
  bad=""
  for v in $found; do [ "$v" = "$ndk" ] || bad="$bad $v"; done
  for m in $majors; do [ "$m" = "${ndk%%.*}" ] || bad="$bad $m"; done
  if [ -z "$bad" ]; then printf 'ok       %-52s NDK %s\n' "$f" "$ndk"
  else printf 'MISMATCH %-52s NDK%s (want %s)\n' "$f" "$bad" "$ndk"; status=1; fi
done < <(git ls-files -- 'packages/*build.gradle' 'packages/*build.gradle.kts' 'packages/*README.md' \
  packages/android/AGENTS.md docs/MACOS.md scripts/check-android-page-size.sh)

echo "workspace version: $want"
exit $status
