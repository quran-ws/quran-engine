#!/usr/bin/env bash
# Regenerate the cross-wrapper scenarios from the Rust engine (docs/standards/TESTING.md).
# Input: dist/pages/042.qvp (scripts/sync-test-data.sh). Output: conformance/scenarios/*.json.
# `--check` exits 1 when the committed files differ from what the engine produces now.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f dist/pages/042.qvp ] || { echo "error: dist/pages/042.qvp is missing; run scripts/sync-test-data.sh" >&2; exit 1; }
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/scenarios"
cargo run -q -p qvp-core --release --example scenarios -- dist/pages/042.qvp "$tmp/scenarios/layout.json"
cargo run -q -p qvp-core --release --example lite_passage_metrics -- dist/pages > "$tmp/lite-passage-metrics.json"
for file in scenarios/layout.json lite-passage-metrics.json; do
  if [ "${1:-}" = "--check" ]; then
    if cmp -s "$tmp/$file" "conformance/$file"; then
      echo "ok   conformance/$file matches the engine"
    else
      echo "FAIL conformance/$file is stale; run scripts/gen-conformance.sh" >&2
      diff "$tmp/$file" "conformance/$file" | head -20 >&2 || true
      exit 1
    fi
  else
    mkdir -p conformance/scenarios
    cp "$tmp/$file" "conformance/$file"
    echo "wrote conformance/$file"
  fi
done
