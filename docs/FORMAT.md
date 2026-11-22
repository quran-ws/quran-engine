# The page, atlas, ornament and letter formats

This page specifies the four binary formats the engine reads, byte by byte, so that a
decoder can be written from it without the Rust source. The reference implementation is
`crates/qvp-format/src/codec.rs` (pages), `crates/qvp-format/src/atlas.rs` (atlas) and
`crates/qvp-format/src/ornaments.rs` (ornaments) and `crates/qvp-format/src/letters.rs`
(letter layers);
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

### What a reader rejects

The reference decoder checks these before it indexes one table by another, and returns
an error that names the table. A writer keeps them.

- `quant` is at least 1.
- Every count fits its section. A record takes at least 2 bytes (line), 8 (ayah
  fragment), 12 (word), 5 (path, with its column bytes), 7 (decoration), 5 (glyph
  outline) and 26 (glyph instance), and a string takes at least 1 byte.
- A delta-coded field lands inside its type: a u16 field in 0 to 65535, a path index in
  0 to 2^32 − 1.
- Every coordinate and every corner of a stored box has an absolute value below 2^30
  quantised units, so the difference of two of them fits an i32.
- A page with ayah fragments has at least one line.
- A line's and an ayah fragment's word run ends inside the words table.
- A word's path run ends inside the paths table, its line and ayah-fragment indices name
  a record, and each of its five string indices names a string or is absent.
- A decoration has at least one path, its run ends inside the paths table, and its line
  and its text name a record or are absent.
- An ayah fragment's ayah-mark decoration names a decoration or is absent.
- A glyph-instance path's instance index names an instance, and an instance's glyph index
  names an outline.
- A page with paths has at least one line.
- The run lengths add up to at most four operations per opcode byte and at most one per
  byte of the x deltas and of the y deltas. The close count is at most one per byte of its
  section, every close index names an operation, and every stream ends exactly where its
  section ends.

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

A reader rejects a varint that does not fit a u16, a `rubu_al_hizb` number of 0, and
counts whose records cannot fit in the bytes that follow them.

## Versioning

The page format version is the u16 at offset 4. A reader rejects a version it does not
know. Changing the encoding means a new version, a full rerun of the identity gate over
all 604 pages, and a new data release (`docs/standards/RELEASING.md`).

`QVL1` has its own version, the u16 at offset 4 of a letter layer. It does not change
with the page format version, and a change to one format does not change the other.

## QVO1: an ornament set

An ornament set holds the ayah medallions, surah bands and page frames of other mushafs
and fonts, converted from the `quran-ws/quran-assets` catalogue. The reference
implementation is `crates/qvp-format/src/ornaments.rs`. Coordinates are asset units, the
asset's own viewBox space, quantised by `quant` (1000 by default). A `str` is a varint
length followed by that many UTF-8 bytes. A `f32` is four bytes, little-endian.

| field | type |
|---|---|
| magic | `QVO1` |
| quant | u16 |
| ornament count | varint |
| ornaments | one record each, below |
| path count | varint |
| paths | one record each, below |
| opcode length | varint |
| opcodes | the QVP1 opcode encoding with origin 0,0 |

The opcodes are those of QVP1: move, line, quadratic and cubic. There is no arc operation.
The converter writes an SVG arc, circle, ellipse or rounded rectangle as cubics.

**Ornament**: `kind` u8 (0 ayah mark, 1 surah header, 2 page frame), `lineage` u8 (0 scan,
1 font), `style` str, `riwayah` str, `license_id` str, `license_status` str,
`redistributable` varint (0 or 1), `attribution` str, `part count` varint, then per part
`name` str, `color` u32 (`0xRRGGBBAA`), `stroke` varint; then one **asset**; then
`has_slices` varint, and when 1: `corner_w`, `corner_h`, `repeat_h`, `repeat_v` as f32,
`rotate` varint, and three assets (corner, horizontal edge, vertical edge). `rotate` 1
turns the corners: the top-right and bottom-left are 180° turns of the corner and the
bottom-right is a plain copy. `rotate` 0 mirrors them. Rotate turns the corners only. The
straight runs are always mirrored, as the catalogue assembles them.

**Asset**: `view_box` 4 × f32 (`x, y, w, h`), `has_slot` varint, when 1 `slot` 4 × f32,
`first_path` varint, `path count` varint.

**Path**: `part` varint, `flags` u8 (1 even-odd, 2 stroke), `stroke_width` zigzag in
quantised asset units (0 when filled), `op_off` varint, `op_len` varint. A path's
bounding box is derived at load.

## QVL1: the letter layer of one page

A letter layer holds the letters of the words of one page. For each letter it gives the
character, the paths that draw it, and the marks of the letter. It also holds the marks
that belong to a word and to no letter of it.

Some page paths join several letters in one outline. The converter cuts such a path
into pieces, and the layer stores the geometry of each piece. The page file does not
change. The reference implementation is `crates/qvp-format/src/letters.rs`.

A letter layer is a 32-byte header followed by four sections whose lengths the header
gives. The sections follow the header back to back, in header order, and the file ends
where the last section ends.

### Header (32 bytes)

| offset | size | field |
|---|---|---|
| 0 | 4 | magic `QVL1` |
| 4 | u16 | format version, 1 |
| 6 | u16 | `quant`, the quantisation factor of the page |
| 8 | u16 | page number |
| 10 | u16 | number of words in the page |
| 12 | u32 | number of paths in the page |
| 16 | u32 | byte length of the tables section |
| 20 | u32 | byte length of the opcodes section |
| 24 | u32 | byte length of the close indices and x deltas section |
| 28 | u32 | byte length of the y deltas section |

An offset (`at`) is a code-point offset in the word's `qpc` text. `0xFFFF` means that the
source gives no offset.

### Section 1: tables

The section starts with four varint counts: letters, members, word marks and pieces.
The four tables follow in that order.

**Letters**, one record each, in word order and then in the order of the word's spelling:

| field | coding |
|---|---|
| word | zigzag delta from the previous letter's word (from 0 for the first letter) |
| `at` | varint |
| text | varint: the Unicode code point of the letter's character |
| member count | varint |
| `more_at` count | varint |
| `more_at` | one varint per offset: other characters that the letter's body also draws |

The first member of a letter is the running total of the member counts before it.

**Members**, stored column by column. A member is a path of a letter: its body ink or a
mark of it.

| column | coding |
|---|---|
| flags, for every member | u8 each: bit 0 body ink, bit 1 a piece, bit 2 a stored `at`. Bits 3 to 7 are 0. |
| path, for every member | varint: with bit 1, a piece index. Without it, an index in the page's path table. |
| `at`, for every member with bit 2 set | varint |

When bit 2 is clear, the reader sets `at` itself: a body member takes its letter's `at`,
and a mark takes `0xFFFF`. The writer sets bit 2 only when `at` differs from that value.

**Word marks**, one record each, for a mark that belongs to a word and to no letter of it:

| field | coding |
|---|---|
| word | zigzag delta from the previous word mark's word |
| path | varint: an index in the page's path table |
| `at` | `opt`: `at + 1`, 0 for `0xFFFF` |

**Pieces**, one record each, in order of source:

| field | coding |
|---|---|
| source | zigzag delta from the previous piece's source, in wrapping 32-bit arithmetic. The source is the page path that the converter cut the piece from. |
| run length | varint: the number of drawing operations in the piece that are not a close |

A piece takes the even-odd flag of its source path. The file does not store it.

### Sections 2 to 4: the drawing operations

Each piece contributes one run of drawing operations, in piece order. A piece's
coordinates are absolute page coordinates, quantised by `quant`. The coding is the
coding of QVP1 sections 2 to 5, with the pieces in place of the paths:

- **Opcodes** (section 2): two bits per operation, four per byte, low bits first.
- **Close indices and x deltas** (section 3): the close indices first, as a varint count
  and then one varint delta per close. The x deltas follow them. The count makes the
  close list end at a known byte, so the x deltas need no length of their own.
- **Y deltas** (section 4).

A piece's first point is a delta from the previous piece's first point. The first
piece's first point is a delta from 0,0.

### What a reader rejects

The reference decoder returns an error for each of these, and a writer keeps them:

- A magic other than `QVL1`, or a version other than 1.
- A `quant` of 0.
- A file shorter than the header or than the section lengths, or bytes after the last
  section.
- A count that its section cannot hold. A record takes at least 5 bytes (letter), 2
  (member), 3 (word mark) and 2 (piece). A `more_at` offset and a close index each take
  at least 1 byte.
- A delta-coded field outside its type, or a varint that does not fit a u16 field.
- A word mark's `at + 1` that is not 0 and does not give an offset in 0 to 65534.
- A letter text that is not a Unicode scalar value.
- A letter `at` of `0xFFFF`. A letter always has an offset. The decoder rejects it, as it
  rejects a word mark's `at + 1` that gives `0xFFFF`. `check_against` also rejects it, so
  that a layer it accepts round-trips.
- Member counts that do not add up to the members count.
- Member flags with a bit other than bits 0 to 2 set.
- A piece member at or past the pieces count. A page member, a word mark's path or a
  piece's source at or past the page's path count. A word at or past the page's word
  count.
- A tables section that the reader does not end on its last byte.
- Run lengths that add up to more than four operations per opcode byte, or to more than
  one per byte of the x deltas or of the y deltas. An opcodes section longer than the
  run lengths need. The reader does not check the unused 2-bit slots of the last opcode
  byte.
- A coordinate with an absolute value of 2^30 quantised units or more.
- A piece that does not start with a move.
- A stream that does not end where its section ends, or a close index that names no
  operation.

These rules tie a layer to its page. `LetterData::check_against` checks them. A reader
calls it before it trusts a layer for a page. A layer that passes these checks decodes
to the same layer that the writer encoded.

- The layer's page number, `quant`, path count and word count are those of the page.
- Letters are in word order, and pieces in source order.
- Inside one word, each letter's `at` is greater than the `at` of the letter before it.
- No letter has an `at` of `0xFFFF`.
- Each letter's first member is the running total of the member counts before it, and
  the member counts add up to the members count.
- Each letter has at least one body member.
- Each piece starts with a move, and each of its coordinates has an absolute value below
  2^30 quantised units.
- Exactly one member uses each piece.
- A letter's page members, and a word mark's path, lie inside the path run of its word.
- A piece's source lies inside the path run of the word of the letter that holds the
  piece, and the source path has kind body. The source is not a glyph instance (flag
  `PF_GLYPH`).
- A page path has at most one role: a member of one letter, one word mark, or the
  source of pieces. One path can be the source of several pieces.
- Every path inside the path run of a word has a role. So a layer covers every word of
  its page. A word whose path run ends past the page's path table is an error.
