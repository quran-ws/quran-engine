#!/usr/bin/env bash
# Check the repository against docs/standards/STRUCTURE.md.
# Input: the tracked files. Output: one line per rule; exit 1 if any rule fails.
set -euo pipefail
cd "$(dirname "$0")/.."
status=0
fail() { echo "FAIL $1"; status=1; }
ok() { echo "ok   $1"; }

# 1. The root holds only the allowed files and directories.
allowed="README.md LICENSE LICENSES CONTRIBUTING.md SECURITY.md CODE_OF_CONDUCT.md CHANGELOG.md AGENTS.md CLAUDE.md Cargo.toml Cargo.lock Package.swift package.json rust-toolchain.toml rustfmt.toml .editorconfig .gitignore .mailmap .terminology.json .github crates packages web docs scripts conformance"
extra=""
for entry in $(git ls-files | cut -d/ -f1 | sort -u); do
  case " $allowed " in *" $entry "*) ;; *) extra="$extra $entry" ;; esac
done
if [ -z "$extra" ]; then ok "root holds only allowed entries"; else fail "root has unexpected entries:$extra"; fi

# 2. No generated file is tracked.
generated="$(git ls-files | grep -E 'packages/ios/.*\.xcodeproj/|^packages/ios/example/Info\.plist$|/jniLibs/|/prebuilt/|packages/flutter/qvp_flutter/assets/(wasm/qvp_ffi\.wasm|js/qvp\.js)$|^pages/|^index/|^dist/|\.DS_Store$|^\.maestro/|\.qvo$' || true)"
if [ -z "$generated" ]; then ok "no generated or local file is tracked"; else fail "tracked generated files: $generated"; fi

# 3. Every package directory has README, CHANGELOG and LICENSE.
for pkg in packages/android/qvp packages/flutter/qvp_flutter packages/ios/QvpKit packages/react-native/qvp-react-native; do
  missing=""
  for f in README.md CHANGELOG.md LICENSE; do [ -f "$pkg/$f" ] || missing="$missing $f"; done
  if [ -z "$missing" ]; then ok "$pkg has README, CHANGELOG, LICENSE"; else fail "$pkg is missing:$missing"; fi
done

# 4. CLAUDE.md is the AGENTS.md import plus at most fifteen lines.
if [ "$(head -1 CLAUDE.md)" = "@AGENTS.md" ] && [ "$(wc -l < CLAUDE.md)" -le 16 ]; then
  ok "CLAUDE.md is the import plus a short Claude-only section"
else
  fail "CLAUDE.md must start with @AGENTS.md and stay under sixteen lines"
fi

# 5. Every relative link in every AGENTS.md resolves.
for f in $(git ls-files | grep -E '(^|/)AGENTS\.md$'); do
  dir="$(dirname "$f")"
  broken=""
  # shellcheck disable=SC2016  # the backticks are the Markdown code span, not a command
  for target in $(grep -oE '`[A-Za-z0-9_./-]+\.(md|sh|py|h|json)`' "$f" | tr -d '`' | sort -u); do
    [ -e "$target" ] || [ -e "$dir/$target" ] || broken="$broken $target"
  done
  if [ -z "$broken" ]; then ok "$f: every named file exists"; else fail "$f names missing files:$broken"; fi
done

exit $status
