#!/usr/bin/env bash
# Fetch the pages used by Android instrumentation tests. Page data is not part of the SDK.
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${QVP_DATA_TAG:-v0.3.0}"
destination="packages/android/qvp/src/androidTest/assets"
mkdir -p "$destination"

if [ -n "${QVP_PAGES:-}" ] && [ -f "$QVP_PAGES/001.qvp" ] && [ -f "$QVP_PAGES/042.qvp" ]; then
  cp "$QVP_PAGES/001.qvp" "$QVP_PAGES/042.qvp" "$destination/"
elif [ -f "dist/pages/001.qvp" ] && [ -f "dist/pages/042.qvp" ]; then
  cp "dist/pages/001.qvp" "dist/pages/042.qvp" "$destination/"
else
  archive="$(mktemp -t qvp-pages.XXXXXX)"
  trap 'rm -f "$archive"' EXIT
  curl --fail --location --retry 3 \
    "https://github.com/quran-ws/quran-engine/releases/download/$tag/quran-engine-pages-hafs-kfgqpc.tar.gz" \
    --output "$archive"
  tar -xOf "$archive" "quran-engine-pages-hafs-kfgqpc/001.qvp" > "$destination/001.qvp"
  tar -xOf "$archive" "quran-engine-pages-hafs-kfgqpc/042.qvp" > "$destination/042.qvp"
fi

python3 - "$destination" "$tag" <<'PY'
import hashlib
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
expected = {
    "001.qvp": "2792ceab5f3cfaa0d86d09a8ca11c80e49b2692de384705b57225c9e3899940a",
    "042.qvp": "0fc141ceb6b3f555a2590332bfe153a17ec185a76f81be05cdfc0b0d4bccadb2",
}
# 001 and 042 are byte-identical across these releases
if sys.argv[2] in ("v0.3.0", "data-v0.2.0"):
    for name, digest in expected.items():
        actual = hashlib.sha256((root / name).read_bytes()).hexdigest()
        if actual != digest:
            raise SystemExit(f"{name} checksum mismatch: {actual}")
PY
echo "Android test pages ready: $destination/{001,042}.qvp"
