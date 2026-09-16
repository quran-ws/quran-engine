//! The default values the engine and every wrapper agree on. The C header repeats them
//! as `QVP_DEFAULT_*` and each wrapper as `QvpDefaults`; `scripts/check-parity.py`
//! fails when any copy differs from this file.

use crate::Rgba;

/// Ink colour of a page before any style rule.
pub const INK: Rgba = 0x231f20ff;
/// Highlight ink and band colours, band padding and seam overlap (page units).
pub const HIGHLIGHT_INK: Rgba = 0x1a73e8ff;
pub const HIGHLIGHT_BAND: Rgba = 0xd6a3264d;
pub const HIGHLIGHT_PAD_X: f32 = 1.2;
pub const HIGHLIGHT_PAD_Y: f32 = 0.0;
pub const HIGHLIGHT_SEAM: f32 = 0.25;
/// Band colour a view uses for the word selection.
pub const SELECTION_BAND: Rgba = 0x2d6fd640;
/// Share of the gap between two words that belongs to the preceding word.
pub const GAP_BIAS: f32 = 0.6;
/// How far (page units) a tap may land from the ink and still hit the nearest word.
pub const TAP_DISTANCE: f32 = 6.0;
/// The line grid this mushaf is designed on.
pub const GRID_LINES: u32 = 15;
/// The content-width bound the examples pass as `max_aspect_slack`.
pub const ASPECT_SLACK: f32 = 1.15;
/// Mask block colour, padding and corner radius (page units).
pub const MASK_BLOCK: Rgba = 0xd9d4c8ff;
pub const MASK_PAD: f32 = 0.6;
pub const MASK_RADIUS: f32 = 0.8;
/// Reveal: steps lit at once, and the colour of the greyed page.
pub const REVEAL_LIT: u32 = 1;
pub const REVEAL_GREY: Rgba = 0xc9c4b8ff;
/// Padding (page units) around a crop.
pub const CROP_PAD: f32 = 2.0;

/// How far a row that comes out much shorter than the row beside it is opened towards it, as
/// a share of the difference. Rows of a page vary in width, but one row far shorter than its
/// neighbours reads as a mistake rather than as a line ending.
pub const REFLOW_RELAX: f32 = 0.5;

/// How far a reflowed row's gaps may open, as a multiple of the air the page keeps between two
/// words. This is what bounds relaxing and justification alike, and it is why a row of few
/// words opens less than a row of many: fewer gaps, less room, before the words stand apart.
/// At this cap the gaps of a relaxed row run at about 4.5 page units against the print's 4.1.
pub const REFLOW_MAX_STRETCH: f32 = 2.0;

/// The reader's pinch limits, as multiples of the layout's own scale, and the point at which
/// a page counts as zoomed rather than settled.
pub const MIN_ZOOM: f32 = 0.5;
pub const MAX_ZOOM: f32 = 12.0;
pub const ZOOMED_THRESHOLD: f32 = 1.02;
/// How far past the midpoint between two zoom steps a pinch must reach before it commits to
/// the next one, either way. The midpoint is where the steps meet; this holds a finger resting
/// on it from flickering between two layouts.
///
/// It cannot simply be raised. `Page::zoom_levels` leaves neighbouring steps at least 1.08
/// apart, and the two thresholds of one step stay clear of each other only while
/// `(1 + this)² < 1.08`.
pub const ZOOM_SNAP_HYSTERESIS: f32 = 0.03;

/// The smallest change in reflow zoom a free pinch asks the page for. Below this the rows come
/// out the same and the layout is work no reader can see: at the sizes a reader uses, a
/// hundredth is under half a percent.
pub const ZOOM_QUANTUM: f32 = 0.01;

/// A released drag is a page swipe when it is this much more sideways than up and down, and
/// either this far (viewport px) or this fast (px per second).
pub const SWIPE_AXIS_RATIO: f32 = 1.5;
pub const SWIPE_DISTANCE: f32 = 40.0;
pub const SWIPE_VELOCITY: f32 = 500.0;

/// How far two words' strokes must overlap, as a share of the printed line spacing, for the
/// pair to count as one piece of calligraphy rather than two words set close. Strokes almost
/// never meet: 10 pairs of 68,612 in this mushaf, and the three the print draws as one
/// (`ٱلرَّحْمَٰنِ ٱلرَّحِيمِ`) overlap by 19 to 26 page units where the next deepest reaches 3.8.
pub const INTERLOCK_DEPTH: f32 = 0.15;

/// How many straight pieces a curve is walked as when tracing a word's silhouette.
pub const CURVE_STEPS: u32 = 8;

/// How many bands a line is sliced into when measuring the air between two words' letters.
///
/// A band reports one leftmost and one rightmost point for the whole of its height, so a tall
/// band compares ink that does not face ink: at 24 bands a pair the measure called 2.3 apart
/// had strokes crossing by 4.6. These bands are about a third of a page unit.
pub const SLICES_PER_LINE: u32 = 96;

/// The revision of the layout itself: how words are broken onto rows, how the air between two
/// words is measured, and how a short row is opened up. Raise it whenever a change to any of
/// those moves words, so the shipped zoom steps (`zoom_table`) are known to be stale. The
/// settings those steps were searched with are recorded beside it, and the regeneration check
/// in `scripts/check.sh gates` catches a change that reaches the rows without passing here.
pub const LAYOUT_REVISION: u32 = 1;

/// The zoom each step of the reader's zoom control aims at, above the printed page. The engine
/// searches around these for the zoom that breaks the page best (`Page::zoom_levels`).
pub const ZOOM_LEVEL_NOMINALS: [f32; 3] = [1.4, 1.8, 2.2];
/// How far either side of a nominal the search may go, as a fraction of it. A wider band finds
/// better rows and makes the ink change size more from one page to the next.
pub const ZOOM_LEVEL_BAND: f32 = 0.06;
/// The search step, in zoom.
pub const ZOOM_LEVEL_STEP: f32 = 0.015;

/// How many rows either side of a row are compared with it when a short row is opened up
/// (`ReflowSpec::relax`). A fixed count, so the same page is set the same way on any screen.
pub const RELAX_NEIGHBOURS: usize = 3;
