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
for p in ("package.json", "packages/react-native/qvp-react-native/package.json"):
    d = json.load(open(p)); d["version"] = v
    json.dump(d, open(p, "w"), indent=2, ensure_ascii=False); open(p, "a").write("\n")
PY
sed -i.bak -E "s/^version: .*/version: $v/" packages/flutter/qvp_flutter/pubspec.yaml && rm packages/flutter/qvp_flutter/pubspec.yaml.bak
sed -i.bak -E "s/^version = \".*\"$/version = \"$v\"/" packages/flutter/qvp_flutter/android/build.gradle && rm packages/flutter/qvp_flutter/android/build.gradle.bak
cargo update -q --workspace 2>/dev/null || true
scripts/check-versions.sh
