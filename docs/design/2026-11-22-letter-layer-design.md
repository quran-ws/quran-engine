# Letter layer: design

Date: 2026-11-22. Status: built. The section "As built" at the end lists what changed.

## Goal

The engine addresses a word and the paths of a word. This design adds the letter: an app
can colour a letter or a mark, find the letter under a finger, and read a letter's box and
outline. The source is the letters archive of the `quran-svg-elements` release v2.0.0.

The design adds to the engine. It does not change the QVP1 format, a struct layout or a
function signature. A page with no letter layer behaves as it does today.

## Scope

This spec has three phases:

1. Move the source to the v2.0.0 words archive.
2. Build a letter layer file for each page.
3. Load the layer in the engine, and bind it in every wrapper.

A second spec will cover what this one leaves out:

- letter-level mask, reveal and recitation follow;
- a highlight band behind a letter;
- a span list, which applies many letter ranges in one call.

## The source

The v2.0.0 release has two archives, built from the same artwork.

| archive | content |
|---|---|
| `quran-svg-elements-hafs-kfgqpc-words.tar.gz` | the v1.1.2 page shape, with `data-w` on each word and five ink fixes |
| `quran-svg-elements-hafs-kfgqpc-letters.tar.gz` | each word divided into `g.letter` groups |

In the letters archive, each letter group holds the body ink of the letter and the marks
of the letter. `data-at` on a letter and on a mark path gives the code-point offset of its
character in the word's `qpc` text. Where the artwork joins two letters in one contour,
the archive cuts the contour, and each piece is a path with `data-cut="1"`.

Measured on pages 1, 3, 50, 232, 300, 526 and 604:

- 4,721 of 7,119 letter-archive word paths (66%) are byte-identical to a path of the same
  word in the words archive.
- 2,397 are cut pieces. They replace 860 of 5,581 word-archive paths (15%).
- 1 path is neither: the hizb rosette, which the letters archive puts inside a word.

Measured on all 604 pages (77,432 words, 333,220 letters):

- The word keys and their order are the same in both archives. The ink outside the words
  is the same, except the 199 hizb rosettes.
- Each of the 508,522 uncut letter paths is byte-identical to a path of the same word,
  except the 199 rosettes. No word has two paths with the same `d`.
- Cut pieces replace 89,853 word paths. Each of them has kind `body`.
- Each letter has an offset inside the word's `qpc` text, and its character equals the
  `qpc` character at that offset, after the rosette prefix is subtracted.
- 2,326 letters have two body paths, and one letter has three.
- One letter draws two characters: the lam of `2:97:5` on page 15 has `data-text="لـ"`
  and `data-at="0 3"`. Its body is also the kashida at offset 3.
- 4 mark paths have no `data-at`. 4,275 mark paths are children of the word and not of a
  letter. All are waqf signs, except 3 `seen_al_qiraah`.
- A bounding-box test maps a cut piece to one body path for 261,269 of 261,953 pieces. It
  gives more than one body for 679 pieces and none for 5.

The letters archive is not print-exact. The edge of a cut piece can differ from the
artwork by up to 0.0125 units, and two pieces that meet at a cut can show a light seam
when a renderer antialiases them. The letter division is the output of DigitalKhatt's
`medinamushafanalyzer` (commit `056191f`, MIT licence). Upstream quotes no accuracy
figure for where a cut falls.

## Decisions

- **The engine draws word ink.** Page data comes from the words archive only. A plain
  page stays print-exact, and the identity gate keeps its meaning.
- **The letter layer is a separate, optional file.** An app that does not use letters
  downloads nothing more.
- **The engine draws cut pieces only when it must.** A word draws its own paths until one
  of its letters has a colour that differs from the rest of the word.
- **One canonical address.** The engine stores a letter or a mark as a word and a `qpc`
  code-point offset. Every other address form resolves to that.

## Phase 1: the v2.0.0 words archive

- `scripts/sync-test-data.sh` takes `v2.0.0` as the default `QVP_SVG_TAG` and fetches the
  words archive under its new name. The checksum check does not change.
- The converter reads `data-w` and writes it to `NNN.words.json` as `w`, beside the five
  text forms. `data-w` is not a key. Its numbering has gaps.
- `qvp_attach_words` ignores a field it does not know, so an older engine reads the new
  sidecar. A test proves this.
- The full identity gate runs on all 604 pages.
- A new data release carries the rebuilt pages. `docs/DATA.md` and
  `docs/UPSTREAM-DATA-ISSUES.md` record the five ink fixes.

## Phase 2: the letter layer file

### The file

One file per page, `NNN.qvl`, magic `QVL1`, in the same data release as `NNN.qvp`. It
uses the conventions of `docs/FORMAT.md`: varints, zigzag deltas, and the page's `quant`.
The file holds four tables.

**Header.** Magic, format version, page number, `quant`, the path count of the page it
belongs to, and the table lengths. The engine rejects a layer whose page number or path
count does not match the loaded page.

**Letters**, one record per letter, in word order and then reading order:

| field | meaning |
|---|---|
| word | the word's index on the page, as a delta |
| `at` | the code-point offset of the letter in the word's `qpc` text |
| text | the letter's character, one code point |
| member count | how many paths the letter holds |
| more offsets | a count, then the offsets of the other characters that the letter's body also draws. The count is 0 for every letter but one (`2:97:5`, a lam and its kashida). |

A letter can hold more than one body member.

**Members**, one record per path of a letter. A member is one of two things:

- a reference to a path that the page already has, by its index in the word;
- a cut piece, with the index of the body path that it replaces.

Each member also carries the `at` of its own character when it is a mark. A dot carries
the `at` of its letter, as upstream writes it.

**Word-level marks.** A mark that belongs to the word and not to one letter stays outside
every letter. The table lists these paths with their `at`, or with none. Upstream leaves
a small number of mark paths without `data-at`. They stay addressable by the existing
`QVP_SELECTOR_WORD_MARK`.

**Cut pieces.** Their drawing operations, coded as the page codes its paths. A cut piece
has kind `body`, the even-odd flag of its source path, and no mark.

### What the converter does

`qvp-convert` gets a `letters` subcommand. It reads a letters-archive page, the
words-archive page and the built `NNN.qvp`, and writes `NNN.qvl`.

- It matches each letter-archive path that has no `data-cut` to a page path by its `d`
  string in the same word. A path with no match is an error, with two known exceptions
  below.
- It maps each cut piece to the body path it came from. Upstream proves that each piece
  lies within 1/8 of a unit of its source contour. The converter tests the points of the
  piece against the outline of each candidate body path, because a bounding-box test is
  ambiguous for 684 pieces. It reports a piece that matches no body path or more than one.
- **Rosettes.** The letters archive puts each of the 199 hizb rosettes inside the next
  word, and starts that word's text with `۞` and a no-break space. The page keeps the
  rosette as a division mark, as today. The converter drops the rosette from the layer and
  subtracts the prefix from every `at` of that word. Every offset then indexes the `qpc`
  text that the engine already publishes. The converter checks this for each of the 199
  words.
- **Body paths that are marks.** 1,670 strokes are `data-kind="mark"` in the words
  archive and the whole body of a letter in the letters archive: the dagger alif, and two
  fused hamzah signs. The page keeps the mark kind and the mark name. The layer records
  the path as the body member of its letter.
- The converter repairs nothing. It reports what does not fit, as `svg2qvp` does.

### Gates

- **Structure.** Every letter has at least one body member. Every `at` is inside the
  word's `qpc` text. Every page path of a word is a member of one letter, a word-level
  mark, or a body path that cut pieces replace. No path is in two letters.
- **Ink.** The letter gate draws each word two ways: from its own page paths, and from its
  letter members with the cut pieces in place. It compares the two with the metric and
  the threshold of the identity gate. It runs on the eight identity-gate pages by default
  and on all 604 with `QVP_TEST_ALL=1`. If cut pieces fail the threshold, the work stops
  and the finding goes upstream. The gate is not loosened.
- **Size.** Phase 2 measures the layer for all 604 pages, raw and compressed, and records
  the figures in `docs/MEASURED.md`.

### Data release

`scripts/package-data.sh` adds the `.qvl` files to the release. `VERSION.json` records
the letters archive tag, the analyzer commit and the letter count. `docs/DATA.md` and the
notice file carry the DigitalKhatt attribution and the MIT licence text, and state the
three limits of the layer: the cut convention, the hairline, and the seam.

## Phase 3: the engine

### Loading

```c
int32_t qvp_attach_letters(QvpPage*, const uint8_t* bytes, uint32_t len);  /* 0 ok, -1 bad or wrong page */
uint8_t qvp_has_letters(const QvpPage*);
```

Attaching appends the cut pieces to the page's path table, after the paths it has. No
existing path index changes. `qvp_page_info` reports the new path count, and
`qvp_colors` covers the new paths. An app attaches the layer before it reads geometry, as
it does with the words sidecar. Attaching a second time replaces the first layer.

### When a word draws its pieces

`qvp_layout_draw_list` is the one place that decides. For each word:

- When every path of the word has the same colour state, the list holds the word's own
  paths. The pieces are not in it.
- When a rule gives a letter of the word a colour that differs from the rest of the word,
  the list holds the pieces of each cut body path in place of that path.
- The word goes back to its own paths when the last such rule is gone and its colour
  transition has ended.

A seam can show only inside a word that is drawn in two colours. A host that caches its
ink already redraws when a colour changes, and the swap happens only with a colour change.

Phase 3 starts with a check of every wrapper and the web code. Any code that draws paths
`0..n_paths` directly, and not through the draw list, would draw the pieces as well. The
check finds such code and moves it to the draw list before the layer lands.

### Reading letters

```c
typedef struct { uint32_t word, index, at, text /* code point */, first_member, n_members, n_marks;
                 float x0, y0, x1, y1; uint8_t has_ink; } QvpLetterInfo;
typedef struct { uint32_t path, at /* QVP_NONE when none */; uint8_t mark, is_body, is_cut; } QvpLetterMember;

uint32_t qvp_letter_count(const QvpPage*, uint32_t word);
int      qvp_letter_info(const QvpPage*, uint32_t word, uint32_t letter, QvpLetterInfo* out);
uint32_t qvp_letter_members(const QvpPage*, uint32_t word, uint32_t letter, QvpLetterMember* out, uint32_t cap);
int      qvp_path_letter(const QvpPage*, uint32_t path, uint32_t* out_letter, uint32_t* out_at);
int      qvp_letter_at_offset(const QvpPage*, uint32_t word, uint32_t at, uint32_t* out_letter);
```

Each path of a member is an ordinary path, so `qvp_geometry` gives its outline. An app
that wants the outline of a letter reads the members and then the geometry.

### Addressing

Four new selectors go at the end of the `QVP_SELECTOR_*` enum. Each works with
`qvp_style_add`, `qvp_style_hide` and every other call that takes a `QvpSelector`.

| selector | a | b | c | selects |
|---|---|---|---|---|
| `QVP_SELECTOR_LETTER` | word | letter | part | the letter: 0 all, 1 body and dots, 2 marks |
| `QVP_SELECTOR_LETTER_MARK` | word | letter | n | the nth mark of the letter |
| `QVP_SELECTOR_LETTER_MARK_NAMED` | word | letter | mark id | the marks of that name on the letter |
| `QVP_SELECTOR_WORD_CHARS` | word | from | to | the ink of the `qpc` characters from..to, inclusive |

`QVP_SELECTOR_WORD_CHARS` is the canonical form. A letter's offset selects its body and
its dots. A mark's offset selects that mark. A character with no ink, such as a kashida
or a space, selects nothing and is not an error. A rule that crosses two words is two
ranges.

A query selects letters by what they are:

```c
typedef struct { QvpTarget scope;      /* page, word, words, ayah, ayah range, line */
                 uint32_t text;        /* a code point, or 0 for any letter */
                 uint8_t  mark;        /* a mark id the letter must have, or 0 */
                 uint8_t  part;        /* what to select of each match: 0 all, 1 body and dots, 2 marks */ } QvpLetterQuery;

uint32_t qvp_find_letters(const QvpPage*, const QvpLetterQuery*, uint32_t* out /* n × {word, letter} */, uint32_t cap);
uint32_t qvp_style_add_letters(QvpPage*, int32_t layer, const QvpLetterQuery*, uint32_t rgba, uint32_t transition_ms);  /* one handle */
```

"Every noon with a shaddah in ayah 2:6" is one call: scope the ayah, text `ن`, mark
`shaddah`.

All offsets are `qpc` offsets. Upstream links ink to the `qpc` text only. The engine does
not map an offset in another text form.

A page with no layer attached returns 0 from every new call, and a new selector on it
selects nothing.

### Hit testing

```c
typedef struct { uint32_t word, letter, path, at, line; float distance; uint8_t mark, is_exact; } QvpLetterHit;
int qvp_hit_test_letter(const QvpPage*, float x, float y, const QvpHitOptions*, QvpLetterHit* out);
int qvp_hit_test_letter_view(const QvpPage*, const QvpView*, float x, float y, const QvpHitOptions*, QvpLetterHit* out);
```

The engine finds the word as `qvp_hit_test` does. Inside the word, a point inside a
member's outline is an exact hit: `path` is that member, `mark` is its mark id or 0, and
`at` is its character. A point outside every outline gives the letter whose box is
nearest, with `is_exact` 0. `QvpHit` and the existing hit functions do not change.

### Wrappers

Each wrapper binds the new calls, as `scripts/check-parity.py` requires, and adds no
formula. The data reads as a tree:

```
page.word("2:6:2").letters[1]     text "ل", at 1, box, marks
                  .marks[0]       name "shaddah", at 2, path
page.letterAt(x, y)               word key, letter index, text, the mark hit if any
page.style(letter, color)         the handle model of today
page.style(letters(text: "ن", mark: .shaddah, in: ayah), color)
```

A letter, a mark and a character range each convert to a `QvpSelector`. The wrapper
builds the selector, and the engine resolves it.

## Compatibility

- QVP1 and its format version do not change. `NNN.qvl` is a new file with its own magic
  and version.
- No existing struct, enum value or function signature changes. New enum values go last.
- `NNN.words.json` gains one field that older engines ignore.
- A wrapper release that knows letters still loads a data release that has none.
- The ABI test covers each new struct, and the conformance cases cover each new call.

## Tests

- Unit tests in `qvp-format` for the `QVL1` codec: round trip, and each reason to reject.
- Unit tests in `qvp-core` on a fixture word: each selector, the query, the hit test, the
  swap in the draw list, and the swap back after a transition.
- The structure gate and the ink gate of phase 2.
- A wrapper test per platform that attaches a layer, colours one letter and reads it back.
- `scripts/check.sh` runs all of them. `scripts/check-parity.py` covers the new calls.

## Docs

`docs/FORMAT.md` (the `QVL1` section), `docs/API.md`, `docs/API-PARITY.md`,
`docs/HOW-IT-WORKS.md`, `docs/DATA.md`, `docs/CDN.md`, each wrapper's changelog, and the
root changelog under Unreleased.

## Risks

- **The piece-to-body match is ambiguous on some page.** The converter reports it and
  stops. The fix is upstream, or a recorded rule in the converter that the gate proves.
- **The pieces fail the ink threshold.** The quantised pieces can differ from the word
  ink at a cut. The ink gate finds this in phase 2, before any engine work depends on it.
- **A host draws all paths without the draw list.** The check at the start of phase 3
  finds it.
- **Layer size.** Unknown until phase 2 measures it. The layer is optional, so a large
  layer costs only the apps that use it.

## As built

The work followed this design, with these changes. `docs/API.md`, `docs/FORMAT.md` and
`docs/DATA.md` describe the result.

- **The file format.** `QVL1` codes a piece as runs and parts of the page path it was cut
  from, not as coordinates. The 604 files are 3.9 MB with brotli. The first layout, which
  this design describes, was 25.9 MB. The header holds a digest of the page and a digest of
  the layer. `decode_letters` takes the page. The file has limits, such as 65,535 commands.
- **Packaging.** `scripts/package-data.sh` and `VERSION.json` change with the data release,
  not in phase 2.
- **The parts of a letter go by offset.** The body and dots part is each member at the
  offset of the letter. It selects the same paths as the character range over that offset.
  The marks part is the other members.
- **The marks of a letter are in text order.** The nth mark counts in that order.
- **The swap is per cut path, not per word.** A cut path draws as its pieces while their
  colour states differ. A path whose pieces have one colour draws whole in that colour.
- **`qvp_layout_draw_revision` is new.** A host reads it to know when to fetch the draw
  list again. With a layer, the draw list and the revision apply rule changes first.
- **The letter hit test prefers ink.** On ink, the word is the word that has the ink. A
  point between two pieces is an exact hit. A point in the counter of a letter gives that
  letter. `QvpLetterHit` has `is_body`. The view variant takes layout px and no view.
- **The page check is stricter.** Each word path has one role, the offsets of the letters
  of a word increase, and an attach refuses a page whose word runs overlap.
- **Wrapper names.** In each wrapper, `text` is the character and `codePoint` is the
  number. A mark has `mark` (the id) and `markName`.
- **The ink gate asserts more.** It asserts, for each cut path, the area of the pieces, a
  partner for each cut side, and the direction of each shared side.
- **Not built, as planned.** Letter-level mask, reveal and follow, a band behind a letter,
  and a span list are for a later design.
