# How the engine works

This page describes the engine's components and data flow for a developer who is new to
the project. A glossary at the end defines the project and domain terms.

## The one-paragraph version

A mushaf page starts as a drawing in which every letter, dot and mark is a vector outline. The
converter turns that drawing into a small binary page file. The engine loads a page file,
indexes the position of every word and mark, and computes answers from that index: the
word under a touch point, the line positions that fill a tall screen, the colour of each
outline at the current moment. Your app renders the outlines with its own graphics API and
passes touch events to the engine. The engine computes geometry, layout and colours, and the
app renders them.

## From a drawing to pixels

```
source SVG ──converter──▶ page file ──engine──▶ outlines + colours ──your app──▶ screen
 (pages/)   qvp-convert    (.qvp)     qvp-core                        Canvas, CoreGraphics,
                                                                      Skia, Canvas2D
```

1. **The source.** One SVG per page from the `quran-svg-elements` release, the Madani
   mushaf in the Hafs reading. Every word is a group of vector outlines (paths made of lines and curves)
   tagged with its key, `surah:ayah:word`. Marks such as vowels and pause signs are
   separate outlines with their own names.
2. **The converter** (`crates/qvp-convert`) reads the SVG and writes a page file. It is
   lossless: no curve is simplified, no point is dropped. A test renders the source and the
   page file back to pixels and compares them. All 604 pages must match.
3. **The page file** (`.qvp`, Quran Vector Page) holds the outlines, plus tables that say
   which outline belongs to which word, which word to which line and ayah, and where each
   one sits on the page. A page is about 210 KB, a quarter of the SVG. Coordinates are
   *page units*: the printed page's own coordinate space, 345 by 550 for this mushaf.
4. **The engine** (`crates/qvp-core`, written in Rust) loads a page file and holds its
   tables and outlines in memory. It exposes about 110 functions through a C header, so
   the same engine runs inside iOS, Android, Flutter and React Native, and in the browser
   through WebAssembly.
5. **The wrapper** is a thin layer in each language. It passes your calls to the engine,
   and it renders what the engine returns with the platform's own canvas. It computes
   nothing itself.

Two more files travel with the pages. The **atlas** (`atlas.qva`) indexes the whole
mushaf: the page that holds ayah 2:255, the ayah where each juz starts. The **words sidecar**
(`NNN.words.json`) carries the other spellings of each word, such as the plain modern
spelling used for search.

## Load, draw, tap

The whole common case in the reference web wrapper:

```js
const page = engine.loadPage(bytes);          // a .qvp file's bytes
draw(page);                                    // your paint routine over page.paths
const hit = page.hitTestViewEx(x, y);          // the word under a finger, or null
```

The features below are optional and reachable from the same page object.

## What the engine computes

### Which word is under a finger

Words on a printed line do not sit in neat boxes: one word's tail often reaches into the
next word's space. The engine first tests the exact outlines. If the finger missed the ink,
it finds the line by its **band** (the horizontal strip a line occupies) and then the
nearest word, splitting the gap between two words 60/40 towards the preceding word,
because that is where the trailing ink goes in this print. There is no dead zone on a
line. This is *gap-aware* hit testing. The exact-only variant exists for when you need
it.

### How the lines spread on a tall screen

A printed page fitted to a phone's width leaves empty paper above and below. The engine can
distribute that space by moving each line further down than the last: line one stays, line two
moves down by *d*, line three by *2d*, and so on. Every gap grows by the same amount and no
line is reshaped. Printed lines are not equally tall, and ink often reaches into the line
above, so the engine snaps nothing to a grid. Spacing only opens up: the printed spacing is the
floor, and the width is always the screen's width.

### What colour each outline is

Styles are rules on layers: base, theme, highlight, selection, top. A rule says "these
outlines, this colour, fade over this many milliseconds". A rule can address a whole page,
a line, an ayah, a word, or one diacritic of one word. Every rule returns a **handle**;
removing the handle undoes exactly that rule. The engine resolves all rules into one colour
per outline, and your app renders those colours.

Mark colouring for tajwid-style display is one such rule set: colour the diacritics blue,
the dots red, the pause marks green.

### Following a recitation

A **highlight** is a rule plus, optionally, a band: one shape behind the words that covers
every line they occupy, with a small overlap at the seams so a six-line ayah renders as one
continuous band. Move a highlight to the next word and the band slides, the ink
cross-fades. A clock call each frame reports whether anything is still animating.

### Selecting

A long press and a drag select whole words between an anchor and a focus. The engine returns
the words, their text, and a citation such as `2:255-257`.

### Memorising

A **mask** hides words: keep the page's shape but paint them invisible, or cover them with
a block or a blur that your app draws. **Reveal** greys the page and lights a moving window
of words, step by step, ayah marks lighting with the ayah they close.

### Searching

Search runs over the words on a page, in any of the stored spellings, with Arabic
normalisation so a plain typed query matches the Uthmani script. Matches come back as word
indices, ready to highlight.

### Cutting out an ayah

Crop returns a standalone SVG of any target with the current colours applied, for sharing.

### Finding a page

The atlas resolves cross-page lookups without a database: which page holds an ayah, the
page range of a surah, the ayah where juz 30 starts, a surah by name in Arabic or English.

## What the app does

Your app controls the screen. It loads bytes, keeps the page object, renders the outlines
with the colours the engine returns, and forwards taps, drags and pinches. The app also
implements zooming and panning. The engine computes every number that affects the position
or the colour of an outline.

## Units and coordinates

- **Page units**: the printed page's space, y down. Every geometric answer is in page units
  unless the name says *view*.
- **View**: viewport pixels through the current layout. `hitTestViewEx` takes them. The
  highlight and mask boxes come back in them, ready to draw.

## Glossary

| term | meaning |
|---|---|
| mushaf | the Quran as a printed and bound volume. This engine renders the Madani mushaf, Hafs reading |
| rasm | the letters of a word without vowels and marks. *Uthmani* is the Quranic spelling, *imlai* the modern one |
| mark | a vowel sign, a dot, a pause sign or a similar small outline attached to a word |
| waqf | a pause sign in the text |
| juz, hizb, rubu al-hizb | the mushaf's thirtieths, sixtieths and quarter-hizbs, marked in the margin |
| ayah mark | the medallion that closes an ayah and carries its number |
| decoration | any drawn element that is not a word: ayah marks, surah banners, basmalah, division rosettes, sajdah signs, page numbers |
| page units | the printed page's coordinate space, 345 × 550 for this mushaf |
| band | the horizontal strip a printed line occupies |
| layer | a level in the style stack. Higher layers win |
| handle | the number a style or highlight call returns. Removing it undoes that call |
| atlas | the whole-mushaf index: pages, surahs, divisions |
| words sidecar | the JSON file with each word's other spellings |
