#!/usr/bin/env bash
# Fetch the pages and the scenario file the Android instrumentation tests read. Page data is
# not part of the SDK. Input: QVP_PAGES, dist/pages or the data release named by QVP_DATA_TAG.
# Output: packages/android/qvp/src/androidTest/assets/{NNN.qvp, passage.json}.
set -euo pipefail
cd "$(dirname "$0")/.."

tag="${QVP_DATA_TAG:-data-v0.5.0}"
destination="packages/android/qvp/src/androidTest/assets"
mkdir -p "$destination"

# 001 and 042 carry the page tests. The rest are what conformance/scenarios/passage.json loads,
# read from the file so a new scenario on a new page is picked up here.
mapfile -t pages < <(python3 - <<'PY'
import json
pages = {1, 42}
for case in json.load(open("conformance/scenarios/passage.json"))["cases"]:
    pages.update(case["pages"])
for p in sorted(pages):
    print(f"{p:03d}")
PY
)

have_all() {
  for p in "${pages[@]}"; do [ -f "$1/$p.qvp" ] || return 1; done
}

if [ -n "${QVP_PAGES:-}" ] && have_all "$QVP_PAGES"; then
  for p in "${pages[@]}"; do cp "$QVP_PAGES/$p.qvp" "$destination/"; done
elif have_all dist/pages; then
  for p in "${pages[@]}"; do cp "dist/pages/$p.qvp" "$destination/"; done
else
  archive="$(mktemp -t qvp-pages.XXXXXX)"
  trap 'rm -f "$archive"' EXIT
  curl --fail --location --retry 3 \
    "https://github.com/quran-ws/quran-engine/releases/download/$tag/quran-engine-pages-hafs-kfgqpc.tar.gz" \
    --output "$archive"
  for p in "${pages[@]}"; do
    tar -xOf "$archive" "quran-engine-pages-hafs-kfgqpc/$p.qvp" > "$destination/$p.qvp"
  done
fi
cp conformance/scenarios/passage.json "$destination/passage.json"

python3 - "$destination" "$tag" <<'PY'
import hashlib
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
expected = {
    "001.qvp": "2792ceab5f3cfaa0d86d09a8ca11c80e49b2692de384705b57225c9e3899940a",
    "002.qvp": "2d6b34293a3884739001de8d89b9ae2a89779095992d1ba5cc2031687d142e08",
    "003.qvp": "c3a9db527d962203b993097ac307ae123d2af524edf2af7a16b6771139eafca5",
    "042.qvp": "0fc141ceb6b3f555a2590332bfe153a17ec185a76f81be05cdfc0b0d4bccadb2",
    "048.qvp": "a70705c9dd50df427dbe5c194b85d9d62f0da5a6c1e75b69226c81caaa1db6c2",
    "272.qvp": "52eb2585f4f888c7b0d91ceb97bdb98602d588bf60e260320e47dfaf63ad0860",
    "598.qvp": "e381d05d13f3e374f4148d8c5a5b6a610e3b9f2576e9cb255191a2af0cfe7fdf",
    "604.qvp": "3c4cab0889bd6a850a403b1e41a58f0e2546f0da41376880f6a35229cd053ec3",
}
# The digests are those of data-v0.4.0 and data-v0.5.0, whose pages are byte-identical.
# 001 and 042 are byte-identical in the two releases before them.
# Nobody checked the other pages there.
# A page the table does not list is copied unchecked.
checked = {
    "data-v0.5.0": set(expected),
    "data-v0.4.0": set(expected),
    "data-v0.3.0": {"001.qvp", "042.qvp"},
    "data-v0.2.0": {"001.qvp", "042.qvp"},
}.get(sys.argv[2], set())
for name in sorted(checked):
    if not (root / name).exists():
        continue
    actual = hashlib.sha256((root / name).read_bytes()).hexdigest()
    if actual != expected[name]:
        raise SystemExit(f"{name} checksum mismatch: {actual}")
PY
echo "Android test data ready: $destination/{$(IFS=,; echo "${pages[*]}")}.qvp and passage.json"
