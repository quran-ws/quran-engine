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

## The screen

- **The page.** Tap, drag, pinch or scroll it. The switch on the page, "tap selects", sets
  what a tap takes: a word, a letter or a mark.
- **Calls.** The strip under the page shows each call that the app made to the engine, as
  source text. Click a line to copy it. "Reset page" removes every style, highlight, mask
  and selection.
- **The side panel.** One topic at a time: Inspect, Letters, Colour, Memorise, Layout and
  Dress. Each heading names the call behind its controls. "Engine stats" at the bottom opens
  the page's counts and timings.

## Letters

The letter controls need the letter layer of the page, `NNN.qvl` (`docs/DATA.md`).
`build.py` links `dist/letters/` into `dist/web/` when it is there, and the app attaches
`letters/NNN.qvl` when it opens a page. A layer attaches only to the page file it was built
with. To build the pages and the layers from the same source:

```sh
QVP_SYNC=letters scripts/sync-test-data.sh                                # pages/ and pages-letters/
cargo run -p qvp-convert --release -- batch pages dist/pages-local       # pages from that source
cargo run -p qvp-convert --release -- letters pages-letters pages dist/letters
QVP_PAGES_DIR=dist/pages-local python3 web/example/build.py dev
```

With no layer for a page, or one that the engine refuses, the letter controls are disabled
and the panel says "no letter layer for this page".

The three calls the app is built on, in `app.js`:

```js
const page = engine.loadPage(bytes);               // load
draw(page);                                         // render over page.paths
const hit = page.hitTestView(x, y);               // tap: the word under the finger
```

## The UI smoke test

`smoke.test.mjs` opens page 440 in Chromium, clicks a word of surah 36, and checks the
highlight three ways: the app's state through `window.__qvp`, the engine's highlight boxes,
and the selection colour in the canvas pixels. Then it checks the letter controls: with a
letter layer, a tap in letter mode adds one letter rule and "Clear letters" removes it.
Without one, the controls are disabled. It serves `dist/web` itself.

```sh
python3 web/example/build.py dev
cd web/example && npm ci && npx playwright install chromium && npm test
```

`scripts/check.sh example` builds the engine and the example, then runs the test. It skips
when Playwright is not installed.
