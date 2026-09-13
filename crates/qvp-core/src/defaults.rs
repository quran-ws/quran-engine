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
