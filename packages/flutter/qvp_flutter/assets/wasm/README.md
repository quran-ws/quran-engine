# assets/wasm

`qvp_ffi.wasm` lands here: the WebAssembly build of `crates/qvp-ffi`, which the web backend
loads. The repository does not track it. `scripts/sync-flutter-web-engine.sh` builds it, and
the release workflow puts the build from the release tag in the pub.dev package.
