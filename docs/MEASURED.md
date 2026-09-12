# Measured, and the rough edges

Numbers you can reproduce on your own hardware, and the things that are known to be wrong.
What the engine *is* and how to use it lives on <https://quran.ws/docs/reference/quran-engine>;
this file is for people working on the engine itself.

## Measured

Page 042, release build, Apple silicon — run it yourself and expect different
numbers on different hardware:

```sh
cargo run -p qvp-core --release --example bench -- quran-engine-pages-hafs-kfgqpc/042.qvp
```

| | |
|---|---|
| page load + geometry | 2.4 ms |
| exact hit-test | 1.4 µs |
| gap-aware hit-test | 0.15 µs |
| full display list, 2 style rules | 0.24 µs |
| search `الله` over the page | 86 µs |
| six-line ayah highlight, bands included | 65 µs |

The reason for the engine is not this table. It is that a fully split page is
hundreds of kilobytes of vector paths, and a phone cannot hold 604 of them in a
DOM and stay responsive.

## Known rough edges

- `revealStart()` in `web/qvp.js` throws `ReferenceError: markers is not defined`
  on every call — the greyed-page reveal is unreachable from the web wrapper. The
  rest of the memorisation surface (`mask`, `revealNext`, `revealWord`, `unmask`,
  `maskHidden`, `maskBoxes`) works.
- `surahs()` returns a `bannerDeco` field that `docs/API.md` does not list; it
  indexes into `page.decos`.
- No package is published, on any registry.
