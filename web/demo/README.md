# The Arabic showcase

One Arabic page that shows what the engine can do. It has six scenes, and each scene is a
live mushaf page that the engine draws through `web/qvp.js`.

| scene | what it shows | engine calls |
|---|---|---|
| البداية | the page appears word by word, and a tap names a word and its letters | `mask`, `unmaskNext`, `hitTestView`, `letters` |
| التجويد | tajwid rulings coloured on their own letters, a legend, and a tap that names the ruling | `style(Sel.wordChars(...))`, `hitTestLetterView` |
| التلاوة | a player whose highlight follows the reciter by word or by letter | `highlight`, `moveHighlight`, `style(Sel.letter(...))`, `letterAtOffset` |
| الحفظ | hide the words, then reveal them by a tap or by the reader's voice | `mask`, `unmaskNext`, `maskBack`, `unmaskWord` |
| القراءة | zoom steps with reflow, pinch modes and paper themes | `zoomToStep`, `zoomPinch`, `zoomMode`, `setDefaultColor` |
| الزخرفة | the same text in the ornaments of another mushaf | `dress`, `undress` |
| المقارنة | the engine against SVG, images, page fonts and the Hafs font with text: size, features, and a measure on the reader's device | `loadPage`, `buildPaths`, `CanvasRenderer.draw` |

Every page takes a pinch, Ctrl with the mouse wheel, and the zoom buttons on its frame.

## Run it

```sh
cargo build -p qvp-ffi --profile dist --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/dist/qvp_ffi.wasm web/
python3 web/demo/build.py && (cd dist/demo && python3 -m http.server 8771)
```

Open http://localhost:8771/. Use `localhost`, not `127.0.0.1`: a development key of Quran
Lab works only there.

## Data

The showcase commits no data. It reads these at run time:

- **Pages and letter layers** from `dist/pages/` and `dist/letters/`, which `build.py` links.
  The tajwid scene and the letter highlight need the letter layers, and a layer attaches only
  to the page file it was built with (`docs/DATA.md`). `QVP_PAGES_DIR` and `QVP_LETTERS_DIR`
  name other directories.
- **Tajwid rulings** from `@quran.ws/tajwid-annotations` and `@quran.ws/tajwid-rules`
  (`quran-ws/quran-tajweed`, CC BY 4.0), through a public CDN. The colours are the colour
  classes of quranpedia.net. The showcase reads the repository at one commit, because the
  npm packages, 0.1.0, are older: a span of theirs covers a letter and its condition, so a
  qalqalah or a madd colours the next letter too. A span of the commit adds a focus, the
  letters that the ruling is about, and the scene colours the focus alone. `QVP_TAJWID`
  names another file of rulings.
- **Audio and timings** from the MP3Quran API, version 4. It needs no key. The showcase uses
  the staging API until the API has a public address.
- **Ornaments** from `ornaments.qvo` beside the pages. The released set has ayah markers
  only, so the scene then changes the markers alone. `QVP_ORNAMENTS` names another set.

## Fonts

The titles and the words of the Quran beside the page can use two fonts of your machine.
The repository does not hold them, because their licences do not allow that:

```sh
QVP_FONT_TITLE=/path/to/thmanyahserifdisplay-Bold.woff2 \
QVP_FONT_QURAN=/path/to/hafs.woff2 \
python3 web/demo/build.py
```

`QVP_FONT_TITLE` is the typeface of the titles, and the showcase is designed with Thmanyah
Serif Display. `QVP_FONT_QURAN` is the typeface for a word of the Quran outside the page:
the KFGQPC Hafs font, so the word looks as it does in the mushaf. Without a file, the page
uses a web font.

`QVP_FONT_QPC` names a directory of QPC page fonts (`p3.woff2` to `p52.woff2`). The
comparison measures how long those fonts and the Hafs font take to show 1, 10 or 50 pages.
The build needs fontTools to list each font's code points.

## The reader's voice

The memorisation scene can listen to the reader with TajweedFormer V4 of Quran Lab. The
model runs on the reader's device, in a frame that Quran Lab serves. The showcase loads
Quran Lab's script at run time and holds none of its files.

1. Get a web key from https://sdk.quranlab.ai.
2. Copy `config.example.js` to `config.local.js` and put the key in the copy.
3. Build again.

`config.local.js` is ignored by git. Do not commit a key, and do not add any file of the
model to this repository.

The model gives the sounds that it heard. It does not judge a recitation. The match in
`demo.js` is the showcase's own and it is simple: a word shows when most of its strong
consonants came, in order. It can be wrong, and it does not replace a qualified teacher.
The licence of the model (NPL-1.2) does not let anyone charge for the model or for a
feature that it powers.
