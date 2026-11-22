#!/usr/bin/env bash
# Check that a built iOS or macOS app exports the engine's C symbols.
#
# Usage: scripts/check-darwin-engine-symbols.sh path/to/App.app
# Input: an .app bundle from `flutter build macos` or `flutter build ios`.
# Output: the binary that exports the engine; exit 1 when no binary in the bundle does.
#
# The Flutter plugin reaches the engine with DynamicLibrary.process(), and no native code
# calls the engine. A build can succeed while the linker drops or strips the engine
# symbols, and the app then fails at its first engine call. qvp_page_load is one of the
# symbols nothing native refers to, so the check looks for it.
# Needs: nm and file (Xcode command line tools). Runs under bash 3.2.
set -euo pipefail

app="${1:?usage: scripts/check-darwin-engine-symbols.sh path/to/App.app}"
[ -d "$app" ] || { echo "error: $app is not a directory" >&2; exit 1; }

found=""
while IFS= read -r f; do
  # grep without -q reads all of its input. With -q it can exit first, nm then dies of
  # SIGPIPE, and pipefail turns a match into a failure.
  file -b "$f" | grep 'Mach-O' >/dev/null || continue
  if nm -gU "$f" 2>/dev/null | grep -E ' T _qvp_page_load$' >/dev/null; then
    found="$f"
    break
  fi
done < <(find "$app" -type f)

if [ -z "$found" ]; then
  echo "error: no binary in $app exports _qvp_page_load; Dart cannot reach the engine" >&2
  exit 1
fi
echo "engine symbols exported by ${found#"$app"/}"
