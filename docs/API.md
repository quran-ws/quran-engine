# QVP engine API

One engine, one contract. The C ABI in `crates/qvp-ffi/include/qvp.h` is the source of
truth; every wrapper (`web/qvp.js`, Kotlin, Dart, React Native, Swift) exposes the same
names in the platform's own casing, so this page documents once and applies everywhere.
Examples are JavaScript; the same call is `page.hitTest(...)` in Dart, Kotlin and Swift
and `qvp_hit_test(...)` in C.

**Conventions**

- **Colours** are `0xRRGGBBAA`. Alpha 0 means *hidden* in a style and *leave alone* in a theme.
  Wrappers also accept `'#rgb'`, `'#rrggbb'`, `'#rrggbbaa'`.
- **Page units** are the printed page's viewBox space (345 × 550 for this mushaf, y down).
  Anything named `…View` is in *viewport pixels through the current layout*.
- **Defaults.** Every default a wrapper applies (ink, highlight colours and padding, gap bias,
  tap distance, mask and reveal colours, crop padding) is a `QVP_DEFAULT_*` value in `qvp.h`,
  defined once in the engine (`crates/qvp-core/src/defaults.rs`) and mirrored as `QvpDefaults`
  in each wrapper; the parity check keeps every copy equal.
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
| `page.targetWords(target)` | word indices in reading order |
| `page.wordForm(i, form)` | `'rasm_uthmani' \| 'rasm_imlai' \| 'qpc' \| 'rasm' \| 'search'` (derived forms need the sidecar; `hasForm(form)`) |
| `page.attachWords(json)` | attach `NNN.words.json` (`{"s:a:w": {rasm_uthmani, rasm_imlai, qpc, rasm, search}}`); returns words updated |
| `page.pathKind/Mark/Family/Category(p)`, `pathWord(p)`, `pathLine(p)`, `pathNthMark(p)` | per-path facts from the geometry table |

An ayah is several fragments. `targetWords('2:255')` gives all its words on the page;
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
ornament/numeral path indices (swap or recolorStyle them); `ayahKeys()`; `wordLabel(i)`,
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
page.hitTestView(vx, vy, {maxDistance: 6, gapBias: 0.6})
// → {word, path, deco, line, distance, isExact, wordKey, ayahKey} | null
```

Exact outline first, then **nearest with direction**: the point is resolved to a line
by its pitch band, then to a word, with a gap between two words split 60/40 towards the
preceding (right-hand) word — trailing ink is drawn *into* the following gap in this
print. `hitAreas()` returns the same partition as boxes (no dead zones on a line);
`lineBands()` the pitch bands. `hitTestExact`/`hitTestExactView` are the exact-only variants.

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
`wordBoundsView(i)` gives a word's box in viewport px for scroll-into-view.

`nominalLines` is the grid the page is laid out *inside*, not the page's own line
count: it defaults to 15 and is clamped up to `page.nLines`, never down. A short page
laid out at 15 — Fatihah's 7 lines, say — is therefore centred in a full-page box:
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
page.recolorStyle(h, '#ff0000', 100); page.removeStyle(h); page.setDefaultColor('#231f20');
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
you write your own renderer on top of `colors()`/`styledPaths()`: it turns the page's op/point
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
hit-testing, `layout()`, styles, `colors()` and `styledPaths()` themselves. So a server-side or
worker consumer can use the engine for everything except the final fill, and should stop
at the display list. The native bindings build paths against their own platform types
(`CGPath`, `android.graphics.Path`, `ui.Path`) and have no such restriction.

`colors()` is the full display list (a colour per path); `styledPaths()` lists only the
paths that differ from the default ink, which is what the cached-base-layer renderer
repaints. `highlightBoxesView()` and `maskBoxesView()` are viewport-px rectangles.

## Highlights

```js
const h = page.highlight('2:255', {mode: 'both', ink: '#0a7d32', band: '#0a7d3224',
                                   height: 'pitch', padX: 1.2, radius: 1.5, seam: 0.25, ms: 250});
page.moveHighlight(h, '2:256');      // the band slides to the new words, ink cross-fades
page.restyleHighlight(h, {...});   // recolour in place
page.removeHighlight(h);               // fades out, then disappears
```

`mode` is `ink`, `band` or `both`. A band is **one path per highlight** covering every
printed line the words occupy, with a `seam` overlap so a six-line ayah reads as one
shape and not six stripes; height is the line pitch or the words' ink. Use one handle
and `moveHighlight` for word-by-word following.

## Selection

`select(anchor, focus)` snaps to whole words; `selection()`, `selectionText(form, withCitation)`.
Draw the band with a highlight in `LAYER.SELECTION`; see `web/example/app.js` for drag-to-select.

## Memorisation

```js
page.mask('2:255', 'hide' | 'block' | 'blur'); page.unmaskNext(1); page.maskBack(1);
page.unmaskWord(i); page.unmaskAll(); page.maskAll(); page.unmask(); page.maskHidden()
const steps = page.revealStart({lit: 2, byAyah: false, grey: '#c9c4b8', ink: '#231f20', ayahMarks: true, ms: 150});
page.revealGoto(at); page.revealStop();
```

`hide` keeps the page's shape (ink alpha 0). `block`/`blur` keep the ink and hand the
host `maskBoxesView()` to draw over. The greyed-page reveal lights a window of `lit` steps
ending at `at`; a medallion lights with the ayah it closes.

## Recitation

`reciteMap(s, a, nSegments)` returns the words to pair with `nSegments` timings, or
`null` when the counts disagree — then follow the ayah whole rather than drift.
Drive the highlight with `moveHighlight(h, T.word(i))`.

## Crop and export

`cropBounds(target, {pad, keepAyahMarks})`; `cropSvg(target, {pad, keepAyahMarks, background})`
returns a standalone SVG string with the current colours (masks, themes and highlights'
ink applied). The medallion is kept only when the whole ayah is inside the crop.

## Atlas (cross-page)

`atlas.pageOf(s,a)`, `pageRange(page)`, `surah(n)`, `surahs()`, `pageOfSurah(n)`,
`juz(n)/hizb(n)/rubuAlHizb(n)` → `{surah, ayah, page}`, `juzOf(s,a)`, `divisionOf(kind, s, a)`,
`pagesOfJuz(n)`, `searchSurahs('cow' | 'البقرة' | '2')`.

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

## Every symbol

One row per C function, with its spelling in the reference wrapper (`web/qvp.js`; the other
wrappers use the same names in their own casing) and what it does. `docs/API-PARITY.md`
says which wrapper binds which.

| section | C | reference wrapper | what it does |
|---|---|---|---|
| memory | `qvp_alloc` | engine internal | Allocate `len` bytes inside the engine's memory, for hosts without their own allocator (wasm). |
| memory | `qvp_dealloc` | engine internal | Free bytes from `qvp_alloc`. |
| page | `qvp_page_load` | `engine.loadPage(bytes)` | Decode a page file into a page handle; null on a malformed file; the bytes are copied. |
| page | `qvp_page_free` | `page.free()` | Free the page handle. |
| page | `qvp_page_info` | `page.width`, `.height`, `.page`, `.nLines`, `.nAyahs`, `.nWords`, `.nPaths`, `.nDecos` | Return the page's dimensions and element counts. |
| page | `qvp_geometry` | `page.paths`, `page.buildPaths()` | Return the outline streams and the per-path table a renderer draws from; they live as long as the page. |
| page | `qvp_word_info` | `page.words[i]` | Return one word: key, line, ayah fragment, bounds, text and its path range. |
| page | `qvp_word_form` | `page.wordForm(i, form)` | Return one of a word's text forms; the derived forms need the words sidecar. |
| page | `qvp_ayah_info` | `page.ayahs[i]` | Return one ayah fragment: key, fragment index and count, flags, word range, ayah-mark decoration, bounds. |
| page | `qvp_line_info` | `page.lines[i]` | Return one printed line: number, header flag, word range, bounds, band and centre. |
| page | `qvp_deco_info` | `page.decos[i]` | Return one decoration: kind, key, line, bounds, text and its path range. |
| page | `qvp_find_word` | `page.findWord(surah, ayah, word)` | Return the page index of a word by its key, or −1 when the word is not on this page. |
| page | `qvp_target_words` | `page.targetWords(target)` | Expand a target (page, word, ayah, range, line, surah) into word indices in reading order. |
| page | `qvp_natural_pitch` | `page.naturalPitch` | Return the printed line spacing of this page, in page units. |
| metadata | `qvp_surah_count` | `page.surahs().length` | Return how many surahs have text on this page. |
| metadata | `qvp_surah_at` | `page.surahs()[i]` | Return the i-th surah on the page: number, ayah count, banner and basmalah flags, place, names. |
| metadata | `qvp_divisions` | `page.divisions()` | Return the juz, hizb, nisf and rubu_al_hizb divisions that start on this page. |
| metadata | `qvp_ayah_marks` | `page.ayahMarks()` | Return the ayah-mark medallions on the page with their centres and radii. |
| metadata | `qvp_rosettes` | `page.rosettes()` | Return the drawn hizb rosettes with the division numbers they mark. |
| metadata | `qvp_sajdahs` | `page.sajdahs()` | Return the sajdah signs on the page. |
| metadata | `qvp_ayah_keys` | `page.ayahKeys()` | Return the ayah keys on the page in reading order, packed as surah in the high 16 bits and ayah in the low 16. |
| metadata | `qvp_ayah_word_count` | `page.ayahWordCount(surah, ayah)` | Return how many of an ayah's words are on this page and whether the ayah is complete here. |
| metadata | `qvp_recite_map` | `page.reciteMap(surah, ayah, nSegments)` | Map an ayah's words onto `nSegments` recitation timings, or −1 when the counts disagree. |
| metadata | `qvp_word_label` | `page.wordLabel(i)` | Return an accessibility label for a word. |
| metadata | `qvp_ayah_label` | `page.ayahLabel(i)` | Return an accessibility label for an ayah fragment. |
| text and search | `qvp_text` | `page.text(target, form, wordSep, lineSep)` | Return the text of a target (the page by default) in one form, with the given separators. |
| text and search | `qvp_search` | `page.search(query, …)` | Search the page's words in one text form with Arabic normalisation; returns matches with their word index. |
| text and search | `qvp_arabic` | `engine.strip()`, `.fold()`, `.normalize()`, `.looseKey()`, `.searchKey()` | Apply one of the Arabic text transforms of the search fold to a string. |
| text and search | `qvp_citation` | `page.citation(words)` | Return a citation such as `2:255-257, 3:1` for a word list. |
| text and search | `qvp_attach_words` | `page.attachWords(json)` | Attach the words sidecar's text forms; returns the number of words updated, −1 on bad JSON. |
| text and search | `qvp_has_form` | `page.hasForm(form)` | Return whether a text form is available (the derived forms need the sidecar). |
| hit testing | `qvp_hit_test_exact` | `page.hitTestExact(x, y)` | Return the word, path or decoration whose exact outline contains a point in page units. |
| hit testing | `qvp_hit_test_exact_view` | `page.hitTestExactView(vx, vy)` | The same for a point in viewport pixels through the current layout. |
| hit testing | `qvp_hit_test` | `page.hitTest(x, y, options)` | Gap-aware: resolve a point to the nearest word by line band and gap bias, with the distance and whether the hit was exact. |
| hit testing | `qvp_hit_test_view` | `page.hitTestView(vx, vy, options)` | The same for viewport pixels. |
| hit testing | `qvp_line_bands` | `page.lineBands()` | Return every line's vertical band in page units. |
| hit testing | `qvp_hit_areas` | `page.hitAreas(gapBias)` | Return the gap-aware rectangle of every word, a partition of each line with no dead zone. |
| layout | `qvp_layout` | `page.layout(spec)` | Lay the page out for a viewport: scale, per-line shifts, slots, content size and the fit transform. |
| layout | `qvp_gap_to_fill` | `engine.gapToFill(pageW, pageH, lines, viewW, viewH, max)` | Return the leading that fills a viewport when fitted to width, from raw dimensions. |
| layout | `qvp_layout_gap_to_fill` | `page.layoutGapToFill(spec, max)` | The same from a layout spec; the padding is subtracted in the engine. |
| layout | `qvp_wasted_fraction` | `engine.wastedFraction(pageW, pageH, viewW, viewH)` | Return the share of a viewport left empty when the page is fitted to width. |
| layout | `qvp_word_bounds_view` | `page.wordBoundsView(i)` | Return a word's bounds in viewport pixels through the current layout. |
| styles | `qvp_style_add` | `page.style(selector, colour, ms, layer)` | Add a colour rule for a selector on a layer; returns a handle, 0 for a bad selector. |
| styles | `qvp_style_add_target` | `page.styleTarget(target, colour, ms, layer)` | Add a colour rule for a target's words. |
| styles | `qvp_style_remove` | `page.removeStyle(handle)` | Remove a rule by handle; returns how many rules were removed. |
| styles | `qvp_style_recolor` | `page.recolorStyle(handle, colour, ms)` | Change a rule's colour in place, with a transition. |
| styles | `qvp_style_clear` | `page.clearStyles()` | Remove every rule. |
| styles | `qvp_style_clear_layer` | `page.clearLayer(layer)` | Remove every rule on one layer. |
| styles | `qvp_style_default_color` | `page.setDefaultColor(colour)` | Set the ink colour a path has when no rule applies. |
| styles | `qvp_style_hide` | `page.hide(selector)` | Add an alpha-0 rule on the top layer; returns its handle. |
| styles | `qvp_theme` | `page.theme(theme)` | Apply a theme (ink, mark families, headers) as one rule set with one handle. |
| styles | `qvp_style_handles` | `page.styleHandles()` | Return the handles of every live rule. |
| clock and colours | `qvp_tick` | `page.tick(nowMs)` | Advance transitions to a time; returns 1 while anything is still animating. |
| clock and colours | `qvp_colors` | `page.colors()` | Return the current colour of every path, mid-transition included. |
| clock and colours | `qvp_styled_paths` | `page.styledPaths()` | Return the (path, colour) pairs whose colour differs from the default ink. |
| clock and colours | `qvp_color_of` | `page.colorOf(path)` | Return one path's current colour. |
| highlights | `qvp_highlight_add` | `page.highlight(target, style)` | Add a highlight (ink, band or both) for a target; returns a handle. |
| highlights | `qvp_highlight_move` | `page.moveHighlight(handle, target)` | Move a highlight to another target: the band slides, the ink cross-fades. |
| highlights | `qvp_highlight_restyle` | `page.restyleHighlight(handle, style)` | Change a highlight's style in place. |
| highlights | `qvp_highlight_remove` | `page.removeHighlight(handle)` | Fade a highlight out and remove it. |
| highlights | `qvp_highlight_clear` | `page.clearHighlights()` | Remove every highlight. |
| highlights | `qvp_highlight_handles` | `page.highlightHandles()` | Return the handles of every live highlight. |
| highlights | `qvp_highlight_words` | `page.highlightWords(handle)` | Return the words a highlight covers. |
| highlights | `qvp_highlight_boxes_view` | `page.highlightBoxesView()` | Return every highlight's band rectangles in viewport pixels; draw each id as one nonzero path behind the ink. |
| highlights | `qvp_word_bands` | `page.wordBands(words, options)` | Return band rectangles for an arbitrary word list, in page units. |
| selection | `qvp_select` | `page.select(anchor, focus)` | Select the whole words between two word indices; `QVP_NONE` clears. |
| selection | `qvp_selection` | `page.selection()` | Return the selected word indices. |
| selection | `qvp_selection_text` | `page.selectionText(form, citation)` | Return the selected text, with its citation when asked. |
| memorisation | `qvp_mask` | `page.mask(target, mode)` | Mask a target's words: hide, block or blur. |
| memorisation | `qvp_mask_from` | `page.maskFrom(word, mode)` | Mask every word from a word index to the end of the page. |
| memorisation | `qvp_mask_options` | `page.maskOptions(options)` | Set the block colour, padding, radius and direction of the mask. |
| memorisation | `qvp_unmask_next` | `page.unmaskNext(n)` | Unhide the next `n` masked words; returns how many are still hidden. |
| memorisation | `qvp_mask_back` | `page.maskBack(n)` | Re-hide the last `n` revealed words. |
| memorisation | `qvp_unmask_word` | `page.unmaskWord(i)` | Unhide one word. |
| memorisation | `qvp_mask_word` | `page.maskWord(i)` | Hide one word again. |
| memorisation | `qvp_unmask_all` | `page.unmaskAll()` | Unhide every masked word. |
| memorisation | `qvp_mask_all` | `page.maskAll()` | Hide every word in the mask again. |
| memorisation | `qvp_unmask` | `page.unmask()` | Remove the mask. |
| memorisation | `qvp_mask_hidden` | `page.maskHidden()` | Return the words currently hidden. |
| memorisation | `qvp_mask_words` | `page.maskWords()` | Return the words in the mask's scope. |
| memorisation | `qvp_mask_boxes_view` | `page.maskBoxesView()` | Return the block or blur rectangles in viewport pixels for the host to draw. |
| memorisation | `qvp_reveal_start` | `page.revealStart(options)` | Grey the page and light a moving window of steps; returns the step count. |
| memorisation | `qvp_reveal_goto` | `page.revealGoto(at)` | Light the window ending at a step; −1 when nothing is lit yet. |
| memorisation | `qvp_reveal_position` | `page.revealPosition()` | Return the current step, −2 when no reveal is running. |
| memorisation | `qvp_reveal_step_count` | `page.revealStepCount()` | Return the number of steps in the running reveal. |
| memorisation | `qvp_reveal_step_of` | `page.revealStepOf(i)` | Return the step that lights a word. |
| memorisation | `qvp_reveal_stop` | `page.revealStop()` | End the reveal and restore the page. |
| crop | `qvp_crop_bounds` | `page.cropBounds(target, options)` | Return the crop rectangle of a target with padding, and whether the ayah mark is inside it. |
| crop | `qvp_crop_svg` | `page.cropSvg(target, options)` | Return a standalone SVG of a target with the current colours applied. |
| atlas | `qvp_atlas_load` | `engine.loadAtlas(bytes)` | Decode the atlas file; null on a malformed file. |
| atlas | `qvp_atlas_free` | `atlas.free()` | Free the atlas. |
| atlas | `qvp_atlas_page_of` | `atlas.pageOf(surah, ayah)` | Return the page an ayah is on. |
| atlas | `qvp_atlas_page_range` | `atlas.pageRange(page)` | Return the first and last ayah keys of a page. |
| atlas | `qvp_atlas_page_count` | `atlas.pageCount()` | Return how many pages the atlas covers. |
| atlas | `qvp_atlas_surah_count` | `atlas.surahs()` | Return how many surahs the atlas covers. |
| atlas | `qvp_atlas_surah` | `atlas.surah(n)` | Return a surah by number: first page, ayah count, place, names. |
| atlas | `qvp_atlas_surah_at` | `atlas.surahs()[i]` | Return the i-th surah record. |
| atlas | `qvp_atlas_division` | `atlas.juz(n)`, `.hizb(n)`, `.nisf(n)`, `.rubuAlHizb(n)` | Return where a division starts: surah, ayah, page. |
| atlas | `qvp_atlas_division_of` | `atlas.juzOf(surah, ayah)`, `.divisionOf(kind, surah, ayah)` | Return the number of the division that contains an ayah. |
| atlas | `qvp_atlas_pages_of_juz` | `atlas.pagesOfJuz(n)` | Return the first and last page of a juz. |
| atlas | `qvp_atlas_search_surahs` | `atlas.searchSurahs(text)` | Search the surah names in Arabic, Latin or English, or by number. |
| atlas | `qvp_atlas_json` | `atlas.json()` | Return the atlas as JSON. |
| names | `qvp_name_count` | `engine.nameCount(table)` | Return how many ids a name table has. |
| names | `qvp_name` | `engine.name(table, id)` | Return the name of an id in a table; empty outside the table. |
| names | `qvp_name_id` | `engine.nameId(table, name)` | Return the id of a name in a table; 255 when the table has no such name. |
| names | `qvp_mark_name` | `engine.markName(id)` | Return a mark's name. |
| names | `qvp_family_name` | `engine.familyName(id)` | Return a mark family's name. |
| names | `qvp_kind_name` | `engine.kindName(id)` | Return a path kind's name. |
| names | `qvp_category_name` | `engine.categoryName(id)` | Return a mark category's name. |
| names | `qvp_mark_from_name` | `engine.markFromName(name)` | Return a mark's id by name; 255 when unknown. |
| names | `qvp_mark_category` | `engine.markCategory(id)` | Return the category a mark belongs to. |
| names | `qvp_version` | `engine.version` | Return the page format version the engine reads. |
| names | `qvp_engine_name` | `engine.engineName()` | Return the engine's name, `qvp`. |

## C ABI notes

Struct layouts, enums and every function signature are in `qvp.h`. Arrays are
returned through `(out, cap)` and the call returns the total count; strings through
`QvpStr {ptr, len}` valid until the next string-returning call on the same thread.
`qvp_alloc/qvp_dealloc` exist for hosts without `malloc` (wasm).
