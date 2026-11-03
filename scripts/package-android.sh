#!/usr/bin/env bash
# Build a release AAR containing the JNI bridge and Rust engine for every supported ABI.
set -euo pipefail
cd "$(dirname "$0")/.."

# The workspace version is the default, as the build file's own default is.
version="${1:-$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)}"
scripts/build-engine-android.sh
(cd packages/android && ./gradlew --no-daemon :qvp:clean :qvp:assembleRelease -PqvpVersion="$version")

out="dist/android"
mkdir -p "$out"
aar="$out/qvp-android-$version.aar"
cp packages/android/qvp/build/outputs/aar/qvp-release.aar "$aar"
scripts/check-android-page-size.sh "$aar"
python3 - "$aar" > "$aar.sha256" <<'PY'
import hashlib
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
print(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
PY
echo "Android SDK packaged: $aar"
