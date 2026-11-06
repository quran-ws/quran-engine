# Web example

A Canvas2D reader built on `web/qvp.js`, the reference wrapper. It implements
`docs/EXAMPLE-APP.md`.

```sh
cargo build -p qvp-ffi --profile dist --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/dist/qvp_ffi.wasm web/
scripts/sync-test-data.sh                                   # dist/pages
python3 web/example/build.py dev && (cd dist/web && python3 -m http.server 8765)
```

Open http://127.0.0.1:8765/?p=42. `build.py embed 1-21,440-445,582,604` writes a single
`dist/demo.html` with the engine and those pages inlined.

The three calls the app is built on, in `app.js`:

```js
const page = engine.loadPage(bytes);               // load
draw(page);                                         // render over page.paths
const hit = page.hitTestView(x, y);               // tap: the word under the finger
```

## The UI smoke test

`smoke.test.mjs` opens page 440 in Chromium, clicks a word of surah 36, and checks the
highlight three ways: the app's state through `window.__qvp`, the engine's highlight boxes,
and the selection colour in the canvas pixels. It serves `dist/web` itself.

```sh
python3 web/example/build.py dev
cd web/example && npm ci && npx playwright install chromium && npm test
```

`scripts/check.sh example` builds the engine and the example, then runs the test. It skips
when Playwright is not installed.
