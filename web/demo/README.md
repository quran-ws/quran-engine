# The showcase

One page, in Arabic and English, that shows what the engine can do. Each scene is a live
mushaf page that the engine draws: tajwid colours, a recitation player, memorisation by
voice, zoom with reflow, ornaments, a comparison with other ways to show a page, and the
platforms.

It is live at https://engine.quran.ws/.

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
| `QVP_FONT_QURAN` | a `.woff2` font for a word of the Quran outside the page | a web font |
| `QVP_FONT_QPC`, `QVP_SVG_DIR` | QPC page fonts and page SVGs, for the comparison | those two rows show no measure |
| `QVP_TAJWID` | a file of tajwid rulings | `quran-ws/quran-tajweed` at a fixed commit |
| `QVP_DEMO_CONFIG`, `QVP_DEMO_OUT` | a config file and an output directory, for a build to share | `config.local.js`, `dist/demo` |

A build can hold some pages only. Then `pages/index.json` lists them, as `{"pages": [1, 3, 42]}`,
and each scene stays inside the list.

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

The model does not judge a recitation. `demo.js` does the rest:

1. **Find the place.** It searches the whole Quran for the sounds that were heard, and opens
   the page as soon as they are in one place only. The isti'adhah and the basmalah do not
   count, because they come before every recitation.
2. **Follow the words.** For each frame of the model's output it lines up the sounds that a
   correct recitation of the page has (`model.reference`). A word shows in the frame where
   its last sound is heard.
3. **Mark a mistake.** When six sounds come with no step forward, the word that was
   expected gets a sound and a red mark, and the reader can say it again.

The SDK asks each reader whether to share recordings of their recitation with Quran Lab,
and the choice is the reader's. A site that uses it must say so in its privacy policy.

It can be wrong, and it does not replace a qualified teacher. The model's licence (NPL-1.2)
does not let anyone charge for a feature that it powers.

## The site

`.github/workflows/showcase.yml` builds the showcase as an image and deploys it to
`engine.quran.ws`, on each push to `main` that changes the engine or the page. The job
converts all 604 pages and their letter layers from `quran-svg-elements`, builds the page,
and makes an nginx image from `web/demo/site/`. An image-bump pull request in the server's
repository then deploys it, as for the other sites of quran.ws.

The site's dress scene shows the ornaments of the Madinah mushafs. The job builds that
set from `quran-ws/quran-assets` without `--redistributable`, with the approval that the
site's owner has for this site. A data release does not hold these ornaments. Before you
show them on another site, read each ornament's licence, which the scene prints.

The site's key of Quran Lab is the repository secret `QURANLAB_WEB_KEY`. Its one allowed
origin is `https://engine.quran.ws`.

## Languages

The page is written in Arabic. `en.js` maps each Arabic string to its English one, and
`?lang=en` selects it. When you add or change a string, add its English to `en.js`: a
string with no English stays Arabic and shows as a warning in the console.
