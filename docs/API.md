# QVP engine API

One engine, one contract. The C ABI in `crates/qvp-ffi/include/qvp.h` is the source of
truth; every wrapper (`web/qvp.js`, Kotlin, Dart, React Native, Swift) exposes the same
names in the platform's own casing, so this page documents once and applies everywhere.
Examples are JavaScript; read `page.hitTestEx(...)` as `page.hitTestEx(...)` in Dart,
`page.hitTestEx(...)` in Kotlin, `qvp_hit_test_ex(...)` in C.

**Conventions**

- **Colours** are `0xRRGGBBAA`. Alpha 0 means *hidden* in a style and *leave alone* in a theme.
  Wrappers also accept `'#rgb'`, `'#rrggbb'`, `'#rrggbbaa'`.
- **Page units** are the printed page's viewBox space (345 × 550 for this mushaf, y down).
  Anything named `…View` is in *viewport pixels through the current layout*.
- **Handles.** Every mutating style/highlight call returns a handle; removing the handle
  undoes exactly that call and nothing else. There is a `clear`, but you never need it.
- **Targets** resolve to a word list: `'page'`, `'2:255'` (ayah), `'2:255:3'` (word),
  `'2:255-257'`, `'line:7'`, `'surah:2'`, a word index, an array of word indices, or a
  `T.*` constructor.
- **Selectors** say what a style rule applies to, from a whole page down to *the second
  diacritic of one word*: `Sel.page()`, `Sel.word(i)`, `Sel.ayah(s,a)`, `Sel.line(n)`,
  `Sel.wordBody(i)`, `Sel.wordMarks(i)`, `Sel.wordMark(i, nth)`, `Sel.wordMarkNamed(i, 'fathah', nth)`,
  `Sel.wordPath(i, nth)`, `Sel.path(p)`, `Sel.mark('shaddah')`, `Sel.category('harakah')`,
  `Sel.family('dots')`, `Sel.kind('mark')`, `Sel.deco('ayah-mark')`, `Sel.decoIdx(d)`.
- **The engine computes, the host renders.** Hit-testing, layout, styling, highlight bands,
  masks and search are engine calls. A wrapper marshals the calls and renders the results.
- **Data is separate from code.** Pages (`NNN.qvp`), the atlas (`atlas.qva`) and the
  optional text sidecars (`NNN.words.json`) are assets your app loads; no package bundles them.

## Loading

```js
const engine = await QvpEngine.init(wasmBytes);           // native: QvpEngine(library path)
const page   = engine.loadPage(await fetchBytes('pages/042.qvp'));
const atlas  = engine.loadAtlas(await fetchBytes('pages/atlas.qva'));   // optional
page.attachWords(await fetchJson('pages/042.words.json'));               // optional forms
page.free(); atlas.free();
```

`page.width/height/page/nLines/nAyahs/nWords/nPaths/nDecos`, `page.naturalPitch`.

## Words, ayahs, lines, decorations

| | |
|---|---|
| `page.words[i]` | `{idx, surah, ayah, word, line, lineIdx, ayahIdx, x0,y0,x1,y1, text, firstPath, nPaths}` |
| `page.ayahs[i]` | one **fragment** per printed line: `{surah, ayah, fragment, fragments, flags, rubuAlHizb, firstWord, nWords, ayahMarkDeco, bbox}` |
| `page.lines[i]` | `{lineNo, isHeader, firstWord, nWords, bbox, bandY0, bandY1, centre}` |
| `page.decos[i]` | `{kind, surah, ayah, line, bbox, text, firstPath, nPaths}` — ayah marks, surah banners, basmalah, division rosettes, sajdah signs, page furniture |
| `page.findWord(s,a,w)` | index or −1 |
| `page.resolve(target)` | word indices in reading order |
| `page.wordForm(i, form)` | `'rasm_uthmani' \| 'rasm_imlai' \| 'qpc' \| 'rasm' \| 'search'` (derived forms need the sidecar; `hasForm(form)`) |
| `page.attachWords(json)` | attach `NNN.words.json` (`{"s:a:w": {rasm_uthmani, rasm_imlai, qpc, rasm, search}}`); returns words updated |
| `page.pathKind/Mark/Family/Category(p)`, `pathWord(p)`, `pathLine(p)`, `pathNthMark(p)` | per-path facts from the geometry table |

An ayah is several fragments. `resolve('2:255')` gives all its words on the page;
`ayahWordCount(s,a)` returns `{count, complete}` — `complete` is false when the ayah
continues on another page.

**Word tokenization.** The mushaf holds **77,432** words, keyed `surah:ayah:word` — the
quran-ws shared word identity (the same keys quran-svg, quran-svg-elements and the tajweed
spans use). A host app with its own word table may tokenize an edge case differently (a
compound written as one word split into two, or the reverse); map at the boundary with
`findWord(s,a,w)` / `words[i].wordKey`, and treat a per-ayah word-count mismatch as
"skip, don't guess" — the engine's numbering follows the standard, never a host table.

## Metadata (no database needed)

`surahs()` → `{number, arabic, latin, english, place, ayahCount, hasBanner, hasBasmalah}`;
`divisions()` → juz/hizb/nisf/`rubu_al_hizb` that **start** on the page; `rosettes()` (drawn division
marks); `sajdahs()`; `ayahMarks()` → real ayah medallions with centre/radius and the
ornament/numeral path indices (swap or restyle them); `ayahKeys()`; `wordLabel(i)`,
`ayahLabel(i)` for screen readers.

## Text and search

```js
page.text('2:255')                                   // with the mushaf's own line breaks
page.text('page', {form: 'search', wordSep: ' '})
page.search('الرحمان', {mode: 'includes'})            // [{word, wordKey, text, index, loose}]
page.citation([12, 13, 14])                           // "2:255" / "2:255-257" / "2:286, 3:1"
engine.strip(s); engine.fold(s); engine.normalize(s); engine.looseKey(s)
```

Search normalises both sides (strip marks + fold) and, when the strict pass finds
nothing, retries with the loose key so a typed `الرحمان` finds the printed `الرحمن`.
Modes: `includes`, `exact`, `prefix`. Without a sidecar it searches the stripped `rasm_uthmani`.

## Hit testing

```js
page.hitTestViewEx(vx, vy, {maxDistance: 6, gapBias: 0.6})
// → {word, path, deco, line, distance, exact, wordKey, ayahKey} | null
```

Exact outline first, then **nearest with direction**: the point is resolved to a line
by its pitch band, then to a word, with a gap between two words split 60/40 towards the
preceding (right-hand) word — trailing ink is drawn *into* the following gap in this
print. `hitBoxes()` returns the same partition as boxes (no dead zones on a line);
`lineBands()` the pitch bands. `hitTest`/`hitTestView` are the exact-only variants.

## Layout

```js
const L = page.layout({viewportW, viewportH, padTop, padBottom, padLeft, padRight,
                       lineSpacing: 1.0, lineGap: 0, fillHeight: false, nominalLines: 15,
                       cropLeft: 0, cropRight: 0, maxAspectSlack: 0});
// L = {scale, ox, oy, contentW, contentH, pitch, lineDy[], slots[], fitScale, fitX, fitY}
page.layoutGapToFill(spec)   // leading (page units) that fills the padded viewport of spec
```

**The fit.** `fitScale`, `fitX`, `fitY` is the view transform that shows the whole laid-out
content in the viewport: shrink by `fitScale` when the content is taller than the viewport
(never enlarge), then centre. A host draws at `fitX + fitScale·vx`, `fitY + fitScale·vy`
and applies its own pan and zoom on top. `maxAspectSlack` bounds the content width to
`viewportH·pageW/pageH·slack` so a landscape screen does not stretch the lines (0 = no
bound; the examples use 1.15). `cropLeft`/`cropRight` cut the printed side margins (page
units) so the ink spans the padded width. Wrappers compute none of this; the engine's
answers for forty viewport cases are in `conformance/scenarios/layout.json`.

**What this is for.** A printed mushaf page is squatter than a phone screen: fitted to the
width of a tall viewport it leaves a band of empty paper top and bottom. The layout knobs
exist to spend that empty band on leading — the lines drift apart until the page fills the
screen — and for nothing else. They are an *expansion* control, never a compression one.

Horizontal placement is as printed; each line moves by `lineDy[line]`. Lines are never
re-spread onto a grid (printed lines are not equally tall or equally pitched, and ink
crosses into neighbouring lines): every line keeps its printed position and the same
delta is added between each pair of consecutive lines. `lineSpacing` sets that delta as
a multiple of the printed pitch (`pitch·(lineSpacing−1)`), `lineGap` adds leading in page
units, `fillHeight` picks the delta that makes the page fill the padded viewport. A short
page (fewer lines than `nominalLines`, pages 1–2) has no height of its own to fill, so under
`fillHeight` it takes the rows a full page gets — `(viewportH − pads)/nominalLines` each —
centred. Leading only opens up: the printed pitch is the floor for all three, and a page
that cannot fit at it reports a `contentH` taller than the viewport. `slots[]` boundaries
sit halfway between neighbouring lines, except beside a header line (surah name, basmalah),
where they stop half a pitch from the line's centre — the banner on pages 1–2 sits several
pitches above the text, and that gap is not the first line's.

**Spacing only opens up.** The printed pitch is the floor: the delta is clamped at 0, so
`lineSpacing < 1`, a negative `lineGap`, or a `fillHeight` that would need to tighten all
lay the page out exactly as printed. Lines can never be pulled closer together than the
mushaf prints them. The text width is not adjustable either: the page is always fitted to
the padded viewport width (`scale = (viewportW − padLeft − padRight) / pageW`), so the
only layout knob a reader gets is more leading, never a narrower or wider line.

Pure helpers:
`engine.gapToFill(pageW, pageH, lines, viewW, viewH, max)` and `wastedFraction(...)`.
`wordBoxView(i)` gives a word's box in viewport px for scroll-into-view.

`nominalLines` is the grid the page is laid out *inside*, not the page's own line
count: it defaults to 15 and is clamped up to `page.nLines`, never down. A short page
laid out at 15 — al-Fatiha's 7 lines, say — is therefore centred in a full-page box:
without `fillHeight` it draws at under half the height, with the rest of the viewport left
empty; with it, its lines take full-page rows. That is the spec working, not a rendering
bug. Pass `nominalLines: page.nLines` when you want the page to fill what you gave it, and
keep 15 only when several pages must share one grid.

## Styles

```js
const h = page.style(Sel.wordMark(w, 1), '#1a73e8', {ms: 200});     // the 2nd diacritic only
page.styleTarget('2:255', '#0a7d32', {layer: LAYER.HIGHLIGHT});
page.hide(Sel.kind('mark'));                                          // reading view without tashkil
page.theme({ink: '#e8e4dc', diacritics: '#7fb0e8', dots: '#ff8a80', ayahMark: '#b8860b', ms: 300});
page.restyle(h, '#ff0000', 100); page.unstyle(h); page.setDefaultInk('#231f20');
```

Rules live in **layers** (`LAYER.BASE 0`, `THEME 10`, `HIGHLIGHT 50`, `SELECTION 60`,
`TOP 100`, or any integer). Resolution per path: highest layer wins, then the most
specific selector, then the newest rule. Every rule carries a `ms` transition; colours
fade on the engine clock. Precedence around rules: masked words are hidden above
everything; the greyed-page reveal sits below rules and above the default ink.

## Clock and display list

```js
function frame(now) {
  const moving = page.tick(now);          // advance fades and band slides
  renderer.draw(page, view, dpr);         // bands → base ink → styled ink → mask boxes
  if (moving) requestAnimationFrame(frame);
}
```

`Renderer.draw()` calls `page.buildPaths()` for you, and `buildPaths()` memoises, so a
consumer using the supplied renderer never calls it directly. Call it yourself only when
you write your own renderer on top of `paint()`/`styled()`: it turns the page's op/point
arrays into the path objects those lists index, and drawing without it has nothing to
fill.

`buildPaths()` constructs `Path2D`, and so does `drawBoxes()` — `Path2D` is a DOM
interface, so **everything from `buildPaths()` down is browser-only**. In Node or in a
worker without a DOM the call fails at its first path:

```console
$ node -e 'new Path2D()'
ReferenceError: Path2D is not defined
```

Everything above drawing is pure wasm and runs anywhere: loading, `words`/`ayahs`/`lines`,
hit-testing, `layout()`, styles, `paint()` and `styled()` themselves. So a server-side or
worker consumer can use the engine for everything except the final fill, and should stop
at the display list. The native bindings build paths against their own platform types
(`CGPath`, `android.graphics.Path`, `ui.Path`) and have no such restriction.

`paint()` is the full display list (a colour per path); `styled()` lists only the
paths that differ from the default ink, which is what the cached-base-layer renderer
repaints. `highlightBoxes()` and `maskBoxes()` are viewport-px rectangles.

## Highlights

```js
const h = page.highlight('2:255', {mode: 'both', ink: '#0a7d32', band: '#0a7d3224',
                                   height: 'pitch', padX: 1.2, radius: 1.5, seam: 0.25, ms: 250});
page.rehighlight(h, '2:256');      // the band slides to the new words, ink cross-fades
page.restyleHighlight(h, {...});   // recolour in place
page.unhighlight(h);               // fades out, then disappears
```

`mode` is `ink`, `band` or `both`. A band is **one path per highlight** covering every
printed line the words occupy, with a `seam` overlap so a six-line ayah reads as one
shape and not six stripes; height is the line pitch or the words' ink. Use one handle
and `rehighlight` for word-by-word following.

## Selection

`select(anchor, focus)` snaps to whole words; `selection()`, `selectionText(form, withCitation)`.
Draw the band with a highlight in `LAYER.SELECTION`; see `web/example/app.js` for drag-to-select.

## Memorisation

```js
page.mask('2:255', 'hide' | 'block' | 'blur'); page.revealNext(1); page.hideBack(1);
page.revealWord(i); page.revealAll(); page.hideAll(); page.unmask(); page.maskHidden()
const steps = page.revealStart({lit: 2, byAyah: false, grey: '#c9c4b8', ink: '#231f20', ayahMarks: true, ms: 150});
page.revealGoto(at); page.revealStop();
```

`hide` keeps the page's shape (ink alpha 0). `block`/`blur` keep the ink and hand the
host `maskBoxes()` to draw over. The greyed-page reveal lights a window of `lit` steps
ending at `at`; a medallion lights with the ayah it closes.

## Recitation

`reciteMap(s, a, nSegments)` returns the words to pair with `nSegments` timings, or
`null` when the counts disagree — then follow the ayah whole rather than drift.
Drive the highlight with `rehighlight(h, T.word(i))`.

## Crop and export

`cropBox(target, {pad, keepAyahMarks})`; `cropSvg(target, {pad, keepAyahMarks, background})`
returns a standalone SVG string with the current colours (masks, themes and highlights'
ink applied). The medallion is kept only when the whole ayah is inside the crop.

## Atlas (cross-page)

`atlas.pageOf(s,a)`, `pageRange(page)`, `surah(n)`, `surahs()`, `pageOfSurah(n)`,
`juz(n)/hizb(n)/rubuAlHizb(n)` → `{surah, ayah, page}`, `juzAt(s,a)`, `divisionAt(kind, s, a)`,
`pagesOfJuz(n)`, `findSurah('cow' | 'البقرة' | '2')`.

## Names

```js
engine.names('mark')            // every name of a table, index = id: 'mark' | 'kind' | 'family' |
                                //   'category' | 'decoration' | 'division' | 'place'
engine.name('decoration', 0)    // 'ayah-mark'
engine.nameId('mark', 'shaddah')  // 7; 255 when the table has no such name
engine.nameCount('mark')        // 36
```

The engine holds every name table. A wrapper reads them from the engine when it starts
(`Sel.mark('shaddah')`, `Sel.deco('ayah-mark')` and the rest resolve through them) and
carries no table of its own; a name the engine does not have resolves to 255, never to 0.

## C ABI notes

Struct layouts, enums and every function signature are in `qvp.h`. Arrays are
returned through `(out, cap)` and the call returns the total count; strings through
`QvpStr {ptr, len}` valid until the next string-returning call on the same thread.
`qvp_alloc/qvp_dealloc` exist for hosts without `malloc` (wasm).
