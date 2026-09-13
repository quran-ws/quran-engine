# QVP rendering benchmark results

For these three pages on this M3 Max Mac, cached Canvas is the preferred starting point: substantially cheaper warm page preparation and essentially the same measured frame pacing as WebGL. WebGL reduces JavaScript submission cost during continuous sharp zoom, but that did not translate into a pacing advantage in this experiment. Direct Canvas also kept up and avoided the extra cache refresh step during sharp zoom.

This is an exploratory desktop result on a phone-sized surface, not a physical-phone result or a statistical sample of all 604 pages. It compares these implementations, not the maximum performance possible with either API. Moving GPU tessellation offline or adding GPU texture caching could change the tradeoffs.

Run: 2026-09-12T04:42:11.577Z. Raw data: [results/2026-09-12T04-44-17.818Z.json](results/2026-09-12T04-44-17.818Z.json).

Browser: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36. Cross-origin isolated: true.

Surface: 390 × 620 CSS pixels at DPR 3. 3 repetitions, 90 measured frames per workload per trial. Idle rAF median: 8.325 ms.

CPU values below are draw-submission times, not GPU execution times. rAF values describe callback pacing, not physical presentation. Quantiles pool all sampled pages and repetitions.

| Renderer | Workload | CPU p50 ms | CPU p95 ms | rAF p95 ms | Late intervals |
| --- | --- | ---: | ---: | ---: | ---: |
| Canvas cached | preview_zoom | 0.030 | 0.035 | 9.230 | 2/810 |
| Canvas cached | sharp_zoom | 0.275 | 0.345 | 9.295 | 0/810 |
| Canvas cached | highlight | 0.030 | 0.040 | 9.235 | 0/810 |
| Canvas direct | preview_zoom | 0.195 | 0.230 | 9.170 | 0/810 |
| Canvas direct | sharp_zoom | 0.200 | 0.235 | 9.220 | 1/810 |
| Canvas direct | highlight | 0.195 | 0.215 | 9.280 | 2/810 |
| WebGL triangles | preview_zoom | 0.025 | 0.035 | 9.130 | 0/810 |
| WebGL triangles | sharp_zoom | 0.025 | 0.035 | 9.200 | 2/810 |
| WebGL triangles | highlight | 0.030 | 0.040 | 9.140 | 0/810 |

Setup includes preparation, construction and a one-pixel readback of the first draw. Downloads and original QVP decoding are excluded.

| Page | Renderer | Setup median ms | Worker mesh median ms | Nominal extra raster MiB | Uploaded mesh MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | Canvas cached | 4.525 | 0.000 | 8.30 | 0.00 |
| 1 | Canvas direct | 5.170 | 0.000 | 0.00 | 0.00 |
| 1 | WebGL triangles | 28.920 | 21.230 | 0.00 | 0.56 |
| 42 | Canvas cached | 7.075 | 0.000 | 8.30 | 0.00 |
| 42 | Canvas direct | 6.100 | 0.000 | 0.00 | 0.00 |
| 42 | WebGL triangles | 78.235 | 68.940 | 0.00 | 1.78 |
| 585 | Canvas cached | 7.630 | 0.000 | 8.30 | 0.00 |
| 585 | Canvas direct | 6.425 | 0.000 | 0.00 | 0.00 |
| 585 | WebGL triangles | 78.360 | 70.830 | 0.00 | 2.23 |

Buffer figures exclude driver/MSAA/swapchain allocations, native Path2D storage, and JS/transient allocations; these are not total memory measurements.

GPU backend: ANGLE (Apple, ANGLE Metal Renderer: Apple M3 Max, Unspecified Version).

See [README.md](README.md) for reproduction, visual checks, and workload limitations.

All modes were warmed symmetrically. Every workload ran its complete 90-frame trajectory untimed before the measured trajectory. Page, renderer and workload orders were rotated. The displayed surface was 390 × 620 CSS pixels. There were 27 trials, 81 workloads and 7,290 measured frames.

Automatic visual validation passed all 36 cases: pages 1, 42 and 585 at 1×, 2× and 4×, cached Canvas and WebGL against direct Canvas, with and without highlights. The heuristic check detected zero missing or extra interior pixels. Maximum mean RGB error was 1.511/255; antialiased edges differ. These checks compare QVB consumers and do not certify pixel identity with native iOS rendering. The raw benchmark result linked above contains the visual metrics.

The result embeds the engine identity and SHA-256 hashes captured at measurement time. This report uses only the final run linked at the top.
