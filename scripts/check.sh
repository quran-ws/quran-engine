#!/usr/bin/env bash
# Run the checks CI runs, with the same commands. No argument runs all of them.
#
# Usage: scripts/check.sh [fmt|lint|test|gates|parity|versions|terminology|docs|structure|shell|web]...
#
# Inputs: the repository; `gates` needs the data from scripts/sync-test-data.sh.
# Output: exit 0 when every selected check passes; the first failing check's output otherwise.
set -euo pipefail
cd "$(dirname "$0")/.."

ALL=(fmt lint test gates parity versions terminology docs structure shell web)
[ $# -eq 0 ] && set -- "${ALL[@]}"

step() { printf '\n== %s\n' "$1"; }

# Build the wasm engine and copy it to web/ once per run. `gates` and `web` both need it.
wasm_built=""
wasm() {
  [ -n "$wasm_built" ] && return 0
  cargo build -p qvp-ffi --profile dist --target wasm32-unknown-unknown
  cp target/wasm32-unknown-unknown/dist/qvp_ffi.wasm web/qvp_ffi.wasm
  wasm_built=1
}

# True when this run also runs `gates`, which runs the mixed web tests with their data.
runs_gates() { case " $* " in *" gates "*) return 0 ;; *) return 1 ;; esac; }

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
      step "data gates: identity (8 pages), line shift, zoom control, passages, ABI, conformance, zoom steps, web replays"
      QVP_REQUIRE_DATA=1 cargo test -p qvp-convert --release --test identity
      QVP_REQUIRE_DATA=1 cargo test -p qvp-core --release --test line_shift
      QVP_REQUIRE_DATA=1 cargo test -p qvp-core --release --test zoom_control
      QVP_REQUIRE_DATA=1 cargo test -p qvp-core --release --test passage
      QVP_REQUIRE_DATA=1 cargo test -p qvp-core --release --test malformed
      QVP_REQUIRE_DATA=1 cargo test -p qvp-ffi --release --test abi
      scripts/gen-conformance.sh --check
      cargo run -p qvp-convert --release -- zoom-levels dist/pages crates/qvp-core/src/zoom_table.rs --check
      # The web tests that read pages. Each one fails here when its data is missing.
      wasm
      QVP_REQUIRE_DATA=1 node web/smoke.mjs
      QVP_REQUIRE_DATA=1 node web/lite-passage.test.mjs
      QVP_REQUIRE_DATA=1 node web/scenarios.test.mjs
      QVP_REQUIRE_DATA=1 node web/passage.test.mjs
      ;;
    parity)
      step "header vs Rust vs every wrapper"
      # --check compares docs/API-PARITY.md with the sources and writes nothing. After an
      # API change, regenerate the table with `python3 scripts/check-parity.py`.
      python3 scripts/check-parity.py --check
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
      step "rustdoc with missing docs as errors, then the prose report"
      RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --release
      # The prose report, for a writer to read. The counts are a guide, so this check
      # fails only when rustdoc or the linter's own tests fail. CONTRIBUTING.md holds
      # the rules.
      python3 scripts/test_ste_lint.py
      documents="$(git ls-files '*.md' | grep -v node_modules || true)"
      if [ -n "$documents" ]; then
        printf '%s\n' "$documents" | tr '\n' '\0' |
          xargs -0 python3 scripts/ste_lint.py
      fi
      ;;
    structure)
      step "repository structure"
      scripts/check-structure.sh
      ;;
    shell)
      step "shellcheck over every shell script"
      if ! command -v shellcheck >/dev/null; then
        echo "skip shellcheck: not installed (brew install shellcheck); CI runs it"
      else
        # Every tracked *.sh file, and every tracked file without an extension whose
        # first line is a bash shebang.
        scripts=()
        while IFS= read -r f; do scripts+=("$f"); done < <(git ls-files '*.sh')
        while IFS= read -r f; do
          if [ -f "$f" ] && head -n 1 "$f" | grep -q '^#!.*bash'; then scripts+=("$f"); fi
        done < <(git ls-files | grep -v '\.[^/]*$')
        # bash 3.2 treats an empty array as unbound under set -u, so test the length first.
        if [ "${#scripts[@]}" -eq 0 ]; then
          echo "ok   shellcheck: 0 scripts"
        else
          shellcheck "${scripts[@]}"
          echo "ok   shellcheck: ${#scripts[@]} scripts, no findings"
        fi
      fi
      ;;
    web)
      step "wasm engine and the web tests without data"
      wasm
      if runs_gates "$@"; then
        echo "skip smoke.mjs and lite-passage.test.mjs here: gates runs them with data in this run"
      else
        node web/smoke.mjs
        node web/lite-passage.test.mjs
      fi
      node web/lite.test.mjs
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
