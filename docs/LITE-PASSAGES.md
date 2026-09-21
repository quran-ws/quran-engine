# Lite passages

`@quran.ws/engine/lite/passage` lays out a contiguous range of complete ayahs in JavaScript. It loads no Wasm. Words keep their original outlines, dots and reading signs; the layout moves them onto rows of the requested width. It includes every selected ayah medallion and the relevant division and sajdah marks. Page numbers, running heads, surah banners and unnumbered basmalahs belong to the printed page and are not part of an excerpt.

This is an optional entry point. Importing `@quran.ws/engine/lite` alone does not load the passage layout. It is not a replacement for the full engine's page reader, zoom control, selection or styles.

```js
import { decodeGeometry } from '@quran.ws/engine/lite'
import { QvpPassage } from '@quran.ws/engine/lite/passage'

const response = await fetch('https://cdn.quran.ws/qvp/v0.4.0/042.qvp')
if (!response.ok) throw new Error(`Page: HTTP ${response.status}`)
const page = decodeGeometry(await response.arrayBuffer())
const passage = new QvpPassage([page], { surah: 2, from: 255 })
const layout = passage.layout({ width: 366, scale: 0.9 })

const ratio = Math.max(2, window.devicePixelRatio || 1)
canvas.width = Math.ceil(layout.width * ratio)
canvas.height = Math.ceil(layout.height * ratio)
canvas.style.width = `${layout.width}px`
canvas.style.height = `${layout.height}px`
passage.draw(canvas.getContext('2d'), layout, { pixelRatio: ratio, ink: '#231f20' })
```

## Inputs and results

`new QvpPassage(pages, {surah, from, to = from})` accepts the geometry returned by `decodeGeometry()`, not `QvpLitePage`. Supply every page containing the requested range. Pages may arrive in any order. Missing ayahs, incomplete fragments, duplicate pages and passages over 4,096 words throw rather than show partial Quran text. The app decides which pages to fetch and how many ayahs to show; the module does not fetch anything. `passage.ayahs` contains `{surah, ayah, text}` in reading order, with the text from the page's own word records.

`passage.layout({width, scale = 1, lineSpacing = 1, padding = 2, align = 'right'})` returns:

- `width`, `height`, `scale`: CSS-pixel canvas dimensions and the actual scale from page units. Scale shrinks only if a whole word and its attached signs would exceed the width.
- `rows`: each row's `box` and `baseline` in CSS pixels.
- `words`: `{surah, ayah, word, row, box}`. A box includes signs that travel with that word.
- `ayahs`: `{surah, ayah, text, box}`, suitable for an accessible text layer or scrolling to a cited ayah.

`align` accepts `right` or `center`. `lineSpacing` is a multiplier, at least 1. The rows may need more room for tall ink. The layout balances row lengths without stretching the gaps; it is intentionally not the full page reader's fitted breaking or zoom policy. Layout is independent of canvas resolution and can run without the DOM.

`passage.draw(context, layout, {pixelRatio = 1, ink = '#231f20'})` draws the layout made by that passage. It saves and restores the context but does not clear or resize its canvas. The caller supplies the backing ratio and caps the canvas dimensions or pixel budget as needed. Drawing requires Canvas2D and `Path2D`; decoding and layout do not.

## Geometry and lifetime

The decoder now exposes `lines`, `ayahs`, `decorations`, word `text`, and path `kind`, `mark` and `family` alongside the existing geometry. Indices are zero-based. Optional decoration and line indices are -1 when absent. Empty line/ayah/decoration bounds are `null`. The QVP wire format is unchanged.

The passage module measures outlines in thin bands rather than separating word bounding boxes. It keeps deeply interlocked adjacent words at their printed relative position. All pages share the same band height, so a range spanning a page boundary can form one row. Curves are sampled only to measure spacing: drawing uses their unchanged original operations. The reference Rust engine supplies fixtures for line spacing and ink clearance; the clearance test allows 0.75 page units because the Rust reader's band height varies with each page while the passage bands must be shared across pages.

Ayah medallions and sajdah signs travel with the word that closes their ayah. A division mark travels with the word it opens. A sajdah stroke belongs to the words geometrically underneath it, which can be in an earlier ayah than the closing sign. If those words wrap, the stroke is drawn on each affected row; only that stroke is stretched horizontally.

Decoded pages are immutable inputs. Preparation is bounded per page to two million sample points and 32,768 allocated measurement bands; exceeding either limit throws. This prevents extreme coordinates or repeated geometry from expanding an otherwise bounded QVP file into unbounded work or memory. The complete 604-page dataset is checked against these limits. Preparation and Canvas paths use weak caches keyed by those pages. Keep a bounded app-owned page cache; dropping a passage, its layouts and unused pages releases the associated data. Reuse the passage when its container changes width. Do not lay it out on every scroll event.

A canvas is not an accessible text document. Supply a Unicode text layer for screen readers and copying. The module's text is optional to the host; an app may keep its existing verified text corpus. Theme changes require drawing with a new ink colour, not recomputing the layout.

## Checks

`pnpm run test:lite` runs synthetic tests and the Rust codec fixture without data. With `dist/pages/` present, it also checks all 604 pages at three widths, all ayah medallions, a range spanning pages and the Rust-generated ink measurements. `scripts/check.sh gates` requires those data tests. `scripts/gen-conformance.sh` regenerates `conformance/lite-passage-metrics.json` from Rust; no Wasm is needed by passage consumers.
