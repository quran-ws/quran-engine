# API design

The test every public member must pass:

> A developer who has never seen this code should guess how to use it correctly, without
> reading the implementation or the documentation.

## The rules

- **Design from the caller's side.** Name a method for the result the app needs
  (`highlight`, `search`, `mask`), never for how the engine does it (`resolve`, `tick`,
  `styled`).
- **A small verb vocabulary, used everywhere.** load, free, get, set, add, remove, move,
  clear, find (exact key), search (text), mask, unmask, hide, show. A new verb needs a row in
  `NAMING.md` first.
- **Read like a sentence at the call site.** `page.highlight(ayah)`, `layout.lineSpacing`,
  `atlas.pageOf(surah, ayah)`.
- **Short common case, sensible defaults.** The common case is three calls: load a page,
  render it, hit-test a point. Every option defaults to the printed mushaf, and a hit test
  needs no options object.
- **Progressive disclosure.** Styles, transitions, masks, reveal, crop and the atlas are
  reachable but invisible until needed. The first screen of a package README never mentions
  them.
- **Expressive variants over flags.** `hitTestExact` not `hitTestExact(exact: true)`.
  `findWord` returns nothing when absent. A variant that throws says so in its name.
- **Hide the machinery.** Handles, layers, path tables, string lifetimes and scratch buffers
  are marshalling. They never appear on a wrapper's public surface.
- **Predictable pairs.** `add`/`remove`, `mask`/`unmask`, `load`/`free`,
  `select`/`selection` (the verb writes, the noun reads). Similar concepts take the same
  shape.
- **Distinguish meaning by name, never by argument.** `hitTestExact` vs `hitTestExact`, `find`
  vs `search`, `pageOf` (which page contains this) vs `pageAt` (the page at this index).
- **No abbreviations, no jargon, no generic names.** `decoration` not `decoration`, `lineSpacing`
  not `pitch`. Never `process`, `handle`, `data` or `info` as a whole name.
- **Fewest parameters.** Anything with an obvious default is optional. More than three
  parameters means an options object.
- **Write the guess first.** Before adding a public member, write down what a developer
  would guess it is called and what it returns. That guess is the name unless `NAMING.md`
  forbids it.
- **Optimise for the call site** even when the wrapper or the engine gets more complex for
  it.

## Layers

| layer | job | guessable? |
|---|---|---|
| `crates/qvp-core` | every computation: geometry, hit testing, layout, styles, highlights, selection, masks, search, crop, atlas | n/a, internal |
| `crates/qvp-ffi/include/qvp.h` | the single contract; flat and complete | no, and it need not be |
| `web/qvp.js` | the reference wrapper. Every other wrapper mirrors its names | yes |
| Kotlin, Dart, Swift, TypeScript packages | the same surface in each language's idiom | yes |

The rules above apply to the wrappers. The header follows `NAMING.md` and the ABI
conventions below.

## Object model

The header's sections are the object model. Every function belongs to exactly one and
carries its noun as prefix.

page (load, info, geometry, elements) · metadata (surahs, divisions, marks) · text and
search · hit testing · layout · style · colours · highlight · selection · mask · reveal ·
crop · atlas · names

## No formulas in wrappers

A wrapper may do arithmetic for two things only: converting engine output to platform
units, and tracking gesture state (pan, pinch, scroll). Anything else, such as a fit
scale, a clamp, box mathematics, index mathematics or colour mathematics, is engine work.
It belongs in the core, exposed through one symbol, and the wrapper calls it.

When you find such a computation in a wrapper, move it to the core and add a scenario in
`conformance/scenarios/`. Then no wrapper can reimplement it differently again. The fit
scale and the centred offsets were once computed six different ways across the wrappers;
they are now `QvpLayout.fit_scale`, `fit_x`, `fit_y`, and `conformance/scenarios/layout.json`
holds the engine's answers that every wrapper test replays.

## The three tiers of a feature

| tier | rule |
|---|---|
| core behaviour | changed once in Rust. Every platform receives it with the next native build. Nothing to mirror |
| engine API (a C symbol) | bound in all five wrappers under the parity table's name, or listed there as a declared gap with an issue number. CI fails on undeclared drift |
| platform convenience | per platform, in its idiom, documented in the package README and listed in the parity table as a gap for the others. Must not compute anything the engine can compute |

A feature is done when it meets its tier's rule.

## The lite decoder

`web/lite.mjs` is the only sanctioned reimplementation of the page format, for Canvas-only
readers that must not load wasm. The conformance fixtures that the Rust codec generates gate it, and it never grows engine logic: no layout, no hit testing beyond word bounds.

## Consumability

Every package must build outside this repository from its published form. A default path
that only resolves inside the monorepo is a bug.

## C ABI conventions

- Coordinates are page units unless the name says `view`.
- Colours are `0xRRGGBBAA`. Alpha 0 means hidden or "leave alone".
- `QVP_NONE` is the absent index. A function that can fail returns `0`/`-1`/`NULL` as its
  header comment states. Every entry point catches a panic in the engine and returns that
  error value; nothing unwinds across the boundary. On wasm there is no unwinding, so a
  panic traps there.
- Strings return as `QvpStr` (pointer, length, UTF-8, not NUL-terminated) and live until the
  next string-returning call on the same thread.
- Array outputs take `(out, cap)` and return the total count.
- Booleans are `uint8_t`.
- `qvp_version()` is the engine version; `qvp_format_version()` is the page format's.

## Evolution

- Pre-1.0: a rename ships without an alias. The changelog carries the old → new table.
- From 1.0: a renamed or removed symbol keeps a deprecated alias for one minor version.
  Adding symbols is a minor bump. Removing or renaming is a major bump. A wrapper declares
  the page format version it reads.

## How it is checked

- `scripts/check-parity.py` generates `docs/API-PARITY.md` and fails on any symbol that a
  wrapper neither binds nor declares as a gap.
- The wrapper test suites run `conformance/scenarios/` and fail on any numeric divergence
  from the Rust golden output.
- Pull requests state the tier of every change (template).
