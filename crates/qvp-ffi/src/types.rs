//! The `#[repr(C)]` structs that mirror the header's, shared by more than one module.

use crate::abi::QvpStr;

#[repr(C)]
pub struct QvpPageInfo {
    pub width: f32,
    pub height: f32,
    pub page: u32,
    pub n_lines: u32,
    pub n_ayahs: u32,
    pub n_words: u32,
    pub n_paths: u32,
    pub n_decorations: u32,
}

#[repr(C)]
pub struct QvpGeometry {
    pub ops: *const u8,
    pub ops_len: u32,
    pub pts: *const f32,
    pub pts_len: u32,
    /// `n_paths` records of 8 × u32: op_start, op_count, pt_start, pt_count, flags, word, line, extra
    pub table: *const u32,
    pub n_paths: u32,
}

#[repr(C)]
pub struct QvpWordInfo {
    pub surah: u16,
    pub ayah: u16,
    pub word: u16,
    pub line_number: u16,
    pub ayah_index: u32,
    pub line_index: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub text: QvpStr,
    pub first_path: u32,
    pub n_paths: u32,
}

#[repr(C)]
pub struct QvpAyahInfo {
    pub surah: u16,
    pub ayah: u16,
    pub fragment: u8,
    pub fragments: u8,
    pub flags: u8,
    pub _pad: u8,
    pub rubu_al_hizb: u16,
    pub first_word: u32,
    pub n_words: u32,
    pub ayah_mark_decoration: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[repr(C)]
pub struct QvpLineInfo {
    pub line_number: u8,
    pub is_header: u8,
    pub first_word: u32,
    pub n_words: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    /// line-spacing band and body centre (page units)
    pub band_y0: f32,
    pub band_y1: f32,
    pub centre: f32,
}

#[repr(C)]
pub struct QvpDecorationInfo {
    pub decoration: u8,
    pub _pad: u8,
    pub surah: u16,
    pub ayah: u16,
    pub _pad2: u16,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub text: QvpStr,
    pub first_path: u32,
    pub n_paths: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpHit {
    pub word: u32,
    pub path: u32,
    pub decoration: u32,
    pub line: u32,
    pub distance: f32,
    pub is_exact: u8,
}

#[repr(C)]
pub struct QvpHitOptions {
    pub max_distance: f32,
    pub gap_bias: f32,
    pub prefer_exact: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpBox {
    pub id: u32,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub color: u32,
    pub radius: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpHitArea {
    pub word: u32,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub ink_x0: f32,
    pub ink_y0: f32,
    pub ink_x1: f32,
    pub ink_y1: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpLineBand {
    pub line: u32,
    pub line_number: u32,
    pub y0: f32,
    pub y1: f32,
    pub mid: f32,
    pub ink_y0: f32,
    pub ink_y1: f32,
}

#[repr(C)]
pub struct QvpLayoutSpec {
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub pad_top: f32,
    pub pad_bottom: f32,
    pub pad_left: f32,
    pub pad_right: f32,
    pub line_spacing: f32,
    pub fill_height: u8,
    pub grid_lines: u32,
    pub crop_left: f32,
    pub crop_right: f32,
    pub max_aspect_slack: f32,
    /// How much bigger the ink is than fit-to-width when the words are broken onto rows of
    /// the page's own width. 0 lays the page out as printed (no reflow).
    pub reflow_zoom: f32,
    /// 0 ragged, 1 justified, 2 centred, 255 the engine's own default
    pub reflow_fill: u8,
    /// how the words are broken onto rows: 0 greedy, 1 even, 255 the engine's own default
    pub reflow_breaks: u8,
    /// 0 the printed gap between the two words, 1 the page's median gap between letters
    pub reflow_gaps: u8,
    /// multiplier on every gap (1 = the gap `reflow_gaps` picked)
    pub reflow_word_gap: f32,
    /// how far a justified row's gaps may stretch, as a multiple of the gaps it started with
    /// (0 = the engine's default, a negative value = no cap)
    pub reflow_max_stretch: f32,
    /// how far a row much shorter than the row beside it is opened towards it, 0 to 1
    /// (a negative value = the engine's default)
    pub reflow_relax: f32,
    /// how big a banner (a surah name, a basmalah) may get as the reader zooms in, as a
    /// multiple of its printed size (0 or below = uncapped, which grows it with the words
    /// around it until it fills the row; 1 holds it at the printed size). It is not part of
    /// the reflow knobs: the zoom control fills those in itself, so a host that pinches would
    /// never get to set it there.
    pub banner_zoom: f32,
    /// 1 draws source-native surah frames on the printed page; 0 omits them. Reflow omits them.
    pub surah_frames: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpGrid {
    pub lines: u32,
    pub line_spacing: f32,
}

#[repr(C)]
pub struct QvpLayout {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub content_w: f32,
    pub content_h: f32,
    pub line_spacing: f32,
    pub n_lines: u32,
    /// n_lines × {dy, slot_top, slot_bottom}; valid until the next qvp_layout call on this thread
    pub lines: *const f32,
    /// The view transform for the content: draw at fit_x + fit_scale·view_x,
    /// fit_y + fit_scale·view_y, and the host's pan and zoom go on top. The fit shows a box:
    /// the content, plus the dress overflow on each side when the page is dressed. On a
    /// printed page, fit_scale shrinks the box to fit both the viewport width and height and
    /// is never more than 1, and the box is centred. On a reflowed page, fit_scale is 1 and
    /// the host scrolls: the box is centred across the viewport, and its top is at the top
    /// of the viewport. fit_x, fit_y place the content's own origin, so the overflow sits to
    /// their left and above them.
    pub fit_scale: f32,
    pub fit_x: f32,
    pub fit_y: f32,
    /// 1 when the words were broken onto rows of their own, 0 when the page is as printed.
    pub reflowed: u32,
    /// Rows the reflow produced (0 when not reflowed). `lines` then holds the rows.
    pub n_rows: u32,
}

/// kind: 0 Page, 1 Word(a), 2 Words(words,n), 3 Ayah(a,b), 4 AyahRange(a,b,c), 5 Line(a), 6 Surah(a), 7 Range(a,b)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpTarget {
    pub target: u8,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub words: *const u32,
    pub n_words: u32,
}

/// kind: 0 Page, 1 Path(a), 2 WordPath(a,b), 3 WordMark(a,b), 4 WordMarkNamed(a, mark b, nth c),
/// 5 WordBody(a), 6 WordMarks(a), 7 Word(a), 8 Ayah(a,b), 9 Line(a), 10 Mark(a), 11 Category(a),
/// 12 Family(a), 13 Kind(a), 14 Deco(a), 15 DecoIdx(a), 16 Letter(a, letter b, part c),
/// 17 LetterMark(a, letter b, nth c), 18 LetterMarkNamed(a, letter b, mark c), 19 WordChars(a, from b, to c)
#[repr(C)]
pub struct QvpSelector {
    pub selector: u8,
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

/// One letter of a word (`qvp_letter_info`). Offsets are `QVP_NONE` when absent. 52 bytes on
/// every target, with no implicit padding.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpLetterInfo {
    pub word: u32,
    pub index: u32,
    pub at: u32,
    /// the letter's code point
    pub text: u32,
    pub n_members: u32,
    pub n_marks: u32,
    /// the first other offset the body draws
    pub more_at: u32,
    pub n_more_at: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub has_ink: u8,
    pub _pad: [u8; 3],
}

/// One path of a letter (`qvp_letter_members`). 12 bytes, no implicit padding.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpLetterMember {
    pub path: u32,
    pub at: u32,
    pub mark: u8,
    pub is_body: u8,
    pub is_cut: u8,
    pub _pad: u8,
}

/// The letter under a point (`qvp_hit_test_letter`). 28 bytes, no implicit padding.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpLetterHit {
    pub word: u32,
    pub letter: u32,
    pub path: u32,
    pub at: u32,
    pub line: u32,
    pub distance: f32,
    pub mark: u8,
    pub is_exact: u8,
    /// 1 when `path` is a body member of its letter
    pub is_body: u8,
    pub _pad: u8,
}

/// Letters by what they are (`qvp_find_letters`, `qvp_style_add_letters`). 40 bytes on a
/// 64-bit target and 32 on wasm32, as `QvpTarget` is 32 or 24. Its own fields leave no hole,
/// but the `QvpTarget` in it has the holes of that struct. It is input only: the engine reads
/// it and never writes it, so no byte of it goes back to the host.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpLetterQuery {
    pub scope: QvpTarget,
    /// a code point; 0 = any letter
    pub text: u32,
    /// a mark id the letter must carry; 0 = any
    pub mark: u8,
    /// `QVP_LETTER_PART_*`
    pub part: u8,
    pub _pad: [u8; 2],
}

// The three letter structs the engine writes (info, member, hit) have no implicit padding, so no
// byte the engine leaves unwritten goes back to the host. `QvpLetterQuery` is input only and keeps
// the holes of its `QvpTarget`. The sizes are checked when each target builds, wasm32 included;
// tests/abi.rs checks every offset.
const _: () = {
    use std::mem::size_of;
    assert!(size_of::<QvpLetterInfo>() == 52);
    assert!(size_of::<QvpLetterMember>() == 12);
    assert!(size_of::<QvpLetterHit>() == 28);
    assert!(size_of::<QvpLetterQuery>() == size_of::<QvpTarget>() + 8);
    assert!(size_of::<QvpTarget>() == if cfg!(target_pointer_width = "64") { 32 } else { 24 });
};

#[repr(C)]
pub struct QvpHighlightStyle {
    pub mode: u8,
    pub height: u8,
    pub ink: u32,
    pub band: u32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub radius: f32,
    pub seam: f32,
    pub transition_ms: u32,
    pub layer: i32,
}

/// Colours with alpha 0 mean "leave alone".
#[repr(C)]
pub struct QvpTheme {
    pub ink: u32,
    pub diacritics: u32,
    pub dots: u32,
    pub waqf: u32,
    pub sifr: u32,
    pub ayah_mark: u32,
    pub numeral: u32,
    pub headers: u32,
    pub transition_ms: u32,
    /// pairs of (mark, colour)
    pub marks: *const u32,
    pub n_marks: u32,
}

#[repr(C)]
pub struct QvpSurah {
    pub number: u16,
    pub ayah_count: u16,
    pub has_banner: u8,
    pub has_basmalah: u8,
    pub place: u8,
    pub _pad: u8,
    pub banner_decoration: u32,
    pub arabic: QvpStr,
    pub latin: QvpStr,
    pub english: QvpStr,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpDivision {
    pub division: u8,
    pub line: u8,
    pub number: u16,
    pub surah: u16,
    pub ayah: u16,
    pub ayah_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpAyahMark {
    pub decoration: u32,
    pub surah: u16,
    pub ayah: u16,
    pub line: u32,
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub ornament_path: u32,
    pub numeral_path: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpSurahHeader {
    pub decoration: u32,
    pub surah: u16,
    pub _pad: u16,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub title_x0: f32,
    pub title_y0: f32,
    pub title_x1: f32,
    pub title_y1: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpRosette {
    pub decoration: u32,
    pub surah: u16,
    pub ayah: u16,
    pub juz: u16,
    pub hizb: u16,
    pub nisf: u16,
    pub rubu_al_hizb: u16,
    pub rubu_al_hizb_in_hizb: u16,
    pub _pad: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpSajdah {
    pub decoration: u32,
    pub surah: u16,
    pub ayah: u16,
    pub sign_path: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpMatch {
    pub word: u32,
    pub index: u32,
    pub is_loose_match: u8,
}

#[repr(C)]
pub struct QvpCropBounds {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub n_words: u32,
    pub ayah_mark_decoration: u32,
}

#[repr(C)]
pub struct QvpAtlasSurah {
    pub number: u16,
    pub first_page: u16,
    pub ayah_count: u16,
    pub place: u8,
    pub _pad: u8,
    pub arabic: QvpStr,
    pub latin: QvpStr,
    pub english: QvpStr,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpAtlasRubuAlHizb {
    pub rubu_al_hizb: u16,
    pub surah: u16,
    pub ayah: u16,
    pub page: u16,
}
