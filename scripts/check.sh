#!/usr/bin/env bash
# Run the checks CI runs, with the same commands. No argument runs all of them.
#
# Usage: scripts/check.sh [fmt|lint|test|gates|parity|versions|terminology|docs|structure|web]...
#
# Inputs: the repository; `gates` needs the data from scripts/sync-test-data.sh.
# Output: exit 0 when every selected check passes; the first failing check's output otherwise.
set -euo pipefail
cd "$(dirname "$0")/.."

ALL=(fmt lint test gates parity versions terminology docs structure web)
[ $# -eq 0 ] && set -- "${ALL[@]}"

step() { printf '\n== %s\n' "$1"; }

for check in "$@"; do
  case "$check" in
    fmt)
      step "rustfmt"
      cargo fmt --all -- --check
      ;;
    lint)
      step "clippy, warnings are errors"
      cargo clippy --workspace --release --all-targets -- -D warnings
      ;;
    test)
      step "workspace tests (data tests skip without data)"
      cargo test --workspace --release
      ;;
    gates)
      step "data gates: identity (8 pages), line shift, zoom control, ABI, conformance, zoom steps"
      QVP_REQUIRE_DATA=1 cargo test -p qvp-convert --release --test identity
      QVP_REQUIRE_DATA=1 cargo test -p qvp-core --release --test line_shift
      QVP_REQUIRE_DATA=1 cargo test -p qvp-core --release --test zoom_control
      QVP_REQUIRE_DATA=1 cargo test -p qvp-ffi --release --test abi
      node web/data.test.mjs
      node web/lite.test.mjs
      QVP_REQUIRE_DATA=1 node web/lite-passage.test.mjs
      scripts/gen-conformance.sh --check
      cargo run -p qvp-convert --release -- zoom-levels dist/pages crates/qvp-core/src/zoom_table.rs --check
      cargo build -p qvp-ffi --release --target wasm32-unknown-unknown
      cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/qvp_ffi.wasm
      node web/scenarios.test.mjs
      ;;
    parity)
      step "header vs Rust vs every wrapper"
      python3 scripts/check-parity.py
      ;;
    versions)
      step "one version everywhere"
      scripts/check-versions.sh
      ;;
    terminology)
      step "Quran.ws terminology audit"
      scripts/check-terminology.sh
      ;;
    docs)
      step "rustdoc with missing docs as errors, prose lint on changed documents"
      RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --release
      ;;
    structure)
      step "repository structure"
      scripts/check-structure.sh
      ;;
    web)
      step "wasm engine and the web tests"
      cargo build -p qvp-ffi --release --target wasm32-unknown-unknown
      cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/qvp_ffi.wasm
      node web/smoke.mjs
      node web/data.test.mjs
      node web/lite.test.mjs
      node web/lite-passage.test.mjs
      # The example is plain JavaScript that no test loads, so parse it here. This is a
      # syntax check, not a typecheck; it catches the file being left unrunnable.
      node --check web/example/app.js
      echo "ok  web/example/app.js parses"
      ;;
    *)
      echo "unknown check: $check (one of: ${ALL[*]})" >&2
      exit 2
      ;;
  esac
done

printf '\nall selected checks passed: %s\n' "$*"
