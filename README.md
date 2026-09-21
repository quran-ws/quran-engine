<div align="center">

<img src=".github/banner.svg" alt="Quran Engine — Rendering, Beta" width="820">

**A rendering engine for interactive Mushaf pages that uses each platform's native graphics APIs.**

<a href="https://quran.ws/blocks/quran-engine"><img alt="See it work" src="https://img.shields.io/badge/See_it_work-15705D?style=for-the-badge&labelColor=102F29"></a>
<a href="https://quran.ws/docs/reference/quran-engine"><img alt="Documentation" src="https://img.shields.io/badge/Documentation-102F29?style=for-the-badge&labelColor=102F29"></a>

</div>

Use it when building Quran applications for mobile or desktop and you need fast, lightweight page rendering with interactive access to page elements.

> محرّك لعرض صفحات المصحف التفاعلية بكفاءة باستخدام الرسم الأصلي لكل منصة.
>
> استخدمه عند بناء تطبيقات القرآن على الجوال أو سطح المكتب عندما تحتاج عرضًا سريعًا وخفيفًا لصفحات المصحف مع إمكانية التفاعل مع عناصرها.

| | |
|---|---|
| **Package** | `@quran.ws/engine` · `0.1.0` |
| **Whole mushaf** | 26 MB as one bundle · 41.8 MB page by page · 92.6 MB raw |
| **Wasm engine** | 311 KB |
| **Licence** | MIT (the code) · source bundle terms (the data) |

```sh
# not published — build the wasm, or take it from the release
```

For a small Canvas-only reader that needs page drawing and word bands, import
the dependency-free QVP decoder. It loads the original page files directly and
does not load Wasm:

```js
import { loadPage } from '@quran.ws/engine/lite'

const page = await loadPage('/pages/042.qvp')
const canvas = document.querySelector('canvas')
page.draw(canvas.getContext('2d'), page.fit(canvas, 24))
```

### Canvas resolution

Canvas2D rasterises these unhinted vector outlines. Give the canvas at least two backing
pixels per CSS pixel. Above that floor, use the display's device-pixel ratio rather than a
multiple of it. Extra supersampling makes the browser downsample the result and can leave
thin strokes and diacritics soft or uneven.

```js
const rect = canvas.getBoundingClientRect()
const pixelRatio = Math.max(2, window.devicePixelRatio || 1)
canvas.width = Math.round(rect.width * pixelRatio)
canvas.height = Math.round(rect.height * pixelRatio)
canvas.style.width = `${rect.width}px`
canvas.style.height = `${rect.height}px`
page.draw(canvas.getContext('2d'), page.fit(canvas, 24 * pixelRatio))
```

An app may cap the backing dimensions or total pixel count to control memory use. The web
example uses this rule and caps its ratio at three.

Page files are served from `cdn.quran.ws` under immutable, versioned URLs, so a
browser can load one page without shipping the data — `docs/CDN.md`:

```js
const page = await loadPage('https://cdn.quran.ws/qvp/v0.4.0/042.qvp')
```

`cdn.quran.ws` mirrors the signed releases of the stack. This repo publishes two
kinds of artifact, each on its own version line:

| folder | holds |
|---|---|
| `qvp/<version>/` | page data, atlas, text sidecars, QVP/SVG/font surah-name assets, the brotli bundle |
| `engine/wasm/<version>/`, `engine/apple/<version>/`, `engine/android/<version>/` | engine builds |

`latest.json` beside each names the current version. Other repositories of the
stack publish their own folders on the same host — `docs/CDN.md` has the layout.

`qvp.quran.ws/<version>/` redirects to `cdn.quran.ws/qvp/<version>/`, so URLs
published before the move still resolve.

Decoded words include their `surah`, `ayah`, `word` and page-coordinate `box`.
`page.hitTestExact(x, y)` returns the word at a point in those same page coordinates.
`drawWords(ctx, wordIndices, options)` draws selected words with their dots,
diacritics and pause marks. `drawDecorations(ctx, options)` draws non-word page
elements such as ayah markers, surah banners, basmalahs, division and sajdah
marks, running heads and page numbers. Both accept the same `scale`, `x`, `y`
and `ink` options as `draw()` and leave clearing and sizing to the caller.

For a standalone verse excerpt that wraps to its container without Wasm, import the optional `QvpPassage` from `@quran.ws/engine/lite/passage`. It takes `decodeGeometry()` pages and a complete ayah range, including ranges across pages, and preserves verse medallions and sajdah signs. See [Lite passages](docs/LITE-PASSAGES.md). The base lite import does not load this layout code.

Use the main package for full-page layout, exact hit-testing, search, styling, selection,
masks and animation.

On iOS and macOS, add the repository as a Swift package and use its `QvpKit` product:

```swift
.package(url: "https://github.com/quran-ws/quran-engine.git", from: "0.2.2")
```

SwiftPM downloads the release XCFramework and verifies its checksum. Page data remains a
separate app or CDN resource.

## Where the documentation is

Everything about using it lives on the site. This repository is the source.

| | |
|---|---|
| **Overview and demo** | [quran.ws/blocks/quran-engine](https://quran.ws/blocks/quran-engine) |
| **Reference** | [quran.ws/docs/reference/quran-engine](https://quran.ws/docs/reference/quran-engine) |
| **Make words clickable** | [quran.ws/docs/build/clickable-words](https://quran.ws/docs/build/clickable-words) |
| **Render on iOS, Android, Flutter and React Native** | [quran.ws/docs/build/platforms](https://quran.ws/docs/build/platforms) |
| **Work offline** | [quran.ws/docs/build/offline](https://quran.ws/docs/build/offline) |
| **Licensing in full** | [quran.ws/docs/reference/licensing](https://quran.ws/docs/reference/licensing) |

## What is in here

| | |
|---|---|
| `crates/` | the Rust core: `qvp-format`, `qvp-core`, `qvp-convert`, `qvp-ffi` |
| `packages/` | the thin platform wrappers — iOS, Android, Flutter, React Native |
| `web/` | the reference web harness the browser demo runs on |
| `conformance/` | the gates that must stay green |
| `scripts/` | build and packaging |
| `docs/` | `HOW-IT-WORKS.md` (start here), the API, the standards, and the measured numbers |

Issues and pull requests are welcome here. `CONTRIBUTING.md` says how; `docs/HOW-IT-WORKS.md` describes the engine's components and data flow. Everything that is not about *changing* this repository is on the site.
