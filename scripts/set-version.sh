#!/usr/bin/env bash
# Set one version everywhere: Cargo.toml (the source of truth) and every package manifest.
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
open("Cargo.toml", "w").write(cargo)
import glob
for p in glob.glob("crates/*/Cargo.toml"):
    t = open(p).read()
    t2 = re.sub(r'(qvp-[a-z]+ = \{[^}]*?version = ")[^"]+(")', lambda m: m.group(1) + v + m.group(2), t)
    if t2 != t: open(p, "w").write(t2)
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
PY
sed -i.bak -E "s/^version: .*/version: $v/" packages/flutter/qvp_flutter/pubspec.yaml && rm packages/flutter/qvp_flutter/pubspec.yaml.bak
sed -i.bak -E "s/^version = \".*\"$/version = \"$v\"/" packages/flutter/qvp_flutter/android/build.gradle && rm packages/flutter/qvp_flutter/android/build.gradle.bak
cargo update -q --workspace 2>/dev/null || true
scripts/check-versions.sh
