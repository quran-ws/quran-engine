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

/// How far a reflowed row's gaps may stretch to reach the margins, as a multiple of the gaps
/// the row started with. Past this the row is left right-aligned.
pub const REFLOW_MAX_STRETCH: f32 = 1.6;

/// The reader's pinch limits, as multiples of the layout's own scale, and the point at which
/// a page counts as zoomed rather than settled.
pub const MIN_ZOOM: f32 = 0.5;
pub const MAX_ZOOM: f32 = 12.0;
pub const ZOOMED_THRESHOLD: f32 = 1.02;
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

/// How many bands a line is sliced into when measuring the air between two words' letters.
pub const SLICES_PER_LINE: u32 = 24;
