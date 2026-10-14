# Dress a page in another mushaf's ornaments

Status: draft for review. Supersedes the placement model of PR #27.

## Goal

A page carries the King Fahd Complex's own ayah medallions, surah bands and page furniture.
`dress` puts another mushaf's ornaments on the page instead, and hands the host a display
list to paint behind the ink. The ornaments come from `quran-ws/quran-assets`, converted
once into an ornament set the engine reads without an XML parser.

PR #27 built this on a version of the engine that moved every path by its printed line.
The engine now draws through placements, exposes the measurements a dress needs, and the
catalogue changed its schema. This design keeps the PR's architecture and rebuilds its
placement on what main has.

## What is kept from PR #27

- A separate binary format, QVO1, in `qvp-format`. An ornament set is not page data and
  must not ride in a page file.
- A display list the host paints behind the page ink. Only the printed ring is hidden. The
  numeral is the print's own ink and stays where it is, on top of the replacement.
- Every ornament is placed from a measurement, never a fixed offset.
- The border goes around the laid-out page, not the printed one. A layout re-places the
  dress, and a revision number tells the host its raster went stale.
- The dressed page is bigger than its text. The engine reports the overflow on each side.
- A tiled frame keeps its corner and repeats only the straight run.
- `line_art` keeps only the constant-width strokes, per kind, and a host can repaint any part.
- The converter expands `<use>` mirrors, flattens transforms and drops the whole-frame
  outlines of a frame that tiles.

## What changes

### 1. The set mirrors the catalogue: one ornament per asset

The catalogue has one entry per style and type. 47 of its 71 entries are ayah markers from
fonts with no header or frame. So the set holds a flat list of ornaments, and a dress names
up to three of them, one per kind.

```rust
pub struct OrnamentSet { pub quant: u16, pub ornaments: Vec<Ornament>, pub paths: Vec<OrnamentPath>, pub ops: Vec<u8> }

pub struct Ornament {
    pub kind: OrnamentKind,          // AyahMark | SurahHeader | PageFrame
    pub style: String,               // the catalogue's style key, e.g. "mushaf-qalon", "font-003-regular"
    pub lineage: Lineage,            // Scan | Font
    pub riwayah: String,             // empty for a font marker
    pub license: License,            // id, status, redistributable, attribution
    pub parts: Vec<Part>,            // the palette: name, 0xRRGGBBAA, stroke
    pub asset: Asset,                // view_box, slot, first_path, n_paths
    pub slices: Option<Slices>,      // a frame that tiles
}
```

`redistributable` is `license.status == "confirmed"`. The catalogue has no separate field.

`Asset.slot` is the catalogue's `slots[0]` box. The engine reads `x, y, w, h` and derives
the centre. A font marker has a slot box but no `slot` part.

### 2. Selection is per kind

```rust
pub struct DressSpec {
    pub ayah_mark: u32,      // index into OrnamentSet::ornaments, or NONE
    pub surah_header: u32,
    pub page_frame: u32,
    pub gap: f32,            // air between the text and the border, page units
    pub band_gap: f32,       // air between a band and the border, page units
    pub ayah_mark_scale: f32,  // the medallion's size against the printed ring; 0 = the design's own (1.5 scan, 1 font)
    pub page_frame_scale: f32, // the border's size against the design's own; the pattern scales as a whole
    pub line_art: u32,       // LINE_ART_* bits: which kinds are drawn as line art
    pub colors: Vec<(u32, u16, Rgba)>,   // (ornament, part, colour) overrides
}
```

An index whose ornament is of another kind is an error, and `dress` returns `None`.

### 3. Draws carry a placement, not a line

```rust
#[repr(C)]
pub struct OrnamentDraw {
    pub op_start: u32, pub op_count: u32, pub pt_start: u32, pub pt_count: u32,
    pub color: Rgba,
    pub flags: u32,          // kind | EVENODD | STROKE
    pub stroke_width: f32,   // page units, 0 when filled
    pub part: u32,
    pub placement: u32,      // index into the layout's placements, or NONE
}
```

`placement` is the same index a page `Draw` carries. A host applies the transform it already
applies to page paths. Without a layout the index is the printed line, which is also what the
page draw list says without reflow.

- A medallion takes the placement of its ring's ornament path.
- A band takes the placement of its title's first path.
- The border takes `NONE`: it is drawn in layout space with no group transform.

This is what makes dress work under reflow. A medallion follows its word onto its row, and a
band follows the title, including the title's banner scaling.

### 4. Placement reads the page's own measurements

| ornament | measured from | rule |
|---|---|---|
| medallion | `ayah_marks()`: `cx, cy, r, ornament_path` | scale so the asset's height equals `2r`, keep the aspect, put the asset's slot centre on `(cx, cy)` |
| band | `surah_headers()`: title box, decoration, line | width is the text column less `2·band_gap`, centred in the column, the slot's vertical centre on the title's vertical centre |
| border | the laid-out page box | the frame's text-area slot maps onto the page box grown by `gap` on every side. A frame with slices is tiled at one uniform scale. A frame without slices is stretched so that its slot lands exactly on the grown page box, with one scale per axis |

The text column is the page width, plus `2·gap` when a border is drawn. It is never this
page's ink: pages 1 and 2 hold one short surah centred in the column, and a band measured
from their lines comes out a third of the width it has elsewhere.

Under reflow a dressed band holds its title: `layout_reflow` caps the banner's growth at
`0.9 · slot height / title height`, so the tallest title stays inside the slot.
Under reflow the band's width is the text column in printed units, the laid-out width
before any placement. The title's placement then scales the band with the title, as it
scales the title's own ink.

The laid-out page box is new on `Layout`: `laid_top` and `laid_height`, in page units.
The printed layout sets them from its padding and block height. The reflow layout sets them
from its padding and content height. Without a layout the box is the printed page.

### 5. Hiding the native ornament uses the style engine

A dress with a medallion hides the printed rings with one style rule
(`Selector::Kind(PathKind::AyahMarkOrnament)`, `Paint::HIDDEN`). A dress with a band hides
the native surah frame the same way, through the paths `surah_name_ornament_paths()` lists.
These rules live on `LAYER_ENGINE`, a reserved layer above every host layer. A host cannot
add a rule there. `qvp_style_clear` and `qvp_style_clear_layer` do not touch it. A higher
layer always wins, so no host rule shows the printed ring again, whatever its selector.
`undress` removes both rules by handle. The layout's `surah_frames` knob is untouched and
independent.

### 6. The layout re-places the dress

Both layout constructors end by calling `redress()`. It rebuilds the display list against
the new layout and bumps `revision` on every rebuild: a new dress, and every layout. A host caches the ornament raster against `revision`.

### 7. Overflow and view box

`view_box()` is `[x, y, w, h]` in page units, the page's own box or the box the border grew
it to. `dress_overflow()` is `left, top, right, bottom` in viewport px through the current
layout, zero on an undressed page. Both are reported by `dress_info`.

## API

### Rust (`qvp-core`)

```rust
impl Page {
    pub fn dress(&mut self, set: &Arc<OrnamentSet>, spec: &DressSpec) -> Option<&Dress>;
    pub fn undress(&mut self);
    pub fn dress_of(&self) -> Option<&Dress>;
    pub fn view_box(&self) -> [f32; 4];
    pub fn dress_overflow(&self) -> (f32, f32, f32, f32);
}
pub struct Dress {
    pub ops: Vec<u8>, pub pts: Vec<f32>, pub table: Vec<OrnamentDraw>,
    pub view_box: [f32; 4],
    pub ayah_mark: u32, pub surah_header: u32, pub page_frame: u32,
    pub n_ayah_marks: u32, pub n_surah_headers: u32, pub n_frame_repeats: u32,
    pub frame_stretched: bool, pub revision: u32,
}
```

### C ABI (10 symbols)

```c
typedef struct QvpOrnaments QvpOrnaments;
enum { QVP_ORNAMENT_AYAH_MARK = 0, QVP_ORNAMENT_SURAH_HEADER, QVP_ORNAMENT_PAGE_FRAME };
enum { QVP_LINEAGE_SCAN = 0, QVP_LINEAGE_FONT };
enum { QVP_DRAW_EVENODD = 1 << 8, QVP_DRAW_STROKE = 2 << 8 };

typedef struct { uint32_t kind, lineage, n_parts, has_slices, redistributable, _pad;
                 QvpStr style, riwayah, license_id, license_status, attribution; } QvpOrnament;
typedef struct { uint32_t color, stroke; QvpStr name; } QvpOrnamentPart;
typedef struct { uint32_t ayah_mark, surah_header, page_frame; float gap, band_gap, ayah_mark_scale, page_frame_scale; uint32_t line_art /* QVP_LINE_ART_* bits */;
                 const uint32_t* colors; /* (ornament, part, rgba) triples */ uint32_t n_colors; } QvpDressSpec;
typedef struct { uint32_t ayah_mark, surah_header, page_frame, n_ayah_marks, n_surah_headers,
                 n_frame_repeats, frame_stretched, n_draws, revision;
                 float view_box[4]; float overflow[4]; } QvpDressInfo;
typedef struct { const uint8_t* ops; uint32_t ops_len; const float* pts; uint32_t pts_len;
                 const uint32_t* table; uint32_t n_draws; } QvpDressGeometry;

QvpOrnaments* qvp_ornaments_load(const uint8_t* bytes, size_t len);
void      qvp_ornaments_free(QvpOrnaments*);
uint32_t  qvp_ornament_count(const QvpOrnaments*);
int       qvp_ornament_info(const QvpOrnaments*, uint32_t i, QvpOrnament* out);
int32_t   qvp_ornament_find(const QvpOrnaments*, uint32_t kind, const uint8_t* style, uint32_t len);
int       qvp_ornament_part(const QvpOrnaments*, uint32_t i, uint32_t part, QvpOrnamentPart* out);
int       qvp_dress(QvpPage*, const QvpOrnaments*, const QvpDressSpec*);
void      qvp_undress(QvpPage*);
int       qvp_dress_info(const QvpPage*, QvpDressInfo* out);
void      qvp_dress_geometry(const QvpPage*, QvpDressGeometry* out);
```

Strings in `QvpOrnament` and `QvpOrnamentPart` live as long as the set. The dress geometry
table is `n_draws × 9` `uint32`, the fields of `OrnamentDraw` in order, `stroke_width` as
float bits. It is valid until the next dress, undress or layout.

Against PR #27 this drops `qvp_content_box`, `qvp_page_view_box` and `qvp_dress_overflow`.
The view box and the overflow are fields of the info struct.

### Web (`web/qvp.js`)

```js
const set = engine.loadOrnaments(bytes);         // QvpOrnaments: count, list(), find(kind, style), part(i, p), free()
page.dress(set, { ayahMark: i, surahHeader: j, pageFrame: k, gap: 5, lineArt: ['surah_header'], colors: [[k, 2, '#7a1f3d']] });
page.dressInfo();        // {ayahMark, surahHeader, pageFrame, ayahMarks, surahHeaders, frameRepeats, frameStretched, draws, revision, viewBox, overflow}
page.dressGeometry();    // [{path, color, kind, evenOdd, stroke, strokeWidth, part, placement}]
page.undress();
```

`CanvasRenderer` paints the dress layer behind the base ink. It rasterises the layer once
per `revision` and view scale, and draws every ornament through the same placement
transform the page paths use. The example app gets a picker per kind, a line-art toggle,
the gap, part swatches, and the licence line of each chosen ornament.

### Other wrappers

Android, iOS, Flutter and React Native are not bound in this change. Each symbol gets a
declared gap in `docs/API-PARITY.md` with one issue per wrapper. The ABI test and the web
binding test name every symbol.

## The converter

```
qvp-convert ornaments <quran-assets dir> <out.qvo> [--redistributable] [--style NAME]...
```

Reads `catalog.json` (`quran-assets/catalog@1`) and the colour SVG of every asset. With
`--redistributable` only assets whose licence status is `confirmed` go in. `--style` limits
the build to named styles. A frame with slices contributes its corner and two edges; its
whole-frame outlines are dropped and its view box and slot are kept. Each asset's palette
comes from the catalogue, and the walk assigns every outline to the part its `data-part`
names. The report lists what went in and warns about what did not.

QVO1 has no arc operation, so arcs are the one place the ornament converter is not
lossless. `shape_d` writes `<ellipse>`, `<circle>` and a rounded `<rect>` as arcs, and
`svgtypes::SimplifyingPathParser` turns every arc into cubics, as it does an `A` or `a` in
a `d`. It asks `kurbo` for an error of at most 0.1 asset units and uses at least one cubic
for each quarter turn. A quarter-circle cubic strays from the circle by about 0.03% of the
radius. The difference is too small to see, but the curve is not the source curve.

At `quran-assets` d4cae84 this touches 4 assets in the redistributable set, the circles
and ellipses of `ayah-markers/font-012-{extralight,light,regular-black,thin}`. It also
touches 24 provisional scanned assets: `color.svg`, `line.svg` and `mono.svg` of the
douri, hafs-adi, hafs-madinah-kabir, hafs-madinah-mumtaza, qalon, shubah, sousi and warsh
sets.

The JSON reader is the PR's hand-written `json.rs`. The converter has no JSON dependency and
this adds none.

## The format: QVO1

Byte layout goes in `docs/FORMAT.md` beside QVP1 and QVA1. Integers are little-endian,
varints and zigzag as there. Coordinates are asset units quantised by `quant` (1000).

```
magic "QVO1", u16 quant
varint n_ornaments, then per ornament:
  u8 kind, u8 lineage, str style, str riwayah,
  str license_id, str license_status, varint redistributable, str attribution,
  varint n_parts, per part: str name, u32 color, varint stroke
  asset: 4×f32 view_box, varint has_slot, [4×f32 slot], varint first_path, varint n_paths
  varint has_slices, [4×f32 corner_w corner_h repeat_h repeat_v, varint rotate, 3 assets]
varint n_paths, per path: varint part, u8 flags, zigzag stroke_width, varint op_off, varint op_len
varint n_ops, the opcode stream (the QVP1 encoding, origin 0,0)
```

`str` is a varint length and UTF-8 bytes. Bounding boxes are derived at load.

## Shipping

- `dist/pages/ornaments.qvo` is built with `--redistributable` from a pinned commit of
  `quran-ws/quran-assets`. `scripts/package-data.sh` includes it when present, and
  `VERSION.json` records the quran-assets commit. `docs/DATA.md` gets a row.
- `scripts/sync-test-data.sh` fetches it from the data release when the release has it.
  Tests that need it skip without it, as the ABI test skips without pages.
- No package ships an ornament set. Scanned styles stay a local build until their licence
  settles. The engine reports each ornament's licence so a host can say so on screen.

## Testing

- `qvp-format`: round trip of a synthetic set, rejection of foreign bytes.
- `qvp-core`: the transform helpers; placement on page 036 with a synthetic set: one draw
  per real medallion, one per surah band, the border around the laid-out box, the view box
  and overflow under padding, re-placement after a taller layout, the medallion's placement
  equal to its ring path's group under reflow, undress restoring the rings.
- `qvp-ffi/tests/abi.rs`: load a set, dress, read info and geometry, undress.
- `web/smoke.mjs`: the same through `qvp.js`.
- `scripts/check-parity.py` with the declared gaps.
- The terminology audit before any new Quranic name.

## Docs and bookkeeping

`docs/API.md` gets a Dress section. `docs/FORMAT.md` gets QVO1. `docs/API-PARITY.md`
gets the gaps. `CHANGELOG.md` gets a line under Unreleased. `.terminology.json` records the
catalogue's own external names.

## Branch mechanics

The work starts on a fresh branch from `main`. The PR's three iOS commits are already on
main. When the work is done the branch is force-pushed to `dress`, which updates PR #27 in
place and keeps its discussion.
