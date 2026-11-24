#!/usr/bin/env bash
# Point the Flutter package's two Package.swift files at the released XCFramework, for
# pub.dev. The package does not carry the framework: Swift Package Manager and the podspec
# download it. In the repository the manifests name the local build, and a manifest that
# chose between the two would not work: Swift Package Manager caches its answer, and keeps
# the downloaded engine after a local build.
#
# Usage: scripts/stage-flutter-apple.sh [package]
# Input: the URL and the checksum in the root Package.swift, and the package directory
#        (packages/flutter/qvp_flutter by default).
# Output: ios/qvp_flutter/Package.swift and macos/qvp_flutter/Package.swift, edited in place.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
python3 - "$root/Package.swift" "${1:-$root/packages/flutter/qvp_flutter}" <<'PY'
import pathlib
import re
import sys

text = pathlib.Path(sys.argv[1]).read_text()
url = re.findall(r'url: "(https://[^"]+/QvpEngine\.xcframework\.zip)"', text)
checksum = re.findall(r'checksum: "([0-9a-f]{64})"', text)
if len(url) != 1 or len(checksum) != 1:
    raise SystemExit("error: the root Package.swift does not name one XCFramework")
local = '.binaryTarget(name: "QvpEngine", path: "QvpEngine.xcframework")'
remote = f'.binaryTarget(name: "QvpEngine", url: "{url[0]}", checksum: "{checksum[0]}")'
for platform in ("ios", "macos"):
    path = pathlib.Path(sys.argv[2], platform, "qvp_flutter", "Package.swift")
    text = path.read_text()
    if text.count(local) != 1:
        raise SystemExit(f"error: {path} does not name one local QvpEngine.xcframework")
    path.write_text(text.replace(local, remote))
PY
