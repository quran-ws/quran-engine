# QVP1 format investigation

12 September 2026. Initially reconstructed from the release files and native engine, then checked against this repository's QVP encoder and decoder source at commit `6d784c0`.

**QVP uses a compact, approachable layout built from familiar binary-encoding techniques.** A direct JavaScript decoder is practical. The web SDK now reconstructs its Canvas geometry from original `.qvp` files without WASM, dependencies, or offline expansion.

The layout is specific to QVP; the integer encoding and bit packing are ordinary techniques. The evidence does not justify treating this as an expensive or unusually complex codec.

## Verified layout

All multibyte header values are little-endian. The header is 64 bytes, followed by six sections whose lengths are stored in the header.

| Offset | Type | Meaning |
| --- | --- | --- |
| 0 | 4 bytes | `QVP1` magic |
| 4 | uint16 | Version, 1 in this corpus |
| 6 | uint16 | Coordinate scale, 100 in this corpus |
| 8 | uint16 | Page number |
| 10 | uint16 | Not interpreted by the prototype |
| 12, 16 | float32 | Page width and height |
| 20, 22, 24, 26 | uint16 each | Line, ayah-fragment, word, decoration counts |
| 28 | uint32 | Path count |
| 32, 34, 36 | uint16 each | String, glyph, instance counts |
| 38 | uint16 | Not interpreted by the prototype |
| 40–63 | 6 × uint32 | Byte lengths of the six sections below |

The sections, in order:

1. **Metadata.** Line, ayah, and word records; four byte arrays for path kind, mark, family, and flags; per-path command counts or instance references; instance boxes; decoration records; glyph definitions; and placement transforms. Most numeric fields are variable-length integers. An instance stores a uint16 glyph reference followed by six float32 affine-transform coefficients.
2. **Drawing operations.** Four operations per byte, low bits first: `0 = move`, `1 = line`, `2 = quadratic`, `3 = cubic`. This section excludes close-path operations.
3. **Close-path positions.** A variable-length count followed by delta-encoded positions in the stream of non-close operations.
4. **X coordinates.** Signed coordinate deltas, encoded with ZigZag and unsigned base-128 varints.
5. **Y coordinates.** The corresponding signed deltas, using the same encoding.
6. **Strings.** UTF-8 strings with varint byte lengths.

A varint carries seven value bits per byte; the top bit indicates another byte follows. ZigZag maps unsigned integers back to signed values with `(n >>> 1) ^ -(n & 1)`. Reconstructing a coordinate adds its delta to the previous coordinate. Each curve's control points and endpoint participate in that sequence. At a shape boundary, the predictor returns to that shape's first move point before decoding the next shape.

Stored integer coordinates are divided by the coordinate scale. Glyph instances reuse a shape and apply an affine transform. Matching the native engine exactly requires doing instance transforms in double precision and rounding the resulting coordinates to float32. Ordinary paths are also exposed as float32 coordinates.

This is enough to reconstruct the Canvas geometry. The Rust implementation in `crates/qvp-format/src/codec.rs` confirms the header, tables, predictors, glyph handling, and operation streams above. The lightweight SDK deliberately exposes only the rendering subset; the source engine also implements the complete metadata and higher-level behavior.

## Implemented and validated

The decoder and Canvas renderer are integrated into the single-file [web SDK](../web/lite.mjs). `decodeGeometry(buffer)` reconstructs the geometry, while `decodePage(buffer)` also builds cached `Path2D` objects ready for drawing.

The [corpus check](../web-benchmark/check-qvp.mjs) compares its result with the existing native engine's QVB1 exports. It passed on **all 604 pages**:

- 611,119 paths.
- 12,285,931 drawing commands, exactly matching.
- 65,319,828 float32 coordinate values, matching byte-for-byte.
- Path operation/coordinate ranges and fill rules, exactly matching.
- 77,432 word bounding boxes, exactly matching.

[Validation counts](../web-benchmark/results/comparisons/qvp-decoder-validation.json). These checks validate the rendering subset against the release corpus. The SDK does not implement the full native API. It reads additional metadata to navigate the file, but exposes only the word metadata needed by lightweight readers. The parser rejects malformed lengths and truncated sections. The source codec gives us a reference implementation for differential fuzzing, but that fuzzing has not been run yet.

Browser pixel comparisons also passed on pages 1, 42, and 585 at 1×, 2×, and 4×, including a word highlight: nine comparisons with identical output between direct QVP decoding and the existing QVB1 loader.

## Measured browser preparation cost

Both formats were fetched before measurement. The measured interval covers decoding/validation and construction of the same SDK `Path2D` objects. It excludes network transfer, HTTP decompression, drawing, and physical presentation. The direct QVP parser performs section, count, reference, numeric, and decoded-size checks; the benchmark-only QVB1 parser checks its magic and final length, so the difference also includes unequal validation work. Each page/format received 20 warmup iterations and 100 measured iterations; page order rotated and format order alternated. Chrome 152, M3 Max, non-isolated localhost context. These are exploratory warm desktop measurements of these implementations, not phone results or optimized lower bounds.

| Page | Direct QVP median | Expanded QVB1 median | Difference between medians |
| --- | ---: | ---: | ---: |
| 1 | 1.0 ms | 0.2 ms | 0.8 ms |
| 42 | 3.1 ms | 0.5 ms | 2.6 ms |
| 585 | 3.5 ms | 0.8 ms | 2.7 ms |

[Results](../web-benchmark/results/comparisons/qvp-browser-comparison.json) and [browser comparison code](../web-benchmark/compare-qvp.js).

**For this SDK and device, offline expansion saves roughly 0.8–2.7 ms of preparation on the sampled pages.** It increases independently Brotli-compressed assets across the full corpus from 37.24 MB to 135.96 MB. On page 42, the extra 159,587 compressed bytes correspond to approximately 128 ms of transfer at a steady 10 Mbps, ignoring latency and HTTP decompression. That is a bandwidth calculation, not a measured network-load result.

The evidence favors loading QVP directly instead of shipping the expanded format for uncached web loads. Mobile measurements, first-load/JIT behavior, HTTP decompression, and adversarial-input fuzzing remain to be checked. There is no inherent drawing-speed benefit to the expanded format: both produce the same curves for the same renderer.

## Comparison with the QCF4 page fonts

Muhaffidh and Almushaf both use the same QCF4 page data and WOFF2 files. A page is about 148–170 positioned glyph spans and normally needs one page font; page 585 also uses the separate basmalah font. The benchmark constructs `FontFace` objects from already-fetched bytes, inserts the apps' actual page markup at 355 × 574 CSS pixels, forces every glyph's layout, and rasterizes those positioned glyphs into the same 1170 × 1860 Canvas backing surface used for QVP. It excludes network transfer.

Chrome 152 on an M3 Max:

| Page | WOFF2 needed | Font construction/load | HTML + forced layout | First font raster | QVP decode + `Path2D` | First QVP draw |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 42 | 824 KB | 16.1 ms | 2.8 ms | 13.4 ms | 3.2 ms | 2.4 ms |
| 300 | 746 KB | 15.2 ms | 2.6 ms | 12.8 ms | 3.2 ms | 2.5 ms |
| 585 | 978 KB across 2 fonts | 20.5 ms | 3.0 ms | 13.1 ms | 3.7 ms | 2.8 ms |

Once raster caches were warm, the font proxy took 7.9–8.5 ms per repaint and QVP took 1.9–2.2 ms. The equivalent run through Almushaf was very close, as expected from its identical data and font files.

These phases should not be added into a universal page-load number. A QCF4 font covers a range of pages and can be reused after the first page that needs it; browser-level caching may also have helped repeated `FontFace` construction in this test. The Canvas text pass isolates glyph rasterization but does not measure DOM paint or physical presentation. The result says that a cold page-font parse/raster is material on this desktop and that direct QVP was faster in this controlled proxy. It does not establish mobile performance.

[Muhaffidh results](../web-benchmark/results/comparisons/font-browser-comparison.json), [Almushaf cross-check](../web-benchmark/results/comparisons/font-browser-comparison-almushaf.json), and [benchmark code](../web-benchmark/compare-font.mjs).

## Supplemental comparison with Muhaffidh's SVG renderer

Muhaffidh does not load prebuilt SVG page files. Its `dk-medina1441-hafs` mode loads a shared font and HarfBuzz WASM, shapes Quran text, and constructs 7–15 line SVGs in the DOM. To estimate a static SVG alternative, we serialized that generated page DOM and measured the browser recreating it from markup. The source was Muhaffidh commit `3e5ce60f7fca57800912a2c83c23c84abbce7da8`; all relevant app source, dependency, font, and WASM paths matched that commit and are covered by an aggregate content hash.

Chrome 152 on the same M3 Max, at a 390 × 620 CSS-pixel surface:

| Page | QVP decode + cached `Path2D` | Detached SVG DOM parse | SVG parse + attach + forced layout | Warm DigitalKhatt generation |
| --- | ---: | ---: | ---: | ---: |
| 1 | 1.0 ms | 1.6 ms | 3.3 ms | 10.9 ms |
| 42 | 3.1 ms | 6.4 ms | 13.5 ms | 35.3 ms |
| 585 | 3.5 ms | 6.5 ms | 13.7 ms | 37.8 ms |

For the two normal 15-line pages, detached DOM creation from the generated SVG markup took **6.4–6.5 ms**, around **1.9–2.1×** the QVP decoder plus `Path2D` construction. Attaching the SVG DOM and reading every line group's geometry took **13.5–13.7 ms**, around **3.9–4.4×** QVP preparation. Paint was not forced. Muhaffidh's full warmed page completion latency was **35–38 ms**; this includes the renderer's intentional animation-frame yields as well as shaping and SVG construction. Network transfer was excluded throughout.

The serialized page markup was 2.08–2.09 MB raw and 96–114 KB with Brotli quality 11 for pages 42 and 585. Their QVP files were 142–146 KB raw and 63–66 KB with the same Brotli setting. The SVG was therefore about 14× larger raw and 1.5–1.7× larger after Brotli in this sample. Muhaffidh's markup repeats glyph path strings; a purpose-built SVG exporter using `<defs>`/`<use>` and other optimization could improve these sizes and timings.

[Measurement details and results](../web-benchmark/results/comparisons/svg-browser-comparison.json). QVP and DigitalKhatt use different outline sources, and these are warm desktop results from three pages. They establish the behavior of these implementations, not a universal SVG decoding cost.

## Reproduce

From the repository root on macOS, put the release pages in `dist/pages` (or set `QVP_PAGES_DIR`), build the current engine, export references, and compare every page:

```sh
bash web-benchmark/export.sh /tmp/qvp-reference {1..604}
node web-benchmark/check-qvp.mjs /tmp/qvp-reference
python3 -m http.server 4186 --bind 127.0.0.1
```

Open `http://127.0.0.1:4186/` in a browser and run this in its developer console:

```js
const { compareQvp } = await import('/web-benchmark/compare-qvp.js')
await compareQvp()
```

The browser comparison uses the three existing reference fixtures and fetches their original QVP files from `qvp.quran.ws`. Pass a different `qvpBase` to `compareQvp()` when testing another release. The native engine is needed only to regenerate the reference exports, not to run the direct browser decoder.

To reproduce the Muhaffidh comparison, start its Vite development server in one terminal, then run the Playwright benchmark from this repository in another:

```sh
pnpm --dir ../../muhaffidh dev --host 127.0.0.1 --port 4187
node web-benchmark/compare-svg.mjs ../../muhaffidh http://127.0.0.1:4187/ web-benchmark/results/comparisons/svg-browser-comparison.json 'Apple M3 Max'
```

The runner uses Muhaffidh's installed Playwright dependency and the system Chrome channel. It records the source commit, relevant-input hash, benchmark hash, browser, summary results, and compressed sizes. Replace the final argument with the machine being measured, or omit it to record `null`.

The QCF4 benchmark additionally needs the required WOFF2 files in a local directory and the QVP static server shown above:

```sh
node web-benchmark/compare-font.mjs ../../muhaffidh http://127.0.0.1:4187/ /tmp/qcf4-fonts web-benchmark/results/comparisons/font-browser-comparison.json 'Apple M3 Max'
```
