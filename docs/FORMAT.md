# The page, atlas, ornament and letter formats

This page specifies the four binary formats the engine reads, byte by byte, so that a
decoder can be written from it without the Rust source. The reference implementation is
`crates/qvp-format/src/codec.rs` (pages), `crates/qvp-format/src/atlas.rs` (atlas),
`crates/qvp-format/src/ornaments.rs` (ornaments) and `crates/qvp-format/src/letters.rs`
(letter layers);
`conformance/qvp1-lite.hex` is a small page written by the encoder, and
`web/lite.mjs` is a second decoder written against this specification. For letter layers,
`scripts/gen-conformance.sh` writes synthetic layers, `conformance/qvl1-letters*.hex`, for
the synthetic pages `conformance/qvp1-letters*.hex`.

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
change. The path that a piece is cut from is its *source*. The reference implementation
is `crates/qvp-format/src/letters.rs`, with the piece geometry in
`crates/qvp-format/src/letters/pieces.rs`.

Most of what a piece draws is already in its source: runs of the source's segments,
parts of one source segment, and cut lines between points on the source. So the file
codes each drawing command of a piece as an operation on its source. A reader needs the
decoded page to decode a layer, and it gets the same commands on every platform, because
the decoder uses integer arithmetic only.

A letter layer is a 72-byte header followed by ten sections whose lengths the header
gives. The sections follow the header back to back, in header order, and the file ends
where the last section ends.

### Header (72 bytes)

| offset | size | field |
|---|---|---|
| 0 | 4 | magic `QVL1` |
| 4 | u16 | format version, 1 |
| 6 | u16 | `quant`, the quantisation factor of the page |
| 8 | u16 | page number |
| 10 | u16 | number of words in the page |
| 12 | u32 | number of paths in the page |
| 16 | u64 | page digest: what the layer depends on in its page (see "The digests") |
| 24 | u64 | body digest: the digest of the whole file, with these eight bytes taken as 0 |
| 32 | 10 × u32 | byte lengths of sections 1 to 10, in order |

The sections are: 1 tables, 2 operations, 3 run lengths, 4 segments, 5 back references,
6 splits, 7 corrections, 8 large corrections, 9 x deltas, 10 y deltas.

An offset (`at`) is a code-point offset in the word's `qpc` text. `0xFFFF` means that the
source gives no offset.

A varint in a letter layer holds at most 32 bits: it has at most five bytes, and a fifth
byte is at most `0x0F`. A reader rejects a longer varint. It accepts a varint with more
bytes than its value needs, as QVP1 does. The page's own opcode runs are QVP1 data, and a
reader reads them as QVP1 does.

### The digests

Both digests use one 64-bit digest of a byte sequence, `digest64`, which takes the bytes
eight at a time:

1. Set `h = 0xcbf29ce484222325`.
2. For each complete group of eight bytes, read as a little-endian u64 `w`: set
   `h = (h XOR w) × 0x9E3779B97F4A7C15` modulo 2^64, then `h = h XOR (h >> 29)`.
3. Take the bytes that are left, zero to seven, padded with zero bytes to eight, as one
   more `w`, and do step 2 with it. This step runs also when no byte is left.
4. Set `h = (h XOR n) × 0x9E3779B97F4A7C15` modulo 2^64, where `n` is the number of
   bytes. The digest is `h XOR (h >> 32)`.

The digest of no bytes is `0x99b6af66b78b8ad8`, of `a` is `0xc24efef7cc63c094`, of
`foobar` is `0xce773884005ec91d`, and of `0123456789abcdef` is `0xa97c4e98d9398a71`.

The **body digest** is `digest64` of the whole file, with bytes 24 to 31 taken as 0. A
reader checks it first, after the magic and the version.

The **page digest** is `digest64` of what the layer depends on in its decoded page, in this
order. Integers are little-endian:

1. The page number (u16), `quant` (u16), the word count (u32) and the path count (u32).
2. For each word: its first path (u32) and its path count (u16).
3. For each path: its kind, mark, family and flags (one byte each), then its geometry.
   For a glyph instance, that is its instance record: the glyph index (u16) and the six
   transform values (each the 32 bits of its f32). For any other path, it is the path's
   commands as one opcode run (below).
4. The glyph outline count (u32), then each glyph outline as one opcode run.

An **opcode run** codes commands as QVP1 codes a path's run in its decoded form: one byte
per command (0 move, 1 line, 2 quadratic curve, 3 cubic curve, 4 close), each followed by
its points as zigzag varint x and y deltas from the point before. In the page digest, a
run's first point is not a delta: the input holds the run's first opcode byte, then (when
that command has a point) the first point as two absolute i32, then the number of the
run's remaining bytes (u32), then those bytes. Each varint is the shortest that holds its
value, whatever varints the page file used. So the input depends on the decoded commands
only, and two page files that decode to the same commands have one page digest. A QVP1
reader that keeps the file's own bytes must code each run again. This engine's decoder
codes every run in this form when it reads a page, so it hashes its runs as they are in
memory. A glyph outline's points are in glyph space.

Strings, boxes, a path's origin and every word field but its path run are not in the
digest. So a words sidecar, which changes word texts, does not change it. Paths alike in
kind, mark, family, flags and geometry are alike to it. The page digest of a page of one
word and one path, page 3, `quant` 100, a body path with flags 0 drawn by
`M 0,0 L 10,0 L 0,10 Z`, is `0x5e3b2943ec1296d2`. Its input is these 46 bytes:

```
0300 6400 01000000 01000000 00000000 0100 00000000
00 0000000000000000 07000000 011400 011314 04 00000000
```

A reader computes the page digest of its page after the header and before any table, and
refuses a layer whose page digest differs: the layer is for another page, or for other
paths of this one. A layer built from one release of the source does not match the pages
built from another release.

### Section 1: tables

The section starts with four varint counts: letters, members, word marks and pieces.

**Alphabet.** A varint count, then one varint per character: the character's code point
minus the code point of the character before it (minus 0 for the first). The alphabet
holds each character of the page's letters once, in code-point order: each delta after the
first is at least 1, and each character is the text of at least one letter.

**Letters**, one record each, in word order and then in the order of the word's spelling.
A record starts with a header byte:

| bits | field |
|---|---|
| 0 to 2 | the member count minus one, for 1 to 7 members. 7: the count is stored, and it is 8 or more. |
| 3 | the letter is in the word after the previous letter's word |
| 4 | `more_at` follows |
| 5 and 6 | `at` code: 0 to 2, or 3 when `at` is stored |
| 7 | a word delta is stored |

Bits 3 and 7 are never both set. The first letter's previous word is 0. With neither
bit, the letter is in the previous letter's word. A stored word delta is neither 0 nor 1,
which have their own codes. A stored `at` is one that codes 0 to 2 cannot give. With bit
4, at least one `more_at` offset follows. The fields that follow the header byte,
in order, each only when its bit or code asks for it:

| field | coding |
|---|---|
| word delta (bit 7) | zigzag delta from the previous letter's word |
| member count (code 7) | varint: the member count itself |
| `at` (code 3) | varint |
| `more_at` (bit 4) | a varint count, then one varint per offset: other characters that the letter's body also draws |
| text | varint: an index into the alphabet |

For an `at` code `c` from 0 to 2: the first letter of a word has `at = c`, and each other
letter has `at` equal to the previous letter's `at + 1 + c`. The first member of a letter
is the running total of the member counts before it.

**Members**, stored column by column. A member is a path of a letter: its body ink or a
mark of it.

| column | coding |
|---|---|
| flags, for every member | u8 each: bit 0 body ink, bit 1 a piece, bits 2 to 4 the `at` code. Bits 5 to 7 are 0. |
| path, for every member | zigzag: see below |
| `at`, for every member with `at` code 7 | varint |

A member's `at` code is 0 for the default `at`: its letter's `at` for a body member, and
`0xFFFF` for a mark. A code `c` from 1 to 5 gives the letter's `at + c - 1`. Code 7 means
that `at` is stored. Code 6 is not used. Each `at` has exactly one code: 0 when the default
gives it, else 1 to 5 for a mark whose `at` is its letter's `at` plus 0 to 4, else 7. So a
body member never has codes 1 to 5, and a code that gives `0xFFFF` for a mark is code 0.

A member's path is a delta in wrapping 32-bit arithmetic. A page path is a delta from the
previous page member's path (from 0 for the first). A piece is a delta from the piece
after the previous piece member (from 0 for the first): the writer gives the pieces in
member order, so most of these deltas are 0.

**Word marks**, one record each, for a mark that belongs to a word and to no letter of it:

| field | coding |
|---|---|
| word | zigzag delta from the previous word mark's word, from 0 for the first |
| path | zigzag delta from the previous word mark's path, from 0 for the first, in wrapping 32-bit arithmetic |
| `at` | `opt`: `at + 1`, 0 for `0xFFFF` |

**Piece groups.** A group is the pieces of one source, next to each other in the pieces
table. A varint group count, then per group: its source, then the number of its pieces
minus one, as varints. The first group's source is stored as it is. Each later group
stores its source minus the previous group's source minus one, so the sources of the
groups rise. The piece counts add up to the pieces count. A piece takes the even-odd flag
of its source path. The file does not store it.

**Operation counts.** One varint per piece, in piece order: the number of operations in
the piece.

**One coding for each table.** The rules above give each table one coding, and a reader
rejects every other: a stored value that a short code gives, an alphabet out of order or
with a character no letter uses, a stored count below 8. The pieces have no such rule: a
writer can code one piece with other operations, and a reader decodes each correctly. The
reference writer is deterministic, so it writes the same bytes again for a layer that it
decoded from its own file.

### Sections 2 to 10: the pieces

| section | holds |
|---|---|
| 2 operations | one 4-bit code per operation, two per byte, low half first |
| 3 run lengths | a varint per run: its segment count minus one |
| 4 segments | varints: segment indices, positions among the segments at a point, and escape codes |
| 5 back references | varints: a count of entries back from the latest of a list |
| 6 splits | varints: split values `v` |
| 7 corrections | one 2-bit code per correction, four per byte, low bits first: 0 is 0, 1 is +1, 2 is −1, 3 is a large correction |
| 8 large corrections | a zigzag per large correction |
| 9 x deltas, 10 y deltas | a zigzag per coordinate, as in QVP1 |

Each operation reads its values from these sections in the order that the table of
operations below gives. A value from section 4 or 5 is read where the operation names it.

#### The segments of a source

A reader decodes the source path's commands from the page (`PageData::ops`, at the path's
own origin) and lists its segments. A source that has commands starts with a move: QVP1
does not require it, the converter never writes a path that does not, and a reader
rejects such a source as a source, because its first command would have no start point.

1. Each line, quadratic curve or cubic curve is one segment. Its start is the current
   point.
2. A subpath ends at a move, a close, or the end of the path. If the subpath's current
   point differs from its first point there, one more segment follows: the line from the
   current point to the first point.
3. A command after a close, with no move between them, starts a new subpath at the first
   point of the subpath that the close ended.

The reader numbers the segments from 0 in that order. The segment after segment `k` is `k + 1`,
except that the segment after the last segment of a subpath is the first segment of that
subpath. "The segments at a point" are the segments that start at that point, in segment
order.

A segment has a degree: 1 for a line, 2 for a quadratic curve, 3 for a cubic curve. Its
points are `P0`, the start, then the control points, then `Pd`, the end.

#### The state of a group

The reader decodes the groups in order, and the pieces of each group in order. At the
start of each group, it lists the source's segments and starts three things empty:

- the **registry**: a list of entries `(point, segment, v)`, a point on a segment
- the **reverse list**: a list of entries `(start, command)`, commands that a later piece
  can draw backwards
- the **run budget**: the number of segments of the source. Each run takes its length
  from the budget.

"The latest registry entry of a point" is the entry with that point that the reader added
last. A back reference `b` names entry `n − 1 − b` of a list of `n` entries.

The **current point** starts at 0,0 for the first piece of the file. At the start of each
later piece, it is the point of the first move of the piece before it, also across
groups. This is the piece's first command, not the start of its last subpath. Each piece
also has a **first point**, the start of its current subpath, and an **open** flag, which
starts false.

#### Operations

| code | operation | reads | draws |
|---|---|---|---|
| 0 | escape | a sub-code from section 4: see below | |
| 1 | run | a position, when several segments start at the current point; a run length from section 3 | the next `length` segments, from the segment at the current point |
| 2 | part to end | a start (see "Parts") | a part of a segment, to the segment's end |
| 3 | part to new | a start; `v1` from section 6; corrections | a part of a segment, to a new point |
| 4 | part to registry | a start; a back reference into the registry | a part of a segment, to a registry point |
| 5 | reverse | a back reference into the reverse list | that entry's command, drawn backwards |
| 6 | cubic | three points from the deltas | a cubic curve |
| 7 | move to segment | a segment index `k` | a move to the start of segment `k` |
| 8 | move to registry | a back reference into the registry | a move to that entry's point |
| 9 | move on segment | a point on a segment (see below) | a move to that point |
| 10 | line to segment | a segment index `k` | a line to the start of segment `k` |
| 11 | line to registry | a back reference into the registry | a line to that entry's point |
| 12 | line on segment | a point on a segment | a line to that point |
| 13 | stored part to end | a stored start, then as code 2 | as code 2 |
| 14 | stored part to new | a stored start, then as code 3 | as code 3 |
| 15 | stored part to registry | a stored start, then as code 4 | as code 4 |

The escape sub-codes are 0 a move from the deltas, 1 no close, 2 a line from the deltas,
3 a quadratic curve from the deltas, 4 a close. Another sub-code is an error.

A point from the deltas is the previous point plus an x delta from section 9 and a y delta
from section 10. The first point of a command is a delta from the current point, and each
other point is a delta from the point before it.

"A point on a segment" reads a segment index `s` from section 4, then `v` from section 6.
The point is the segment's point at `v` (see "Splitting a segment") plus two corrections,
x then y. The reader adds the entry `(point, s, v)` to the registry.

A run draws the segment at the current point and then the segment after it, `length`
times in all. A run length greater than the budget is an error.

After each operation that draws, the current point is the end of what it drew.

#### Moves and closes

A close is not an operation of its own. The reader draws it:

- **A move.** When the open flag is set, the reader closes the subpath first. Then it reads
  the move's point, draws the move, sets the first point and the current point to that
  point, and sets the open flag.
- **A drawing operation** draws its command and sets the open flag.
- **Escape 4, a close**, closes the subpath and clears the open flag.
- **Escape 1, no close**, clears the open flag and draws nothing. The writer uses it for a
  subpath that the piece does not close.
- **The end of the piece.** When the open flag is set, the reader closes the subpath.

To close the subpath, the reader draws a close. If the current point differs from the first
point, it adds `(current point, line to the first point)` to the reverse list. Then it sets
the current point to the first point.

The writer uses escape 4 only for a close that the rules above do not give: a close with a
command after it, or a close directly after another close.

#### The reverse list

The reader adds an entry for each command that it reads from the deltas (escapes 2 and 3,
code 6), for each line of codes 10 to 12, and for a close that draws a line (see above).
An entry is the command's start and the command. Code 5 draws an entry backwards, from its
end to its start. A line becomes a line to the start. A quadratic curve keeps its control
point. A cubic curve swaps its two control points. Runs, parts and code 5 itself add no
entry.

The second piece at a cut usually draws the cut line of the first piece backwards.

#### Parts

A part is the piece of one segment from a start value `u0` to an end value `u1`.

**The start.** Codes 13 to 15 store the start: a segment index `s` from section 4, then
`v0` from section 6. The reader adds `(current point, s, v0)` to the registry when the
operation ends (see below), not when it reads `v0`. Codes 2 to 4 take the start from the
current point:

1. If segments start at the current point, the part's segment is one of them: the only
   one, or the one at a position read from section 4. `u0` is 0.
2. Else, if the registry has an entry for the current point, the part's segment and `v0`
   are those of the latest entry of that point.
3. Else the file is an error.

**The end.**

- Part to end: `u1` is 2^16, and the end point is the segment's end.
- Part to new: `v1` from section 6. The end point is computed and corrected like the other
  points.
- Part to registry: a back reference into the registry. The entry's segment must be the
  part's segment. `u1` comes from the entry's `v`, and the end point is the entry's point.

**The points.** The part has the degree of its segment. Its start is the current point. For
`i` from 1 to the degree, its point `i` is the segment's part point `i` (see "Splitting a
segment") plus two corrections, x then y. The end point of a part to end or to registry is
the given point, and reads no correction.

**The registry, at the end of the operation.** A part reads every value first: the start,
the end's value or back reference, then the corrections. Only then does it add its
entries, in this order: the stored start `(current point, s, v0)` of codes 13 to 15, then
the new end `(end point, s, v1)` of codes 3 and 14. So a back reference of code 15 never
names the entry of its own stored start.

#### Splitting a segment

A split value `v` is stored on the segment's own grid of `2^bits` steps:

- `L` is the sum, over the segment's control polygon `P0 P1 … Pd`, of `max(|Δx|, |Δy|)` of
  each edge.
- `bits` is the bit length of `max(L, 1)`, plus 1, limited to 2 to 16. The bit length of
  `n` is the number of binary digits of `n`.
- `v` runs from 0 to `2^bits`. A larger `v` is an error. The split parameter is
  `u = v × 2^(16 − bits)`, from 0 to 2^16.

The reader computes points in fixed point with 14 fractional bits, in signed 64-bit
integers:

1. Set `X_j = P_j × 2^14` for each point `j` from 0 to `d`, for x and for y apart.
2. One de Casteljau level at parameter `w` replaces `X_j` with
   `X_j + floor((X_(j+1) − X_j) × w / 2^16)` for `j` from 0 to one less than the number of
   values, and drops the last value. `floor` rounds towards minus infinity: an arithmetic
   shift right by 16.
3. A blossom at `w_1, …, w_d` runs `d` levels, level `k` at `w_k`, and gives the one value
   left.
4. A value rounds to the quantised grid as `floor((X + 2^13) / 2^14)`: halves round up.

The segment's point at `u` is the blossom at `u, …, u`. The part point `i` of the part from
`u0` to `u1` is the blossom at `d − i` copies of `u0`, then `i` copies of `u1`.

With coordinates below 2^30 in absolute value, `X` is below 2^44, a difference below 2^45,
and a product with `w ≤ 2^16` below 2^61. So no step overflows a signed 64-bit integer.
Each level gives a value between its two inputs.

#### Corrections

A correction is the difference between a point of the piece and the computed point. The
writer stores one correction per coordinate. Most are 0, +1 or −1 and take a 2-bit code.
Code 3 reads a zigzag from section 8. Because the file stores the corrections, the split
rule above is exact by definition: any rule would round-trip, but every reader must use
this one.

#### A worked example

The source is one path: `M 0,0 C 1000,3000 4000,3000 5000,0 L 5000,-2000 Q 2500,-4000
0,-2000 Z`. Its segments are 0 the cubic, 1 the line, 2 the quadratic curve, and 3 the line
from 0,-2000 back to 0,0. The cubic has `L` = 9000 and 15 bits. The quadratic curve has
`L` = 5000 and 14 bits. A cut at x = 2500 makes two pieces:

- left: `M 0,0 C 500,1500 1500,2250 2500,2250 L 2500,-3000 Q 1250,-3000 0,-2000 Z`
- right: `M 2500,2250 C 3500,2250 4500,1500 5000,0 L 5000,-2000 Q 3750,-3000 2500,-3000
  L 2500,2250 Z`

The file holds one group with source 0 and the operation counts 4 and 5:

| piece | operation | values | state after it |
|---|---|---|---|
| left | 7 move to segment | segment 0 | current point 0,0 |
| left | 3 part to new | start: segment 0 starts at 0,0, so `u0` = 0; `v1` 16384; six corrections 0 | registry: 0 is 2500,2250 on segment 0 |
| left | 12 line on segment | segment 2, `v` 8192, two corrections 0 | registry: 1 is 2500,−3000 on segment 2; reverse list: 0 is the cut line |
| left | 2 part to end | start: registry entry 1; two corrections 0 | |
| left | end of piece | the reader closes: 0,−2000 differs from 0,0 | reverse list: 1 is the line to 0,0 |
| right | 8 move to registry | back 1: entry 0 | current point 2500,2250 |
| right | 2 part to end | start: registry entry 0; four corrections 0 | |
| right | 1 run | length 1: segment 1 | budget 3 |
| right | 4 part to registry | start: segment 2 starts at 5000,−2000; back 0: entry 1 | |
| right | 5 reverse | back 1: reverse list entry 0 | |
| right | end of piece | the reader closes, at the first point | |

The page of the example is page 1, with `quant` 100, one word that holds path 0, and path
0, a body path with flags 0 drawn by the commands above. The layer has two letters, ل at
offset 0 and ا at offset 1, each with one body member: piece 0 and piece 1.

The header holds `quant` 100, page 1, one word, one path, the page digest
`0x80050d8c356e268c`, the body digest `0x28ac373a6749fa54`, and the section lengths 21, 5, 1,
2, 3, 5, 4, 0, 0 and 0. The sections of this file: operations `37 2C 28 41 05` in
hexadecimal, where the high half of the last byte is unused, run lengths `00`, segments
`00 02`, back references `01 00 01`, splits `80 80 01 80 40`, corrections four bytes of 0,
and no deltas. The tables section is
`02 02 00 02 02 A7 0C 1D 00 01 00 00 03 03 00 00 01 00 01 04 05`. The whole file is 113
bytes:

```
51564c310100640001000100010000008c266e358c0d058054fa49673a37ac28
1500000005000000010000000200000003000000050000000400000000000000
00000000000000000202000202a70c1d00010000030300000100010405372c28
4105000002010001808001804000000000
```

`crates/qvp-format/src/letters/vectors.rs` holds this file and eleven more, each with the
exact commands it decodes to. Its tests do not call the encoder. Here is one more of them,
a run that wraps from the end of a subpath to its start. The page is page 9, `quant` 100,
one word of three paths with flags 0:

- path 0, a body path: `M -1000,-1000 C -700,-1301 -299,-1299 1,-999 L 1,-2001
  Q -500,-2303 -1000,-2001 Z`
- path 1, a body path: `M 5000,0 L 6000,0 L 6000,1000 Z M 7000,0 C 7300,0 7700,400
  8000,1000 L 7000,1000 Z`. Its segments are 0 to 2 for the first subpath, then 3 (the
  cubic), 4 and 5 (the closing line) for the second.
- path 2, a mark: `M 0,500 L 100,500 L 100,600 Z`

The layer has one letter, ب at offset 0, whose one body member is the piece. The piece is
cut from path 1: `M 8000,1000 L 7000,1000 L 7000,0 C 7300,0 7700,400 8000,1000 Z`. It is a
move to the start of segment 4, then a run of 3 that draws segments 4, 5 and 3. The file:

```
51564c3101006400090001000300000094fdac7c3f3af286718536064fee83fd
0f00000001000000010000000100000000000000000000000000000000000000
00000000000000000101000101a80c0000030001010002170204
```

### Limits

A layer keeps these limits, and a reader rejects a layer past one before it allocates for
it. The writer and `check_against` refuse a layer past one. Each limit is at least four
times the largest value in the 604 layers of `quran-svg-elements` v2.0.0:

| limit | value | largest in the 604 layers |
|---|---|---|
| commands in a layer | 65,535 | 10,880 |
| commands in a piece | 512 | 62 |
| operations in a piece | 1,024 | 29 |
| operations in a layer | 131,070 | 5,474 |
| pieces of one source | 64 | 10 |
| registry entries in a group | 256 | 46 |

A piece has at least one command, so a layer holds at most 65,535 pieces. A command takes
at most two operations, so a layer inside the command limits is inside the operation
limits. The writer never fills the registry: when a group's registry is full, it codes a
point from deltas.

So a layer decodes to at most 65,535 commands, about 1.8 MB of commands at 28 bytes each.
The largest file inside the limits that we built holds 65,535 commands of parts of
segments, on page 552: 109,601 bytes, decoded in 2.4 ms, with 1.5 MB more peak memory than
the page alone.

### What a reader rejects

The reference decoder returns an error for each of these, and a writer keeps them. Each
error is of one of four kinds, so a caller can tell them apart:

- **Not a letter layer** (`NotLetterLayer`): the file does not start with `QVL1`.
- **A version this reader does not know** (`LetterLayerVersion`).
- **A layer for another page** (`OtherPage`): the page number, `quant`, path count or word
  count of the page differs from the header's, the page digest differs or cannot be taken
  (a run that leaves the opcode stream, a glyph instance with no record), or a source is
  not a path that the page can give.
- **Damaged bytes** (`Damaged`): the body digest differs, the file is too short, or the
  file breaks any other rule below.

The decoder checks in this order: the magic and the version, the length of the header,
the body digest, the section lengths, the page's counts, the page digest, the tables, the
sources, then the pieces. The rules:

- A magic other than `QVL1`, or a version other than 1.
- A body digest that is not the digest of the file. A page digest that is not the digest
  of the page.
- A layer past one of the limits above.
- A `quant` of 0.
- A file shorter than the header or than the section lengths, or bytes after the last
  section.
- A page whose page number, `quant`, path count or word count differs from the header's.
- A count that its section cannot hold. A record takes at least 2 bytes (letter), 2
  (member), 3 (word mark) and 1 (piece), and a group 2. An alphabet character and a
  `more_at` offset each take at least 1 byte.
- A varint of more than five bytes, or a fifth byte above `0x0F`.
- A delta-coded field outside its type, or a varint that does not fit a u16 field.
- A table that is not in its one coding: an alphabet delta of 0 after the first, an
  alphabet character that no letter uses, a stored word delta of 0 or 1, a stored member
  count below 8, a stored letter `at` that codes 0 to 2 give, bit 4 with a `more_at` count
  of 0, or a member `at` code other than the one its `at` has.
- An alphabet code point that is not a Unicode scalar value, or a letter text index past
  the alphabet.
- A letter header with bits 3 and 7 set.
- A letter `at` of `0xFFFF`, stored or computed. A letter always has an offset.
  `check_against` also rejects it, so that a layer it accepts round-trips.
- A word mark's `at + 1` that is not 0 and does not give an offset in 0 to 65534.
- Member counts that do not add up to the members count.
- Member flags with bits 5 to 7 set, or an `at` code of 6. A computed member `at` above
  `0xFFFF`.
- A piece member at or past the pieces count. A page member, a word mark's path or a
  group's source at or past the page's path count. A word at or past the page's word
  count.
- A group count above the pieces count, a group source that overflows, or group piece
  counts that do not add up to the pieces count.
- A tables section that the reader does not end on its last byte.
- Operation counts that add up to more than two per byte of the operations section, or an
  operations section longer than they need. A set bit in the unused half of the last
  operation byte, or in the unused codes of the last correction byte.
- A source that is a glyph instance (flag `PF_GLYPH`), a source whose commands do not
  decode, a source whose first command is not a move, or a source point with an absolute
  value of 2^30 or more.
- A large correction of −1, 0 or +1.
- An escape sub-code above 4. A segment index past the source's segments, a position past
  the segments at the current point, or a back reference past its list.
- A run where no segment starts at the current point, or a run longer than the budget.
- A part of codes 2 to 4 whose start is neither a segment start nor in the registry. A part
  to registry whose entry is on another segment. A split value above `2^bits`.
- A coordinate with an absolute value of 2^30 quantised units or more, or a coordinate
  overflow.
- A piece that does not start with a move.
- A section that the reader does not end on its last byte, except for the unused bits
  above.

These rules bound the work and the output. Each source is listed once, because the group
sources rise, and over a group the runs draw at most the source's segment count. The
limits bound the commands, the operations and the registry before anything is allocated
for them.

These rules tie a layer to its page. `LetterData::check_against` checks them. A reader
calls it on what `decode_letters` returns, before it trusts the layer for a page. The
writer encodes every layer that passes these checks, and the reader decodes it to the
same layer, every command of every piece bit for bit.

- The layer's page number, `quant`, path count and word count are those of the page.
- Letters are in word order, and pieces in source order.
- The layer is inside the limits above.
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

Coverage has a cost for later versions. A layer gives every path of every word exactly
one role, so a layer covers every word of its page. A layer cannot leave out a word that
its source does not divide into letters.

Upstream keeps the `data-unsplit` attribute for such a word, and no page uses it today.
The converter stops a page that has one. To support such a word, add a new record or a
new format version. Do not make a reader relax this check.

### What the engine checks at attach

`Page::attach_letters` in `crates/qvp-core/src/letters.rs` (`qvp_attach_letters` in C) refuses
a layer, and leaves the page as it was, in this order:

1. A file over 1 MiB (`MAX_LETTER_LAYER_BYTES`), before it decodes anything. The largest of
   the 604 layers is 17,057 bytes, and a layer at the command limit about 110 KB.
2. Every rule of "What a reader rejects": `decode_letters` against the loaded page. The body
   digest, the page's counts and the page digest are part of this step. The engine takes the
   page digest from the page it holds, at the attach. Loading a page does not take it.
3. Every rule of `check_against` above, against the same page.
4. A page whose word path runs overlap, so that a path is in two words. The format allows
   such a page, but the engine takes no layer for it: each path of a word must belong to
   that word only. No page of the mushaf has overlapping runs.
5. A layer whose counts, added to the page's, do not fit the engine's indices.

The engine does not refuse a layer for its offsets. When the page has a words sidecar that
gives a word's `qpc` text, the engine counts an offset at or past the length of that text as
no offset. It checks again when a sidecar comes after the layer. Without a sidecar it uses the
offsets as stored.

The engine numbers the pieces after the page's own paths. A page with `N` paths and a layer
with `P` pieces has the paths `0` to `N + P - 1`, the pieces in the order of the pieces
table. No page path changes its number, and no word's path run holds a piece. Because a
layer covers every word of its page, each path of each word has a role in the layer once it
is attached. A second attach removes the pieces of the first and numbers the new ones from
`N` again.

### Versions

A reader rejects every version other than 1. Any change to this layout is a new version:
a field, a section, an operation, the split rule, the rounding, or the digest. A writer
does not change the meaning of a version-1 file.
