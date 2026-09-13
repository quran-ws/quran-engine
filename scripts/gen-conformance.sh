#!/usr/bin/env bash
# Regenerate the cross-wrapper scenarios from the Rust engine (docs/standards/TESTING.md).
# Input: dist/pages/042.qvp (scripts/sync-test-data.sh). Output: conformance/scenarios/*.json.
# `--check` exits 1 when the committed files differ from what the engine produces now.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f dist/pages/042.qvp ] || { echo "error: dist/pages/042.qvp is missing; run scripts/sync-test-data.sh" >&2; exit 1; }
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
cargo run -q -p qvp-core --release --example scenarios -- dist/pages/042.qvp "$tmp/layout.json"
if [ "${1:-}" = "--check" ]; then
  if cmp -s "$tmp/layout.json" conformance/scenarios/layout.json; then
    echo "ok   conformance/scenarios/layout.json matches the engine"
  else
    echo "FAIL conformance/scenarios/layout.json is stale; run scripts/gen-conformance.sh" >&2
    diff "$tmp/layout.json" conformance/scenarios/layout.json | head -20 >&2 || true
    exit 1
  fi
else
  mkdir -p conformance/scenarios
  cp "$tmp/layout.json" conformance/scenarios/layout.json
  echo "wrote conformance/scenarios/layout.json"
fi
