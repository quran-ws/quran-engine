# The example app

Every platform ships the same example app so the four can be compared side by side and so a
developer who has used one can use the others. This page is its specification. The web
example is the reference.

## Data

The example bundles 29 pages as gitignored assets, provisioned by
`scripts/sync-example-data.sh <platform>`:

```
001 to 021, 440 to 445, 582, 604    (.qvp and .words.json)  +  atlas.qva
```

The script also copies the letter layer of each page, `NNN.qvl`, when `dist/letters/` holds
it (`docs/DATA.md`). A layer attaches only to the page file it was built with, so build the
pages and the layers from the same source. `QVP_PAGES_DIR` and `QVP_LETTERS_DIR` name other
directories than `dist/pages` and `dist/letters`. An example works without the layers: its
letter controls are then disabled.

The iOS example may bundle all 604 pages for the shareable zip, but its default build uses
the same 29.

## Features, in the order the screen shows them

1. **Page navigation**: previous, next, go to page, go to surah:ayah through the atlas.
2. **Tap a word**: the gap-aware hit test. The word's key and text shown.
3. **Search**: a text field, matches listed, tapping a match highlights it.
4. **Selection**: long-press and drag selects whole words. A panel shows the citation and
   the selected text, with copy.
5. **Highlight**: a mode picker (band and ink, band, ink) and a fade slider. "follow words"
   moves the highlight word by word on a timer.
6. **Marks**: toggle mark colours (diacritics, dots, waqf, sifr), hide marks, gold ayah
   markers.
7. **Themes**: light, sepia, dark. The three palettes are the same on every platform:

   | theme | paper | ink | background |
   |---|---|---|---|
   | light | `#FFFDF7` | `#231F20` | `#F6F1E7` |
   | sepia | `#F3E7CF` | `#3B2A14` | `#E9DCC3` |
   | dark | `#1E2126` | `#E8E4DC` | `#15171B` |

      | use | colour |
   |---|---|
   | selection highlight | `#1A73E8` |
   | ayah highlight | `#0A7D32` |
   | search highlight | `#C62828` |
   | band alpha | 0.12 to 0.18 |
   | diacritics, dots, waqf, sifr | `#1A73E8`, `#C62828`, `#0A7D32`, `#EF6C00` |
   | gold ayah markers | `#B8860B` |

8. **Memorisation**: mask (hide, block, blur), reveal step by step, reveal all.
9. **Layout**: sliders for line spacing and padding, a fill-height switch.
10. **Crop**: crop the selection to SVG and share it.
11. **Metadata and engine stats**: page info, surahs on the page, load and hit-test durations.
12. **Letters**: colour single letters and marks. The example attaches `NNN.qvl` when it
    opens a page. The controls, and what a tap does, are the same on every platform:

    | control | values | default |
    |---|---|---|
    | tap target | word, letter, mark | word |
    | colour | `#C62828`, `#1A73E8`, `#0A7D32`, `#EF6C00` | `#C62828` |
    | every match on the page | off, on | off |
    | clear letters | removes every letter rule of the page | |

    - **word**: a tap does what feature 2 does.
    - **letter**: a tap runs the letter hit test in view coordinates. A hit with a letter
      colours that letter's body and dots: the `letter` selector with the part `body and
      dots`. With "every match" on, the tap colours each letter of the page that has the same
      character: the letter query with scope page, that `text` and the same part.
    - **mark**: a tap on the ink of a mark colours that mark on its letter: the `letter mark
      named` selector with the hit's `mark`. A tap on a body colours all the marks of that
      letter: the `letter` selector with the part `marks`. With "every match" on, a tap on a
      mark colours that mark on each letter of the page: the letter query with scope page,
      that `mark` and the part `marks`. A tap on a body then does nothing.
    - A tap is on a mark when the hit is exact, is not on a body, and has a mark id that is
      not 0. Each other hit with a letter counts as a tap on the body.
    - A hit with no letter (a word-level mark, or no hit) colours nothing.
    - Each tap adds one rule in the chosen colour with a 200 ms transition, on the top
      style layer, so a mark theme does not cover it. The rules stay
      until "clear letters", or until the example opens another page.
    - A line of text shows the last hit: the letter's character and code point, the word
      key, the letter's index, its offset, its mark count, the mark name when the tap hit a
      mark, and whether the hit was exact.
    - When the page has no layer, or the engine refuses it, the controls are disabled and
      the line says "no letter layer for this page".

This list is closed: a platform may add a feature only after adding it here and to the
other platforms.

## Shared behaviour

- **Navigation**: a request for a page that is not in the bundled set opens the nearest
  bundled page. An example that bundles every page clamps to the first and last page, which
  is the same rule.
- **Zoom**: pinch limits are 0.5 to 12 times the fitted scale on every platform. The
  constants live in each package's page view, not in the example.
- **Reveal slider**: the range is 0 to one less than the step count the engine returns from
  `revealStart`; the slider is disabled when there is one step or none.

## Page cache

An example that pages through the mushaf keeps one permanent controller per page and
cycles the page data through a least-recently-used cache. The current page and its two
neighbours stay resident. `QvpPageCache` on iOS implements this policy and the other
platforms follow it.

## Tests

One UI smoke test per example: open page 042, tap a word, highlight an ayah, assert the
highlight boxes are non-empty.
