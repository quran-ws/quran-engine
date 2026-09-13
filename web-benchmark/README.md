# QVP browser rendering benchmark

Compare cached Canvas 2D, direct Canvas 2D as a control, and a batched WebGL2 triangle renderer using the actual outlines in this package. No browser WebAssembly or SVG is involved.

[Read the measured results](RESULTS.md).

## Run

```sh
cd web-benchmark
pnpm install --frozen-lockfile
pnpm start
```

Open http://127.0.0.1:4173. The three exported binary fixtures are included. “Run full benchmark” first checks visual output, then runs 27 trials (three pages × three renderers × three repetitions), each with three workloads, a complete 90-frame warmup trajectory and 90 measured frames. Keep the tab visible and avoid other intensive work. Results are saved locally under `results/` and can also be downloaded.

The default surface is 390 × 620 CSS pixels at DPR 3 (1170 × 1860 backing pixels). This does **not** emulate phone CPU or GPU performance. To measure an actual phone on a trusted local network, start with `HOST=0.0.0.0 pnpm start` and open this computer's LAN address on the phone. The server is an unauthenticated local development tool; its `/results` endpoint writes benchmark JSON. HTTP over LAN may not provide cross-origin isolation/high-resolution timers, unlike localhost; the result records isolation status.

The other profiles use 720 × 1100 at DPR 2 or a 390 × 620 surface at the browser's real DPR. Profiles change rendering resolution, not hardware speed. This is a fixed backing-surface benchmark: the visible canvas can be CSS-downscaled on narrow screens. Each setup records its actual CSS display dimensions; the curve tolerance refers to backing pixels, not necessarily physical screen pixels.

## Data

`pnpm export` regenerates the fixtures on macOS using the repository's current Rust engine and `clang`. Put the release page files in `dist/pages`, or set `QVP_PAGES_DIR` to a directory containing `001.qvp` through `604.qvp`. The exporter scans all pages and selects page 1 (opening), explicitly selected example page 42, and page 585 (largest decoded coordinate count in the original run). This is not a statistical sample or a claim that page 42 represents the other pages. Other operating systems can run the browser benchmark with the included fixtures.

The checked-in September 2026 result predates this benchmark's move into `quran-engine`. Its raw result embeds the engine identity, selection metrics, and SHA-256 hashes captured by the original repository at measurement time. New runs record hashes for the benchmark sources, dependency lockfile, manifest, and exported fixtures in this repository.

The native engine decodes `.qvp` **offline**. The exported `QVB1` fixtures contain the exact decoded float coordinates and path/word associations. Their simple typed-array parsing does **not** measure original QVP decompression or prove that a JavaScript QVP decoder will be fast. Downloads are excluded from renderer setup measurements, and these expanded files are not a proposed production replacement for QVP.

`QVB1` is little-endian: 32-byte header (`QVB1`, float32 width/height, uint32 path/op/coordinate/word counts and page number), `n_paths × 8` uint32 path table, uint8 operations padded to four-byte alignment, float32 coordinates, then `n_words × 4` float32 bounding boxes. See `export.c` and `geometry.js`.

| Page | Paths | Coordinate floats | Original QVP bytes | Exported bytes | Exported Brotli bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 268 | 42,054 | 38,252 | 186,404 | 64,043 |
| 42 | 1,061 | 106,220 | 141,970 | 480,708 | 222,505 |
| 585 | 1,087 | 148,334 | 145,949 | 660,468 | 295,008 |

Brotli sizes use Node's default `brotliCompressSync`. The development server serves uncompressed data, outside the measured setup interval.

## Rendering and measurement

- **Canvas cached:** build `Path2D` objects once; rasterize transparent ink into one viewport-sized offscreen surface; composite a band behind the cached ink. Zoom previews scale the bitmap. Sharp zoom refreshes the cache from curves at each zoom level.
- **Canvas direct:** use the same cached `Path2D` objects, repainting all paths each frame. This separates the benefit of caching from the choice of graphics API.
- **WebGL:** flatten quadratic/cubic curves adaptively in a persistent worker, tessellate using `libtess` with each path's original nonzero/even-odd rule, then upload one unindexed triangle buffer. Draw ink in one batch and the highlight rectangle in another. Request native antialiasing; record the actual GPU/backend and sample count. The curve tolerance is 0.2 backing pixels at the maximum tested 4× zoom. GPU rendering therefore approximates the curves; arbitrary deeper zoom requires finer geometry.

Each trial constructs a fresh renderer under an explicitly **warm setup** regime: visual validation exercises every page/mode, then every mode receives an additional untimed setup/dispose before idle calibration. Page, renderer and workload orders rotate across repetitions, and the actual sequence is recorded. Two idle frames follow context disposal. Setup includes buffer copy/transfer, preparation, paths/context/buffer creation, the initial draw, and a one-pixel readback to force completion. It excludes asset download, offline QVP decoding, and cold app/library startup. The readback is a setup diagnostic, never inside the animated measurement loops.

The three workloads use the same positions and transforms: 1×→4×→1× zoom with horizontal movement; continuously sharp zoom; and a background band moving through word bounding boxes. Cached Canvas's preview zoom is temporarily softer than the geometry renderers. The highlight measurement does not cover changing ink colours, layout/reflow, hit-testing, or selection logic. Layered HTML canvases or GPU texture caching could further optimize some workloads; they are not implemented here.

CPU measurements cover **JavaScript command submission**, not completed rasterization/GPU execution. `requestAnimationFrame` intervals measure callback pacing, not physical display presentation. The late-interval threshold is 1.5× the idle median sampled before the run. Both measurements matter: lower CPU submission times alone do not prove smoother display. Localhost uses COOP/COEP to enable more precise clocks where supported.

Memory figures report explicit cached source buffers, the live decoded geometry copy, uploaded mesh buffers and nominal RGBA surfaces. They exclude browser-native `Path2D` storage, JS object overhead, driver copies, multisampling, swapchains, and transient allocations. Do not interpret them as total process/GPU memory or assume the smaller listed buffer means lower total memory. Quantiles use the sorted observation at `floor((n-1)*q)`. Pooled frame quantiles are descriptive; adjacent frames are correlated, and these are not confidence intervals or significance tests. Three repetitions on one device support exploratory conclusions for this sample.

## Visual verification

Before each run, automatic validation compares direct Canvas against cached Canvas in sharp mode and WebGL for every selected page at 1×, 2× and 4×, with and without a middle-word highlight: 36 cases by default. A failed case prevents measurement. Thresholds are stored with each case: mean RGB difference ≤0.25/255 for cached Canvas or ≤2/255 for WebGL, and at most eight missing/extra interior pixels. These are regression guardrails allowing different antialiasing, not certification thresholds.

“Compare pixels” additionally displays direct Canvas and WebGL side by side. The metrics include average RGB difference, differences within a consistently defined ink union, and a heuristic count of missing/extra interior pixels using a cross of neighboring reference pixels. Inspect the images as well; a zero interior count is not a proof that every fine detail is identical. Both consumers share the QVB export and parser; they cannot detect a common error relative to a native-rendered golden. No claim of pixel identity with the native iOS app is made.

The original coordinates and fill rules are preserved through export. GPU tessellation retains holes and handles overlapping contours. The fixtures are a selected exploratory sample, not validation of the entire mushaf.

## Sources

- [MapLibre rendering architecture](https://github.com/maplibre/maplibre-gl-js/blob/main/ARCHITECTURE.md): workers prepare geometry for GPU rendering.
- [Canvas optimization guidance](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Optimizing_canvas): cache static work and separate changing content.
- [WebGL performance guidance](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices): batching, avoiding blocking readbacks in animation loops, and memory limitations.
- [libtess](https://github.com/brendankenny/libtess.js): JavaScript tessellation with winding rules; pinned version 1.2.2, SGI-B-2.0 license.
