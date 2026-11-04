#!/usr/bin/env bash
# Set the XCFramework release URL, its SwiftPM checksum and the commit it was built from in
# the root package manifest. .github/workflows/prepare-ios-release.yml runs it after it
# builds the XCFramework. release.yml reads all three values back from the tagged commit.
#
# Usage: scripts/set-xcframework.sh vX.Y.Z <sha256> <commit> [manifest]
# Input: the tag, the 64-digit hex checksum, the 40-digit hex commit, and the manifest
#        (the root Package.swift by default).
# Output: the manifest, edited in place.
set -euo pipefail

usage="usage: scripts/set-xcframework.sh vX.Y.Z <sha256> <commit> [manifest]"
tag="${1:?$usage}"
checksum="${2:?$usage}"
commit="${3:?$usage}"
manifest="${4:-$(dirname "$0")/../Package.swift}"

python3 - "$tag" "$checksum" "$commit" "$manifest" <<'PY'
import pathlib
import re
import sys

tag, checksum, commit, manifest = sys.argv[1:]
if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
    raise SystemExit(f"error: '{tag}' is not vX.Y.Z")
if not re.fullmatch(r"[0-9a-f]{64}", checksum):
    raise SystemExit(f"error: '{checksum}' is not a SHA-256 checksum in hex")
if not re.fullmatch(r"[0-9a-f]{40}", commit):
    raise SystemExit(f"error: '{commit}' is not a full commit hash")

path = pathlib.Path(manifest)
text = path.read_text()
text, urls = re.subn(
    r"(releases/download/)v[^/]+(/QvpEngine\.xcframework\.zip)",
    rf"\g<1>{tag}\g<2>",
    text,
)
text, checksums = re.subn(r'(checksum: ")[0-9a-f]{64}(")', rf"\g<1>{checksum}\g<2>", text)
if urls != 1 or checksums != 1:
    raise SystemExit(f"error: {manifest} does not contain one release URL and one checksum")

# The comment above the binary target records the commit. Replace it, or add it when the
# manifest does not have it yet.
note = f"// Built by .github/workflows/prepare-ios-release.yml from commit {commit}."
text, notes = re.subn(
    r"// Built by \.github/workflows/prepare-ios-release\.yml from commit [0-9a-f]{40}\.",
    note,
    text,
)
if notes == 0:
    text, notes = re.subn(
        r"^( *)(\.binaryTarget\()$", rf"\g<1>{note}\n\g<1>\g<2>", text, flags=re.M
    )
if notes != 1:
    raise SystemExit(f"error: {manifest} does not contain one binary target")
path.write_text(text)
PY
