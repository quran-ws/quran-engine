# The showcase

One page, in Arabic and English, that shows what the engine can do. Each scene is a live
mushaf page that the engine draws: tajwid colours, a recitation player, memorisation by
voice, zoom with reflow, ornaments, a comparison with other ways to show a page, and the
platforms.

## Run it

```sh
cargo build -p qvp-ffi --profile dist --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/dist/qvp_ffi.wasm web/
python3 web/demo/build.py && (cd dist/demo && python3 -m http.server 8771)
```

Open http://localhost:8771/. Use `localhost`, not `127.0.0.1`: a development key of Quran
Lab works only there.

## What the build takes

The repository holds no data, no font and no key. `build.py` reads these variables:

| variable | what it names | without it |
|---|---|---|
| `QVP_PAGES_DIR`, `QVP_LETTERS_DIR` | pages and letter layers from the same build (`docs/DATA.md`) | `dist/pages`, `dist/letters` |
| `QVP_ORNAMENTS` | an ornament set (`.qvo`) | the set beside the pages, which has ayah markers only |
| `QVP_FONT_TITLE`, `QVP_FONT_QURAN` | two `.woff2` fonts: the titles, and a word of the Quran outside the page | web fonts |
| `QVP_FONT_QPC`, `QVP_SVG_DIR` | QPC page fonts and page SVGs, for the comparison | those two rows show no measure |
| `QVP_TAJWID` | a file of tajwid rulings | `quran-ws/quran-tajweed` at a fixed commit |
| `QVP_DEMO_CONFIG`, `QVP_DEMO_OUT` | a config file and an output directory, for a build to share | `config.local.js`, `dist/demo` |

At run time the page reads tajwid rulings from a public CDN, audio and timings from the
MP3Quran API, and the text of the Quran from `text.quran.ws`.

## The reader's voice

The memorisation scene listens with TajweedFormer V4 of Quran Lab. The model runs on the
reader's device, in a frame that Quran Lab serves.

1. Make a web key at https://sdk.quranlab.ai.
2. Copy `config.example.js` to `config.local.js` and put the key in the copy.
3. Build again.

A web key is public by design: it works only on the origins that you list for it. So give
each address its own key, and do not allow `localhost` on a key that a public page uses.
Git ignores `config.local.js`. Do not commit a key or any file of the model.

The model gives the sounds that it heard and does not judge. `demo.js` does the rest, and
it is simple: it finds the one place in the Quran that matches what it heard, opens that
page, shows each word that the reader says, and marks a word that did not come. It can be
wrong, and it does not replace a qualified teacher. The model's licence (NPL-1.2) does not
let anyone charge for a feature that it powers.

## Languages

The page is written in Arabic. `en.js` maps each Arabic string to its English one, and
`?lang=en` selects it. When you add or change a string, add its English to `en.js`: a
string with no English stays Arabic and shows as a warning in the console.
