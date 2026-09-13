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

Page files are served from `qvp.quran.ws` under immutable, versioned URLs, so a
browser can load one page without shipping the data — `docs/CDN.md`:

```js
const page = await loadPage('https://qvp.quran.ws/v0.1.0/042.qvp')
```

Decoded words include their `surah`, `ayah`, `word` and page-coordinate `box`.
`page.hitTest(x, y)` returns the word at a point in those same page coordinates.
`drawWords(ctx, wordIndices, options)` draws selected words with their dots,
diacritics and pause marks. `drawDecorations(ctx, options)` draws non-word page
elements such as ayah markers, surah banners, basmalahs, division and sajdah
marks, running heads and page numbers. Both accept the same `scale`, `x`, `y`
and `ink` options as `draw()` and leave clearing and sizing to the caller.

Use the main package for layout, exact hit-testing, search, styling, selection,
masks and animation.

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
| `docs/` | the API, the format, and the measured numbers |

Issues and pull requests are welcome here. Everything that is not about *changing* this repository is on the site.
