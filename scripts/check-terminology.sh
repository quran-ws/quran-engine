#!/usr/bin/env bash
# Run the Quran.ws terminology audit over this repository with .terminology.json as config.
#
# The audit script lives in the quran-ws/guidelines repository (the `quranic-terminology`
# skill). This script clones that repository at a pinned commit into target/ and runs the
# audit from there, so a developer and CI audit with the same version.
# Environment: QVP_GUIDELINES_REF (default: the pinned commit below);
#              QVP_TERMINOLOGY_SCRIPT, a path to audit_terminology.py to run instead of
#              the pinned clone, for work on the audit itself;
#              QVP_TERMINOLOGY_BASELINE, the number of errors tolerated (0: any finding fails).
# Output: the audit's report; exit 1 when the error count exceeds the baseline.
set -euo pipefail
cd "$(dirname "$0")/.."

ref="${QVP_GUIDELINES_REF:-79e3c6bb2b52dd1df81b3e64debfebf9dd5f8ea5}"
clone="target/guidelines"

if [ -n "${QVP_TERMINOLOGY_SCRIPT:-}" ]; then
  script="$QVP_TERMINOLOGY_SCRIPT"
else
  if [ ! -d "$clone/.git" ]; then
    git clone --quiet --depth 1 https://github.com/quran-ws/guidelines.git "$clone"
  fi
  (cd "$clone" && git fetch --quiet --depth 1 origin "$ref" && git checkout --quiet FETCH_HEAD)
  script="$clone/skills/quranic-terminology/scripts/audit_terminology.py"
fi

[ -f "$script" ] || { echo "error: audit_terminology.py not found at $script" >&2; exit 1; }

baseline="${QVP_TERMINOLOGY_BASELINE:-0}"
report="$(python3 "$script" . 2>&1)" || true
printf '%s\n' "$report"
errors="$(printf '%s\n' "$report" | sed -n 's/^\([0-9][0-9]*\) errors, .*/\1/p' | tail -1)"
[ -n "$errors" ] || { echo "error: could not read the error count from the audit report" >&2; exit 1; }
if [ "$errors" -gt "$baseline" ]; then
  echo "FAIL terminology: $errors errors; every name follows the Quran.ws standard (docs/standards/NAMING.md)"
  exit 1
fi
echo "ok   terminology: $errors errors, baseline $baseline"
