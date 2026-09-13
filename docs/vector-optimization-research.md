# Smaller Quran vector pages for the web

Research brief · 12 September 2026

## What we are trying to do

We want to display all 604 pages of the Madani mushaf in a browser, preserving the printed Arabic calligraphy, with fast page loading, smooth zooming, and word-level interaction. The source material is already vector outlines stored in per-page `.qvp` binary files.

The immediate research question is: **How much smaller can we make these outlines, beyond ordinary lossless compression, while preserving their appearance and keeping browser decoding and drawing inexpensive?**

We are looking for existing tools, algorithms, and relevant production experience. A proven offline optimizer would be preferable to writing a new curve simplifier.

## Constraints and quality requirements

- **Browser runtime:** JavaScript with Canvas 2D is the current starting point; GPU rendering remains an option. Avoiding browser WebAssembly is a project preference, not a measured conclusion that JavaScript is always faster.
- **Delivery:** compact binary assets, fetched and cached page by page. Shipping SVG is outside the current direction because of size concerns; SVG is acceptable as an offline interchange format.
- **Appearance:** preserve letterforms, dots, diacritics, decorations, small holes, contour relationships, and page layout. Tiny local changes can matter even when a whole-page difference score is low.
- **Interaction:** retain path-to-word associations and the metadata needed for selection, highlighting, and styling. Combining unrelated paths solely to save bytes could break these features.
- **Preprocessing:** native tools and expensive offline work are acceptable if they reduce download size or browser work.

The acceptable loss remains an open decision. Exact geometry preservation and visually indistinguishable approximation are different targets. We have not approved a lossy tolerance or a maximum supported zoom.

## Data and tooling available

The release contains all 604 `.qvp` files, optional word-text JSON sidecars, a cross-page atlas, and platform packages. This repository contains the Rust codec, SVG converter, core engine, C ABI, web decoder, and platform wrappers. Its code is MIT-licensed; the source artwork retains the King Fahd Complex's published usage terms recorded here.

Decoded geometry contains move, line, quadratic Bézier, cubic Bézier, and close operations; float32 coordinates; fill rules; and path associations. The page coordinate space is 345 × 550 units. See the [C API](../crates/qvp-ffi/include/qvp.h) and [API guide](API.md).

The QVP encoder is `crates/qvp-format/src/codec.rs`, and `crates/qvp-convert` provides SVG→QVP and QVP→SVG conversion. We independently built a small browser JavaScript decoder before receiving the source; it matches all 604 native exports byte-for-byte for commands, coordinates, path flags, associations, and word boxes. The source codec confirms its rendering logic and gives optimization experiments a supported way to write `.qvp` files.

The original binary uses packed operations, delta and variable-length integer coordinates, and reusable glyph instances. The [format investigation](qvp-format-investigation.md) documents the verified layout. Optimization experiments must account for those existing techniques.

## What we have measured

### File size across all 604 pages

These totals include only `.qvp` files, excluding word sidecars, the atlas, and engine code. Each page was compressed independently using Node's zlib APIs, then the output lengths were summed. MB below is decimal; HTTP overhead is excluded.

| Representation | Total bytes | Total MB | Reduction from original |
| --- | ---: | ---: | ---: |
| Original QVP | 81,508,351 | 81.51 | — |
| gzip, level 9 | 41,656,734 | 41.66 | 48.9% |
| Brotli, quality 6 | 39,927,719 | 39.93 | 51.0% |
| Brotli, quality 11 | 37,243,529 | 37.24 | 54.3% |

Lossless compression already roughly halves download size. Serving negotiated Brotli responses with `Content-Encoding: br` lets the browser decode the HTTP content encoding; it does not require our own WASM decompressor. It does not reduce the size of the resulting decoded geometry. See [HTTP content encoding](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Encoding).

**37.24 MB is the independent-page compressed baseline to beat.** The published `hafs-kfgqpc.tar.br` instead compresses the complete corpus as one Brotli stream and is 25.9 MB because it can find repetition across page boundaries. That bundle is useful for native clients downloading every page; browser readers should normally fetch individual pages from the CDN. No vector simplification savings have been measured yet.

### Curve composition in three sampled pages

We exported pages 1, 42, and 585. Page 585 has the largest decoded coordinate count across the corpus; these three pages are not a statistical sample.

| Page | Quadratic segments | Cubic segments |
| --- | ---: | ---: |
| 1 | 3,724 | 4,177 |
| 42 | 1,064 | 16,304 |
| 585 | 10,640 | 16,828 |
| Total | 15,428 | 37,309 |

About **71% of curve segments in this sample are cubic**. This matters when considering optimizers designed for quadratic TrueType outlines.

### Browser rendering

We built a [reproducible benchmark](../web-benchmark/README.md) comparing cached Canvas 2D, direct Canvas 2D using cached `Path2D` objects, and WebGL2 using worker-generated triangles.

On an M3 Max Mac running Chrome, at 390 × 620 CSS pixels and DPR 3, Canvas was a promising starting point. For page 42, median warm setup was 7.075 ms for cached Canvas versus 78.235 ms for WebGL. During continuous sharp zoom, pooled CPU submission p95 was 0.345 ms versus 0.035 ms, but measured frame callback pacing was similar. Direct Canvas also kept up. Full [results and limitations](../web-benchmark/RESULTS.md) are available.

This renderer comparison tested three pages on desktop hardware and excluded downloads and original QVP decoding: the native engine exported decoded geometry offline into simple binary fixtures. A separate [decoder comparison](qvp-format-investigation.md#measured-browser-preparation-cost) measures direct JavaScript QVP preparation at 1.1–4.0 ms median on the same desktop class of hardware. Neither benchmark establishes phone performance.

## Candidate approaches

| Approach | Potential benefit | What needs investigation |
| --- | --- | --- |
| Exact geometric cleanup | Remove redundant operations; reduce curve degree where exactly equivalent | How much redundancy actually exists, and whether renderer behavior is preserved |
| Coordinate quantization and delta encoding | Fewer coordinate bytes and potentially better compression | Required precision at the supported zoom; what QVP already does |
| Reuse identical outlines | Store a shape once with instance transforms | Actual repetition, existing reuse, metadata preservation, and page-fetch dependencies |
| Approximate Bézier simplification | Represent an outline with fewer segments or control points | Local error, corners, holes, tangent continuity, and final compressed savings |
| Mixed cubic/quadratic encoding | Use the cheaper representation for each section | Whether conversion saves coordinates after all segments and metadata are counted |

These are hypotheses, not established improvements for this corpus. Fewer points do not automatically mean fewer compressed bytes or faster rendering.

### Fontcrunch

[Fontcrunch](https://github.com/googlefonts/fontcrunch#how-it-works) is a particularly relevant candidate. It optimizes TrueType outlines using dynamic programming, balancing point count against integrated position and tangent error. Its C++ core accepts smooth sequences of quadratic Béziers directly, so it could be adapted to offline outline processing without shipping a font.

Its main constraints for us are quadratic curves, grid-quantized coordinates, and a cost model that rewards TrueType's implied on-curve midpoints. Those omitted points only save binary space if the destination encoding supports the convention. Its error objective is not a strict maximum-deviation guarantee or a guarantee of unchanged holes and pixels.

A useful experiment would optimize existing quadratic sections first, then separately test cubic conversion followed by Fontcrunch. Corners and nonsmooth joins need appropriate segmentation, and grid scale must be chosen deliberately.

### fontTools cu2qu and SVGO

[fontTools cu2qu](https://fonttools.readthedocs.io/en/latest/cu2qu/index.html) converts cubic Béziers to quadratic splines with a specified tolerance. It can subdivide a cubic into multiple quadratics, so conversion alone can increase point count. It is a candidate adapter for Fontcrunch; any subsequent optimization needs its own error evaluation.

[SVGO's convertPathData](https://svgo.dev/docs/plugins/convertPathData/) offers redundant-command removal, curve conversion, and coordinate rounding. It could provide an offline cleanup baseline. SVG text shortening does not necessarily translate into binary savings, so we must measure the re-encoded result and preserve path associations.

## Proposed evaluation

1. **Establish references.** Keep immutable original outlines and metadata. Render the originals and candidates through the same renderer; also compare export output against native-engine reference renders to catch shared export mistakes.
2. **Try separate transformations.** Compare exact cleanup, quantization, quadratic-only Fontcrunch, and cubic conversion plus Fontcrunch. Keep an unchanged baseline through the same encoding pipeline.
3. **Define a quality envelope.** Explore tolerances such as 0.1, 0.25, and 0.5 backing pixels at 4× zoom, as experiments rather than accepted limits. Convert these to page units using the complete rendering transform, and account for error accumulated across stages.
4. **Check local failures.** Inspect differences around dots, diacritics, joins, and holes; measure contour and component changes as well as pixel errors. Inspect worst cases visually. A low average image error cannot certify every detail.
5. **Measure the delivered result.** Record raw and Brotli bytes, operation counts, offline processing time, browser parsing and preparation, first draw, interaction pacing, and memory. Include any shared dictionaries and additional runtime code.
6. **Expand validation.** Use the three fixtures for exploration, then evaluate all 604 pages and actual target phones before selecting a production approach. Retain original geometry for any section that fails validation.

Implement experimental cleanup or simplification before `qvp_format::encode`, then measure the resulting `.qvp` and Brotli bytes directly. The existing identity test already compares source SVG and SVG→QVP→SVG rasters at 4×; lossy work needs stricter local geometry, topology, and worst-case checks in addition to that whole-page gate.

## Questions to ask researchers and tool authors

- Which existing optimizers handle mixed cubic and quadratic filled outlines while preserving corners and contour topology?
- Can they enforce a maximum geometric error, or only minimize an average or integrated error objective?
- Is there a cubic equivalent of Fontcrunch that avoids conversion overhead?
- How should a point-count objective be adapted to actual binary and Brotli cost?
- How much reuse is realistic for positioned Arabic calligraphy without changing shapes or word associations?
- What validation catches disappearing marks or closing holes reliably across a large corpus?
- What browser implementations have measured the full download → decode → first-draw path on phones for comparable data?

## Short prompt to share

> We are optimizing 604 pages of Quran calligraphy stored as binary vector outlines for a browser reader. Original page files total 81.51 MB; independent Brotli compression reduces them to 37.24 MB losslessly, while one cross-page Brotli stream is 25.9 MB. We want further savings while preserving dots, diacritics, small holes, page layout, and word-level metadata. Our three sampled pages contain about 71% cubic and 29% quadratic curve segments. Canvas 2D is the current rendering baseline, with a validated 4.1 KB-gzipped JavaScript decoder and no browser WASM; expensive native preprocessing is fine. The MIT-licensed Rust encoder and SVG converter are available. The format already uses packed operations, delta-varint coordinates, and reusable glyph instances. Which existing tools or algorithms would you evaluate for outline simplification, quantization, and additional shape reuse? Fontcrunch looks relevant, but it targets quadratic TrueType curves. We especially need advice on error guarantees, topology preservation, and measuring compressed-byte savings rather than point count alone. No lossy simplification has been tested or approved yet.
