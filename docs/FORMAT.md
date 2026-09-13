# The page and atlas formats

This page specifies the two binary formats the engine reads, byte by byte, so that a
decoder can be written from it without the Rust source. The reference implementation is
`crates/qvp-format/src/codec.rs` (pages) and `crates/qvp-format/src/atlas.rs` (atlas);
`conformance/qvp1-lite.hex` is a small page written by the encoder, and
`web/lite.mjs` is a second decoder written against this specification.

## Conventions

- All integers are little-endian.
- `varint` is an unsigned LEB128 integer: seven bits per byte, low bits first, the high
  bit set on every byte but the last.
- `zigzag` is a signed integer stored as a varint after the zigzag mapping
  `(v << 1) ^ (v >> 31)`, so small negative and positive values both stay short.
- `opt` is a varint that stores an optional 16-bit index as `index + 1`, with 0 meaning
  absent.
- Coordinates are page units, the printed page's own space with the origin top-left and y
  down, quantised: a stored integer is the coordinate multiplied by `quant` (100 by
  default, so one unit is 0.01 page units).
- Ids and indices run from 0. `0xFFFF` and 255 mean "none".

## QVP1: one page

A page file is a 64-byte header followed by six sections whose lengths the header gives.

### Header (64 bytes)

| offset | size | field |
|---|---|---|
| 0 | 4 | magic `QVP1` |
| 4 | u16 | format version, 1 |
| 6 | u16 | `quant`, the quantisation factor |
| 8 | u16 | page number |
| 10 | u16 | flags, reserved, 0 |
| 12 | f32 | page width in page units |
| 16 | f32 | page height in page units |
| 20 | u16 | number of lines |
| 22 | u16 | number of ayah fragments |
| 24 | u16 | number of words |
| 26 | u16 | number of decorations |
| 28 | u32 | number of paths |
| 32 | u16 | number of strings |
| 34 | u16 | number of glyph outlines |
| 36 | u16 | number of glyph instances |
| 38 | u16 | padding, 0 |
| 40 | 6 × u32 | the byte length of each section, in file order |

The sections follow the header back to back, in this order: tables, opcodes, close
indices, x deltas, y deltas, strings.

### Section 1: tables

The records of every table, one table after another, each field delta-coded against the
previous record where the field is monotonic. Fields that a decoder can recompute are not
stored: bounding boxes (except a glyph's, see below), a path's origin, and byte offsets
into the opcode stream.

**Lines**, one record per printed line:

| field | coding |
|---|---|
| line number | u8 |
| word count | varint |

The first word of a line is the running total of the counts before it.

**Ayah fragments**, one record per piece of an ayah on this page (an ayah spans lines, so
it has one fragment per line):

| field | coding |
|---|---|
| surah | zigzag delta from the previous fragment |
| ayah | zigzag delta from the previous fragment |
| fragment index, fragment count | u8, u8 |
| flags | u8: bit 0 juz starts here, bit 1 hizb, bit 2 `rubu_al_hizb`, bit 3 nisf |
| word count | varint |
| ayah-mark decoration | opt |
| `rubu_al_hizb` number | varint: 1 to 240 when a division starts at this ayah, else 0 |

The juz, hizb and nisf numbers derive from the `rubu_al_hizb` number:
`juz = (r − 1) / 8 + 1`, `hizb = (r − 1) / 4 + 1`, `nisf = (r − 1) / 2 + 1`.

**Words**, one record per word in reading order:

| field | coding |
|---|---|
| first path | zigzag delta from the previous word's end (`first + count`) |
| surah, ayah, word | zigzag deltas from the previous word |
| line index, ayah-fragment index | zigzag deltas from the previous word |
| text, `rasm_imlai`, qpc, rasm, search | five `opt` string indices; only the first is set in production data, the rest come from the words sidecar |
| path count | varint |

**Paths**, stored column by column so that like bytes sit together:

| column | coding |
|---|---|
| kind, for every path | u8 each: 0 body, 1 mark, 2 ayah number, 3 ayah-mark ornament, 4 header ink, 5 ornament, 6 page number, 7 running head, 255 other |
| mark, for every path | u8 each: the mark id, 0 for none (`qvp_name` table `mark` lists them) |
| family, for every path | u8 each: 0 none, 1 diacritic, 2 tanwin, 3 dots, 4 waqf, 5 sifr, 6 sajdah, 7 reading sign |
| flags, for every path | u8 each: bit 0 even-odd fill, bit 1 stacked form, bit 2 staggered form, bit 3 standalone mark, bit 4 glyph instance, bit 5 drawn twice in the artwork |
| run length or instance, for every path | varint: for a glyph instance (bit 4) the instance index; otherwise the number of drawing operations in the path's run |
| glyph instance boxes | for every glyph-instance path, in order: zigzag x0, y0, width, height |

A glyph instance's box is stored because it was measured on the source coordinates under
the instance transform; recomputing it from the quantised outline can differ by a unit.

**Decorations**, one record per non-word element (ayah marks, surah names, basmalah,
division marks, sajdah marks, page numbers, running heads):

| field | coding |
|---|---|
| first path | zigzag delta from the previous decoration's end |
| kind | u8: 0 ayah mark, 1 surah name, 2 basmalah, 3 division mark, 4 sajdah mark, 5 page number, 6 running head, 255 other |
| surah | zigzag delta from the previous decoration |
| ayah | varint |
| text | opt string index |
| path count | varint |
| line | opt line index |

**Glyph outlines**, one record per shared outline:

| field | coding |
|---|---|
| run length | varint: the number of drawing operations |
| box | zigzag x0, y0, width, height, in glyph space |

**Glyph instances**, one fixed-width record per placement of an outline:

| field | coding |
|---|---|
| glyph | u16 index into the outlines |
| a, b, c, d, e, f | six f32: the affine transform, page = M · glyph, with glyph coordinates already divided by `quant` |

### Sections 2 to 5: the drawing operations

Every path that is not a glyph instance, then every glyph outline, contributes one run of
drawing operations, in that order. The runs are concatenated and split into four streams.

- **Opcodes** (section 2): two bits per operation, four per byte, low bits first. 0 move,
  1 line, 2 quadratic curve, 3 cubic curve. A close operation is not in this stream.
- **Close indices** (section 3): a varint count, then one varint per close, each a delta
  from the previous value. A value `n` means a close after the first `n` operations of the
  whole stream.
- **X deltas** (section 4) and **y deltas** (section 5): one zigzag per point, the points
  of each operation in order (one for move and line, two for quadratic, three for cubic),
  each coordinate a delta from the previous point. A run's first point is a delta from the
  previous run's *first* point, not its last; a glyph outline's first point is a delta
  from 0,0.

A decoder reads `run length` operations for each path or outline, sets the path's origin
to its own bounding box minimum, and stores the path's offset into the operation stream
as it goes.

### Section 6: strings

For each string: a varint byte length, then that many bytes of UTF-8.

## QVA1: the atlas

One small file for the whole mushaf. Everything after the magic is a varint or a
length-prefixed string.

| field | coding |
|---|---|
| magic | `QVA1` |
| page count, surah count, `rubu_al_hizb` count | three varints |
| pages, sorted by page | per page: page, first surah, first ayah, last surah, last ayah, word count (six varints) |
| surahs, sorted by number | per surah: number, first page, ayah count, place (0 makkah, 1 madinah, 255 unknown), then the Arabic, Latin and English names, each a varint length and UTF-8 bytes |
| `rubu_al_hizb` boundaries, sorted | per boundary: `rubu_al_hizb` number, surah, ayah, page (four varints) |

## Versioning

The page format version is the u16 at offset 4. A reader rejects a version it does not
know. Changing the encoding means a new version, a full rerun of the identity gate over
all 604 pages, and a new data release (`docs/standards/RELEASING.md`).
