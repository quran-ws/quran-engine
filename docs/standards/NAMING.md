# Naming

Each concept has one word, and the noun that owns a value says which flavour of the value
it is. Names carry no abbreviations and no prefix that encodes a type or an origin. The
same operation has the same name in every language.

## Identity

| what | name |
|---|---|
| the repository | `quran-engine` |
| the engine, the page format, the file extension | `qvp`, Quran Vector Page |
| the atlas format and extension | `qva`, Quran Vector Atlas |
| the organisation | `quran.ws` |
| Rust crates | `qvp-format`, `qvp-core`, `qvp-convert`, `qvp-ffi` |
| npm | `@quran.ws/engine`, `@quran.ws/qvp-react-native` |
| Kotlin / Android | `ws.quran.qvp` |
| Swift | `QvpKit` |
| Dart | `qvp_flutter` |

Expand QVP and QVA once, in the README and here. Nowhere else needs to.

## Quranic terms

The Quran.ws terminology standard fixes every Quranic word: `surah`, `ayah`, `juz`,
`hizb`, `rubu_al_hizb`, `rasm_uthmani`, `rasm_imlai`, `ayah_mark`, `waqf`, `sajdah`, and
the mark names from the source bundle's taxonomy (`fathah`, `hamzat_al_wasl`,
`omitted_alif`, `small_meem`, …). `.terminology.json` records what this repository has
adopted and every exception with its reason. Run the audit before adding a word.

## The owning noun carries the meaning

Never put a value's origin, type, unit or mode into its name. Put the value on the thing that
owns it and let the noun say the rest.

| value | lives on | name |
|---|---|---|
| the spacing of the mushaf's design grid | `page.grid` | `lineSpacing` |
| the spacing as printed on this page | `page` | `lineSpacing` |
| the multiplier the app asks for | layout options | `lineSpacing` |
| the spacing the layout produced | `layout` | `lineSpacing` |

Same word four times, and the call site reads as a sentence: the page's line spacing, the
layout's line spacing. In the C header the noun becomes the prefix mechanically:
`qvp_page_line_spacing`, `qvp_grid_line_spacing`, `QvpLayoutOptions.line_spacing`,
`QvpLayout.line_spacing`. Setting a ratio and reading back a length under one name is how
CSS `line-height` works, a pattern that web and mobile developers already use.

When a group of values needs a home, introduce a noun (`grid`), never a prefix (`nominal_`).

Never `lineHeight`: printed lines are not equally tall, and the name would promise a grid.

## Booleans

- State you read: `is_` or `has_`. `is_header`, `has_banner`, `is_exact`, `is_open`.
- An option you set: name the behaviour it controls, no prefix. `fill_height`,
  `keep_ayah_marks`, `prefer_exact`, `loose_match`.
- `xEnabled` only where "enabled" is the natural concept, which is a feature switch on a
  platform view: `zoomEnabled`, `selectionEnabled`. The engine and the C header have none.
- Never `is_` on an option. One width in the C header: `uint8_t`.

## The C header

- Functions: `qvp_<noun>_<verb>`. The noun is the object-model section the function belongs
  to (page, atlas, style, highlight, mask, reveal, layout, …). `qvp_highlight_add`,
  `qvp_atlas_page_of`, `qvp_style_remove`.
- Structs: `Qvp<Noun>`. Opaque handles are bare nouns (`QvpPage`, `QvpAtlas`). Snapshots of
  the five indexable elements are `Qvp<Element>Info` (`QvpWordInfo`, `QvpLineInfo`, …).
  Metadata records are plain nouns (`QvpSurah`, `QvpDivision`, `QvpRosette`).
- Constants: `QVP_<ENUM>_<VALUE>`, and a struct's discriminator field takes its enum's
  name: a `kind` field holds a `QVP_KIND_*`, a `decoration` field a `QVP_DECORATION_*`, a
  `division` field a `QVP_DIVISION_*`. One enum name is never reused for another axis.
- Full words. The only abbreviations are `x0 y0 x1 y1`, `n_` for counts and `cap` for a
  buffer capacity. So `decoration` not `decoration`, `index` not `index`, `word_index` not `wordIndex`,
  `offset_x` not `ox`, `view_x` not `viewX`, `line_number` not `line_number`.
- `surah, ayah, word` is always the key of a word. A page-local position is
  `_index` (0-based), a printed value is `_number` (1-based), and `page` is the mushaf
  page number.
- Array outputs keep the `(out, cap)` protocol and return the total count.

## Verbs, and what each one promises

| verb | promise |
|---|---|
| `load` / `free` | lifetime of an opaque handle |
| `get` (usually implicit) | read, no side effect |
| `set` | write one value |
| `add` / `remove` | a rule or highlight enters or leaves; `add` returns a handle |
| `move` | an existing handle points somewhere else |
| `clear` | remove everything of one kind |
| `find` | exact lookup by key. Returns none, never throws |
| `search` | text query. Returns matches |
| `_at(i)` | by index into the page's list |
| `_of(surah, ayah)` | the thing that contains this ayah |
| `_count` | how many. A plural noun is always a list, never a count |
| `mask` / `unmask`, `hide` / `show` | the memorisation and style subsystems' own pairs |

A new verb needs a row here first.

## Rectangles

- `bounds`: an extent you read (`crop_bounds`, `word_bounds_view`).
- `band`: the vertical strip of a line (`line_bands`, `word_bands`).
- `boxes`: rectangles the app draws (`highlight_boxes_view`, `mask_boxes_view`).
- `_view` on every result in viewport pixels. Everything else is page units.

## Across the wrappers

One operation, one camelCase name, identical in JavaScript, Kotlin, Dart, Swift and
TypeScript, derived from the C name by dropping `qvp_` and the noun that the receiver
already is: `qvp_highlight_add` → `page.highlightAdd` is wrong; `page.highlight(…)` is
right when the receiver is the page and the verb reads naturally, and the parity table
records the exact spelling per language. A platform's own additions (a SwiftUI view, a
Compose modifier) follow that platform's conventions and must not shadow an engine name.

## Pending renames

The rename list that applies this standard to today's header lives in
`docs/API-PARITY.md`. Before 1.0, renames ship without aliases, and the changelog carries
the old → new table.

## How it is checked

- `scripts/check-parity.py` reads `qvp.h` and fails on any symbol outside the
  `qvp_<noun>_<verb>` shape, any abbreviation from the banned list, any `is_` on an option
  field, and any wrapper method whose spelling differs from the parity table.
- `audit_terminology.py` (the Quran.ws terminology audit) runs in the same CI job.
