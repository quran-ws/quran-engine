#!/usr/bin/env bash
# Set one version everywhere: Cargo.toml (the source of truth), every package manifest, the
# Android library's default qvpVersion, the iOS example's CFBundleShortVersionString in
# project.yml (xcodegen writes Info.plist from it) and the React Native example's lock file
# record of the package it links.
#
# Usage: scripts/set-version.sh X.Y.Z
# Output: the edited files; scripts/check-versions.sh confirms they agree.
set -euo pipefail
cd "$(dirname "$0")/.."
v="${1:?usage: scripts/set-version.sh X.Y.Z}"
echo "$v" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.]+)?$' || { echo "error: '$v' is not X.Y.Z" >&2; exit 2; }

python3 - "$v" <<'PY'
import json, re, sys
v = sys.argv[1]
cargo = open("Cargo.toml").read()
cargo = re.sub(r'^version = ".*"$', f'version = "{v}"', cargo, count=1, flags=re.M)
# The internal crates, qvp-format and qvp-core, are under [workspace.dependencies].
cargo, replacements = re.subn(r'^(qvp-[a-z]+ = \{ path = "[^"]+", version = ")[^"]+(" \})$', lambda m: m.group(1) + v + m.group(2), cargo, flags=re.M)
if replacements != 2:
    raise SystemExit(f"error: Cargo.toml [workspace.dependencies] has {replacements} internal crate versions, want 2")
open("Cargo.toml", "w").write(cargo)
for p in ("package.json", "packages/react-native/qvp-react-native/package.json"):
    d = json.load(open(p)); d["version"] = v
    json.dump(d, open(p, "w"), indent=2, ensure_ascii=False); open(p, "a").write("\n")
swift = open("Package.swift").read()
swift, replacements = re.subn(
    r"(releases/download/)v[^/]+(/QvpEngine\.xcframework\.zip)",
    rf"\g<1>v{v}\g<2>",
    swift,
    count=1,
)
if replacements != 1:
    raise SystemExit("error: Package.swift does not contain one XCFramework release URL")
open("Package.swift", "w").write(swift)
gradle = open("packages/android/build.gradle.kts").read()
gradle, replacements = re.subn(
    r'(gradleProperty\("qvpVersion"\)\.getOrElse\(")[^"]*("\))',
    lambda m: m.group(1) + v + m.group(2),
    gradle,
)
if replacements != 1:
    raise SystemExit("error: packages/android/build.gradle.kts does not contain one default qvpVersion")
open("packages/android/build.gradle.kts", "w").write(gradle)
spec = open("packages/ios/example/project.yml").read()
spec, replacements = re.subn(
    r'^(\s*CFBundleShortVersionString:\s*)"?[^"\n]*"?$',
    lambda m: m.group(1) + '"' + v + '"',
    spec,
    flags=re.M,
)
if replacements != 1:
    raise SystemExit("error: packages/ios/example/project.yml does not contain one CFBundleShortVersionString")
open("packages/ios/example/project.yml", "w").write(spec)
# The React Native example links the package from ../qvp-react-native, and its lock file
# records the version it linked.
lock_path = "packages/react-native/example/package-lock.json"
lock = open(lock_path).read()
lock, replacements = re.subn(
    r'("\.\./qvp-react-native": \{\s*"name": "@quran\.ws/qvp-react-native",\s*"version": ")[^"]*(")',
    lambda m: m.group(1) + v + m.group(2),
    lock,
)
if replacements != 1:
    raise SystemExit(f"error: {lock_path} does not record one linked qvp-react-native version")
open(lock_path, "w").write(lock)
PY
sed -i.bak -E "s/^version: .*/version: $v/" packages/flutter/qvp_flutter/pubspec.yaml && rm packages/flutter/qvp_flutter/pubspec.yaml.bak
sed -i.bak -E "s/^version = \".*\"$/version = \"$v\"/" packages/flutter/qvp_flutter/android/build.gradle && rm packages/flutter/qvp_flutter/android/build.gradle.bak
sed -i.bak -E "s/s\.version *= *'.*'/s.version          = '$v'/" packages/flutter/qvp_flutter/ios/qvp_flutter.podspec && rm packages/flutter/qvp_flutter/ios/qvp_flutter.podspec.bak
sed -i.bak -E "s/s\.version *= *'.*'/s.version          = '$v'/" packages/flutter/qvp_flutter/macos/qvp_flutter.podspec && rm packages/flutter/qvp_flutter/macos/qvp_flutter.podspec.bak
cargo update -q --workspace 2>/dev/null || true
scripts/check-versions.sh
