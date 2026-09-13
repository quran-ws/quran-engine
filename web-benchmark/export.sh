#!/usr/bin/env bash
set -euo pipefail

benchmark_dir="$(cd "$(dirname "$0")" && pwd)"
repository_dir="$(cd "$benchmark_dir/.." && pwd)"
pages_dir="${QVP_PAGES_DIR:-$repository_dir/dist/pages}"
output="${1:-$benchmark_dir/data}"

if (( $# )); then shift; fi
if [[ "$output" != /* ]]; then output="$(pwd)/$output"; fi

mkdir -p "$output" "$benchmark_dir/.build"
cargo build --manifest-path "$repository_dir/Cargo.toml" -p qvp-ffi --release
clang -O2 \
  -I "$repository_dir/crates/qvp-ffi/include" \
  "$benchmark_dir/export.c" \
  "$repository_dir/target/release/libqvp_ffi.a" \
  -framework Security \
  -framework CoreFoundation \
  -o "$benchmark_dir/.build/export"
"$benchmark_dir/.build/export" "$pages_dir" "$output" "$@"
