#!/usr/bin/env bash
# Zip the XCFramework that scripts/build-engine-ios.sh wrote, for the SwiftPM binary target,
# and write its SwiftPM checksum (the SHA-256 of the zip, in hex).
#
# Usage: scripts/package-ios.sh
# Input: packages/ios/QvpKit/QvpEngine.xcframework. Run scripts/build-engine-ios.sh first.
# Output: dist/ios/QvpEngine.xcframework.zip and dist/ios/QvpEngine.xcframework.zip.checksum.
#
# The zip stores its entries in sorted order, with one fixed date and fixed modes, so the
# same XCFramework files give the same zip and the same checksum.
set -euo pipefail
cd "$(dirname "$0")/.."

xcframework=packages/ios/QvpKit/QvpEngine.xcframework
archive=dist/ios/QvpEngine.xcframework.zip
[ -d "$xcframework" ] || {
  echo "error: $xcframework is missing; run scripts/build-engine-ios.sh first" >&2
  exit 1
}
mkdir -p "$(dirname "$archive")"

python3 - "$xcframework" "$archive" <<'PY'
import os
import pathlib
import sys
import zipfile

source, archive = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
root = source.parent
date = (1980, 1, 1, 0, 0, 0)  # the earliest date a zip entry can hold


def entry(name, mode):
    info = zipfile.ZipInfo(name, date)
    info.create_system = 3  # Unix, so unzip restores the mode below
    info.external_attr = mode << 16
    return info


paths = [source] + sorted(source.rglob("*"), key=lambda p: p.relative_to(root).as_posix())
with zipfile.ZipFile(archive, "w") as out:
    for path in paths:
        name = path.relative_to(root).as_posix()
        if path.is_symlink():
            out.writestr(entry(name, 0o120777), os.readlink(path))
        elif path.is_dir():
            out.writestr(entry(name + "/", 0o40755), b"")
        else:
            info = entry(name, 0o100755 if os.access(path, os.X_OK) else 0o100644)
            info.compress_type = zipfile.ZIP_DEFLATED
            out.writestr(info, path.read_bytes(), compresslevel=9)
PY

python3 - "$archive" > "$archive.checksum" <<'PY'
import hashlib
import pathlib
import sys

print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest())
PY
unzip -tq "$archive" >/dev/null
echo "XCFramework packaged: $archive"
echo "SwiftPM checksum: $(cat "$archive.checksum")"
