# Web example

A Canvas2D reader built on `web/qvp.js`, the reference wrapper. It implements
`docs/EXAMPLE-APP.md`.

```sh
cargo build -p qvp-ffi --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/
scripts/sync-test-data.sh                                   # dist/pages
python3 web/example/build.py dev && (cd dist/web && python3 -m http.server 8765)
```

Open http://127.0.0.1:8765/?p=42. `build.py embed 1-21,440-445,582,604` writes a single
`dist/demo.html` with the engine and those pages inlined.

The three calls the app is built on, in `app.js`:

```js
const page = engine.loadPage(bytes);               // load
draw(page);                                         // render over page.paths
const hit = page.hitTestViewEx(x, y);               // tap: the word under the finger
```
