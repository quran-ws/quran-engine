/* QVP engine C ABI — matches crates/qvp-ffi/src/lib.rs (keep in sync).
 *
 * Conventions
 *  - colours are 0xRRGGBBAA; alpha 0 = hidden / "leave alone"
 *  - QVP_NONE (0xFFFFFFFF) = absent index
 *  - strings come back as QvpStr {ptr,len}: UTF-8, not NUL-terminated, valid until the next
 *    string-returning call on the same thread (QvpWordInfo.text / QvpDecorationInfo.text live as long as the page)
 *  - array outputs take (out, cap) and return the TOTAL count (may exceed cap: call again bigger)
 *  - coordinates are page units (viewBox space, y down) unless the name says "view" (viewport px
 *    through the current layout: view_x = offset_x + x*scale ; view_y = offset_y + (y + dy[line])*scale)
 */
#ifndef QVP_H
#define QVP_H
#include <stdint.h>
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif

#define QVP_NONE 0xFFFFFFFFu
/* Defaults the engine and every wrapper agree on (crates/qvp-core/src/defaults.rs is the source;
   the parity check compares every copy). Colours are 0xRRGGBBAA, lengths page units. */
#define QVP_DEFAULT_INK            0x231f20ffu
#define QVP_DEFAULT_HIGHLIGHT_INK  0x1a73e8ffu
#define QVP_DEFAULT_HIGHLIGHT_BAND 0xd6a3264du
#define QVP_DEFAULT_HIGHLIGHT_PAD_X 1.2f
#define QVP_DEFAULT_HIGHLIGHT_PAD_Y 0.0f
#define QVP_DEFAULT_HIGHLIGHT_SEAM 0.25f
#define QVP_DEFAULT_MIN_ZOOM       0.5f
#define QVP_DEFAULT_MAX_ZOOM       12.0f
#define QVP_DEFAULT_ZOOMED_THRESHOLD 1.02f
#define QVP_DEFAULT_ZOOM_SNAP_HYSTERESIS 0.03f
#define QVP_DEFAULT_ZOOM_QUANTUM   0.01f
#define QVP_DEFAULT_SWIPE_AXIS_RATIO 1.5f
#define QVP_DEFAULT_SWIPE_DISTANCE 40.0f
#define QVP_DEFAULT_SWIPE_VELOCITY 500.0f
#define QVP_DEFAULT_SELECTION_BAND 0x2d6fd640u
#define QVP_DEFAULT_GAP_BIAS       0.6f
#define QVP_DEFAULT_TAP_DISTANCE   6.0f
#define QVP_DEFAULT_GRID_LINES     15u
#define QVP_DEFAULT_ASPECT_SLACK   1.15f
#define QVP_DEFAULT_MASK_BLOCK     0xd9d4c8ffu
#define QVP_DEFAULT_MASK_PAD       0.6f
#define QVP_DEFAULT_MASK_RADIUS    0.8f
#define QVP_DEFAULT_REVEAL_LIT     1u
#define QVP_DEFAULT_REVEAL_GREY    0xc9c4b8ffu
#define QVP_DEFAULT_CROP_PAD       2.0f
typedef struct QvpPage QvpPage;
typedef struct QvpAtlas QvpAtlas;
typedef struct { const uint8_t* ptr; uint32_t len; } QvpStr;

/* enums --------------------------------------------------------------------------------- */
enum { QVP_KIND_BODY = 0, QVP_KIND_MARK, QVP_KIND_AYAH_NUMBER, QVP_KIND_AYAH_MARK_ORNAMENT, QVP_KIND_HEADER_INK,
       QVP_KIND_ORNAMENT, QVP_KIND_PAGE_NUMBER, QVP_KIND_RUNNING_HEAD, QVP_KIND_OTHER = 255 };
enum { QVP_FAMILY_NONE = 0, QVP_FAMILY_DIACRITIC, QVP_FAMILY_TANWIN, QVP_FAMILY_DOTS, QVP_FAMILY_WAQF, QVP_FAMILY_SIFR, QVP_FAMILY_SAJDAH, QVP_FAMILY_READING_SIGN };
enum { QVP_CATEGORY_NONE = 0, QVP_CATEGORY_HARAKAH, QVP_CATEGORY_TANWIN, QVP_CATEGORY_LETTER_DOT, QVP_CATEGORY_ORTHOGRAPHIC, QVP_CATEGORY_DABT, QVP_CATEGORY_WAQF, QVP_CATEGORY_READING_SIGN, QVP_CATEGORY_STANDALONE };
enum { QVP_DECORATION_AYAH_MARK = 0, QVP_DECORATION_SURAH_NAME, QVP_DECORATION_BASMALAH, QVP_DECORATION_DIVISION_MARK, QVP_DECORATION_SAJDAH_MARK,
       QVP_DECORATION_PAGE_NUMBER, QVP_DECORATION_RUNNING_HEAD, QVP_DECORATION_OTHER = 255 };
enum { QVP_FORM_RASM_UTHMANI = 0, QVP_FORM_RASM_IMLAI, QVP_FORM_QPC, QVP_FORM_RASM, QVP_FORM_SEARCH };
enum { QVP_SEARCH_INCLUDES = 0, QVP_SEARCH_EXACT, QVP_SEARCH_PREFIX };
enum { QVP_ARABIC_STRIP = 0, QVP_ARABIC_FOLD, QVP_ARABIC_NORMALIZE, QVP_ARABIC_LOOSE, QVP_ARABIC_SEARCH_KEY };
enum { QVP_TARGET_PAGE = 0, QVP_TARGET_WORD, QVP_TARGET_WORDS, QVP_TARGET_AYAH, QVP_TARGET_AYAH_RANGE, QVP_TARGET_LINE, QVP_TARGET_SURAH, QVP_TARGET_RANGE };
enum { QVP_SELECTOR_PAGE = 0, QVP_SELECTOR_PATH, QVP_SELECTOR_WORD_PATH, QVP_SELECTOR_WORD_MARK, QVP_SELECTOR_WORD_MARK_NAMED, QVP_SELECTOR_WORD_BODY, QVP_SELECTOR_WORD_MARKS,
       QVP_SELECTOR_WORD, QVP_SELECTOR_AYAH, QVP_SELECTOR_LINE, QVP_SELECTOR_MARK, QVP_SELECTOR_CATEGORY, QVP_SELECTOR_FAMILY, QVP_SELECTOR_KIND, QVP_SELECTOR_DECORATION, QVP_SELECTOR_DECORATION_INDEX };
enum { QVP_HIGHLIGHT_INK = 0, QVP_HIGHLIGHT_BAND, QVP_HIGHLIGHT_BOTH };
enum { QVP_BAND_LINE_SPACING = 0, QVP_BAND_INK };
enum { QVP_MASK_HIDE = 0, QVP_MASK_BLOCK, QVP_MASK_BLUR };
enum { QVP_LAYER_BASE = 0, QVP_LAYER_THEME = 10, QVP_LAYER_HIGHLIGHT = 50, QVP_LAYER_SELECTION = 60, QVP_LAYER_TOP = 100 };
enum { QVP_DIVISION_JUZ = 0, QVP_DIVISION_HIZB, QVP_DIVISION_NISF, QVP_DIVISION_RUBU_AL_HIZB };
/* mark ids: qvp_mark_name / qvp_mark_from_name. The names are the quran-svg
   `mark-taxonomy` v2 vocabulary (the bundle's schema/mark-taxonomy.json):
   0 none, 1 fathah, 2 kasrah, 3 dammah, 4 tanwin_al_fath, 5 tanwin_al_kasr, 6 tanwin_al_damm,
   7 shaddah, 8 sukun, 9 maddah, 10 hamzah, 11 hamzat_al_wasl, 12 omitted_alif, 13 small_waw,
   14 small_yaa, 15 small_noon, 16 dot, 17 two_dots, 18 three_dots, 19 rounded_zero,
   20 rectangular_zero, 21 waqf_jaiz_mustawi_al_tarafayn, 22 waqf_jaiz_waqf_awla,
   23 waqf_jaiz_wasl_awla, 24 waqf_lazim, 25 waqf_al_muanaqah, 26 saktah, 27 small_meem,
   28 hizb, 29 sajdah, 30 sajdah_mark, 31 sajdah_line, 32 seen_al_qiraah, 33 tashil,
   34 ishmam, 35 imalah, 255 unknown */

/* structs ------------------------------------------------------------------------------- */
typedef struct { float width, height; uint32_t page, n_lines, n_ayahs, n_words, n_paths, n_decorations; } QvpPageInfo;
/* table: n_paths × 8 uint32: op_start, op_count, pt_start, pt_count, flags, word, line, extra
   flags = kind | mark<<8 | family<<16 | (evenodd?1:0)<<24 | (glyph_instance?2:0)<<24
   extra = category | nth_in_word<<8 | nth_mark<<16 (0xff = not a mark)
   ops: 0=MoveTo 1=LineTo 2=QuadTo 3=CubicTo 4=Close ; pts: x,y pairs consumed in order */
typedef struct { const uint8_t* ops; uint32_t ops_len; const float* pts; uint32_t pts_len; const uint32_t* table; uint32_t n_paths; } QvpGeometry;
typedef struct { uint16_t surah, ayah, word, line_number; uint32_t ayah_index, line_index; float x0, y0, x1, y1; QvpStr text; uint32_t first_path, n_paths; } QvpWordInfo;
typedef struct { uint16_t surah, ayah; uint8_t fragment, fragments, flags, _pad; uint16_t rubu_al_hizb; uint32_t first_word, n_words, ayah_mark_decoration; float x0, y0, x1, y1; } QvpAyahInfo;
typedef struct { uint8_t line_number, is_header; uint32_t first_word, n_words; float x0, y0, x1, y1, band_y0, band_y1, centre; } QvpLineInfo;
typedef struct { uint8_t decoration /* QVP_DECORATION_* */, _pad; uint16_t surah, ayah, _pad2; uint32_t line; float x0, y0, x1, y1; QvpStr text; uint32_t first_path, n_paths; } QvpDecorationInfo;
/* one hit result for every hit test: the exact variants set line, distance 0 and is_exact 1 */
typedef struct { uint32_t word, path, decoration, line; float distance; uint8_t is_exact; } QvpHit;
typedef struct { float max_distance /* <=0: unlimited */, gap_bias /* 0.6 */; uint8_t prefer_exact; } QvpHitOptions;
typedef struct { uint32_t id, line; float x0, y0, x1, y1; uint32_t color; float radius; } QvpBox;
typedef struct { uint32_t word, line; float x0, y0, x1, y1, ink_x0, ink_y0, ink_x1, ink_y1; } QvpHitArea;
typedef struct { uint32_t line, line_number; float y0, y1, mid, ink_y0, ink_y1; } QvpLineBand;
/* line_spacing: a multiplier on the printed spacing (1 = as printed; below 1 clamps to 1). grid_lines: the grid to lay
   the page out inside, 0 = the page's own (qvp_page_grid). crop_left/right: printed side margins to cut (page units,
   0 = none). max_aspect_slack: the content is never wider than viewport_h·page_w/page_h·slack (0 = no bound). */
/* reflow_zoom: break the words onto rows of the page's own width, with ink reflow_zoom times the size it has at
   fit-to-width (0 = lay the page out as printed). reflow_fill: 0 ragged, 1 justified, 2 centred, 255 the engine's own default. reflow_breaks: 0 greedy, 1 even, 2 fitted, 255 default.
   reflow_relax: how far a row much shorter than the row beside it is opened towards it, 0 to 1 (negative = default). reflow_gaps: 0 the printed gap
   between the two words, 1 the page's median gap. reflow_word_gap: multiplier on every gap (0 = 1). reflow_max_stretch: how far a justified row's gaps may stretch,
   as a multiple of what they started with (0 = the engine's default, negative = no cap). */
typedef struct { float viewport_w, viewport_h, pad_top, pad_bottom, pad_left, pad_right, line_spacing; uint8_t fill_height; uint32_t grid_lines; float crop_left, crop_right, max_aspect_slack, reflow_zoom; uint8_t reflow_fill, reflow_breaks, reflow_gaps; float reflow_word_gap, reflow_max_stretch, reflow_relax; } QvpLayoutSpec;
/* the grid a page is designed on: the mushaf's line count (15 here, or more when a page has more) and the printed spacing */
typedef struct { uint32_t lines; float line_spacing; } QvpGrid;
/* fit_*: the view transform that shows the whole content (shrink to the viewport height, never enlarge, centred):
   draw at fit_x + fit_scale·view_x, fit_y + fit_scale·view_y; the host's pan and zoom go on top. */
typedef struct { float scale, offset_x, offset_y, content_w, content_h, line_spacing; uint32_t n_lines; const float* lines; /* n_lines × {dy, slot_top, slot_bottom}; the rows when reflowed */ float fit_scale, fit_x, fit_y; uint32_t reflowed, n_rows; } QvpLayout;
typedef struct { uint8_t target /* QVP_TARGET_* */; uint32_t a, b, c; const uint32_t* words; uint32_t n_words; } QvpTarget;
typedef struct { uint8_t selector /* QVP_SELECTOR_* */; uint32_t a, b, c; } QvpSelector;
typedef struct { uint8_t mode, height; uint32_t ink, band; float pad_x, pad_y, radius, seam; uint32_t transition_ms; int32_t layer; } QvpHighlightStyle;
typedef struct { uint32_t ink, diacritics, dots, waqf, sifr, ayah_mark, numeral, headers, transition_ms; const uint32_t* marks /* (mark,colour) pairs */; uint32_t n_marks; } QvpTheme;
typedef struct { uint16_t number, ayah_count; uint8_t has_banner, has_basmalah, place /* 0 makkah 1 madinah */, _pad; uint32_t banner_decoration; QvpStr arabic, latin, english; } QvpSurah;
typedef struct { uint8_t division /* QVP_DIVISION_* */, line; uint16_t number, surah, ayah; uint32_t ayah_index; } QvpDivision;
typedef struct { uint32_t decoration; uint16_t surah, ayah; uint32_t line; float cx, cy, r; uint32_t ornament_path, numeral_path; } QvpAyahMark;
typedef struct { uint32_t decoration; uint16_t surah, ayah, juz, hizb, nisf, rubu_al_hizb, rubu_al_hizb_in_hizb, _pad; } QvpRosette;
typedef struct { uint32_t decoration; uint16_t surah, ayah; uint32_t sign_path; } QvpSajdah;
typedef struct { uint32_t word, index; uint8_t is_loose_match; } QvpMatch;
typedef struct { float x0, y0, x1, y1; uint32_t n_words, ayah_mark_decoration; } QvpCropBounds;
typedef struct { uint16_t number, first_page, ayah_count; uint8_t place, _pad; QvpStr arabic, latin, english; } QvpAtlasSurah;
typedef struct { uint16_t rubu_al_hizb, surah, ayah, page; } QvpAtlasRubuAlHizb;

/* memory (hosts without malloc, e.g. wasm) */
uint8_t* qvp_alloc(size_t len);
void     qvp_dealloc(uint8_t* p, size_t len);

/* page ---------------------------------------------------------------------------------- */
QvpPage* qvp_page_load(const uint8_t* bytes, size_t len);              /* NULL on error; bytes are copied */
void     qvp_page_free(QvpPage*);
void     qvp_page_info(const QvpPage*, QvpPageInfo* out);
void     qvp_geometry(const QvpPage*, QvpGeometry* out);               /* pointers live as long as the page */
int      qvp_word_info(const QvpPage*, uint32_t index, QvpWordInfo* out);
int      qvp_word_form(const QvpPage*, uint32_t index, uint8_t form, QvpStr* out);
int      qvp_ayah_info(const QvpPage*, uint32_t index, QvpAyahInfo* out);
int      qvp_line_info(const QvpPage*, uint32_t index, QvpLineInfo* out);
int      qvp_decoration_info(const QvpPage*, uint32_t index, QvpDecorationInfo* out);
int32_t  qvp_find_word(const QvpPage*, uint16_t surah, uint16_t ayah, uint16_t word);   /* -1 = not on page */
uint32_t qvp_target_words(const QvpPage*, const QvpTarget*, uint32_t* out, uint32_t cap);   /* → word indices */
float    qvp_page_line_spacing(const QvpPage*);                                     /* the printed line spacing, page units */
void     qvp_page_grid(const QvpPage*, QvpGrid* out);

/* metadata ------------------------------------------------------------------------------ */
uint32_t qvp_surah_count(const QvpPage*);
int      qvp_surah_at(const QvpPage*, uint32_t i, QvpSurah* out);
uint32_t qvp_divisions(const QvpPage*, QvpDivision* out, uint32_t cap);   /* divisions that START on this page */
uint32_t qvp_ayah_marks(const QvpPage*, QvpAyahMark* out, uint32_t cap);       /* real ayah medallions */
uint32_t qvp_ayah_marks_view(const QvpPage*, QvpAyahMark* out, uint32_t cap);  /* the same, in viewport px through the current layout */
uint32_t qvp_rosettes(const QvpPage*, QvpRosette* out, uint32_t cap);     /* drawn hizb rosettes */
uint32_t qvp_sajdahs(const QvpPage*, QvpSajdah* out, uint32_t cap);
uint32_t qvp_ayah_keys(const QvpPage*, uint32_t* out, uint32_t cap);      /* surah<<16 | ayah, reading order */
uint32_t qvp_ayah_word_count(const QvpPage*, uint16_t surah, uint16_t ayah, uint8_t* is_complete);
int32_t  qvp_recite_map(const QvpPage*, uint16_t surah, uint16_t ayah, uint32_t n_segments, uint32_t* out, uint32_t cap); /* -1 = mismatch */
void     qvp_word_label(const QvpPage*, uint32_t word_index, QvpStr* out);       /* accessibility */
void     qvp_ayah_label(const QvpPage*, uint32_t ayah_index, QvpStr* out);

/* text & search ------------------------------------------------------------------------- */
void     qvp_text(const QvpPage*, const QvpTarget* /* NULL = page */, uint8_t form, const uint8_t* word_sep, uint32_t word_sep_len, const uint8_t* line_sep, uint32_t line_sep_len, QvpStr* out);
uint32_t qvp_search(const QvpPage*, const uint8_t* query, uint32_t query_len, uint8_t form, uint8_t mode, uint8_t normalize, uint8_t loose_match, uint32_t limit /* 0 = all */, QvpMatch* out, uint32_t cap);
void     qvp_arabic(uint8_t op /* QVP_ARABIC_* */, const uint8_t* s, uint32_t len, QvpStr* out);
void     qvp_citation(const QvpPage*, const uint32_t* words, uint32_t n, QvpStr* out);   /* "2:255-257, 3:1" */
int32_t  qvp_attach_words(QvpPage*, const uint8_t* json, uint32_t len);   /* sidecar {"s:a:w": {"rasm_uthmani","rasm_imlai","qpc","rasm","search"}} → words updated, -1 = bad JSON */
uint8_t  qvp_has_form(const QvpPage*, uint8_t form);

/* hit testing --------------------------------------------------------------------------- */
int      qvp_hit_test(const QvpPage*, float x, float y, const QvpHitOptions* opt /* NULL ok */, QvpHit* out);       /* gap-aware, page units */
int      qvp_hit_test_view(const QvpPage*, float view_x, float view_y, const QvpHitOptions* opt, QvpHit* out);       /* gap-aware, viewport px */
int      qvp_hit_test_exact(const QvpPage*, float x, float y, QvpHit* out);                                /* exact outline only, page units */
int      qvp_hit_test_exact_view(const QvpPage*, float view_x, float view_y, QvpHit* out);                         /* exact outline only, viewport px */
uint32_t qvp_line_bands(const QvpPage*, QvpLineBand* out, uint32_t cap);
uint32_t qvp_hit_areas(const QvpPage*, float gap_bias, QvpHitArea* out, uint32_t cap);
uint32_t qvp_hit_areas_view(const QvpPage*, float gap_bias, QvpHitArea* out, uint32_t cap);   /* the same, in viewport px; a reflowed page's rows */

/* layout -------------------------------------------------------------------------------- */
/* The reader's pan and zoom on top of a layout: a point p in layout px draws at offset + scale·p. The engine owns this
   arithmetic so every platform pinches the same; the host owns only the gesture. */
typedef struct { float scale, offset_x, offset_y; } QvpView;
void     qvp_view_zoom_about(const QvpView*, float focal_x, float focal_y, float factor, float min /* 0 = default */, float max, QvpView* out);
void     qvp_view_pan(const QvpView*, float dx, float dy, QvpView* out);
void     qvp_view_clamp(const QvpView*, float content_w, float content_h, float viewport_w, float viewport_h, QvpView* out);
void     qvp_view_anchor(const QvpPage*, const QvpView*, uint32_t word, float nx, float ny, float to_x, float to_y, float viewport_w, float viewport_h, QvpView* out);  /* hold a word's point on screen across a relayout */
void     qvp_view_to_layout(const QvpPage*, const QvpView*, float vx, float vy, float* out /* x, y */);
int32_t  qvp_view_swipe(float dx, float dy, float vx, float vy);                 /* which way the finger went: +1 right, -1 left, 0 not a swipe */
int32_t  qvp_swipe_pages(float dx, float dy, float vx, float vy);                /* how many pages that turns, in reading order: a mushaf is read right to left */

/* The reader's zoom control: what a pinch does to the page. Stepped reflows onto the page's own zoom steps and is what a
   host gets for free (a zeroed QvpZoom is stepped, on the printed page); continuous reflows to the zoom the fingers ask
   for; magnify scales the printed page and never changes a row. The engine picks the step, lays the page out and holds
   the word under the fingers; the host owns only the gesture. */
typedef struct { uint32_t mode /* 0 stepped, 1 continuous, 2 magnify */, step; float zoom; } QvpZoom;
typedef struct { QvpZoom zoom; QvpView view; uint32_t relaid; } QvpZoomChange;
void     qvp_zoom_mode(QvpPage*, const QvpLayoutSpec*, const QvpZoom*, uint32_t mode, QvpZoom* out);   /* another policy, keeping the size the reader is at */
void     qvp_zoom_pinch(QvpPage*, const QvpLayoutSpec*, const QvpZoom*, const QvpView*, float factor /* against the fingers' distance when they went down */, float focal_x, float focal_y, QvpZoomChange* out);
void     qvp_zoom_to_step(QvpPage*, const QvpLayoutSpec*, const QvpZoom*, uint32_t step /* 0 = the printed page */, const QvpView*, QvpZoomChange* out);
void     qvp_zoom_spec(QvpPage*, const QvpLayoutSpec*, const QvpZoom*, QvpLayoutSpec* out);   /* the spec this control asks for */
void     qvp_zoom_carried(QvpPage*, const QvpLayoutSpec*, const QvpZoom*, QvpZoom* out);   /* the same control on this page: what a page turn keeps */
float    qvp_zoom_at_step(QvpPage*, const QvpLayoutSpec*, uint32_t step);        /* the reflow zoom one step means; 0 = the printed page */
int      qvp_zoom_is_zoomed(const QvpZoom*, const QvpView*, float fit_scale /* 0 = the page is at its fitted size */);   /* has the reader zoomed in, by either road */
uint32_t qvp_sideways_drag(const QvpPage*, const QvpZoom*, const QvpView*, float fit_scale);   /* what a sideways drag means: 0 pan, 1 turn the page */

void     qvp_layout(QvpPage*, const QvpLayoutSpec*, QvpLayout* out);        /* out.lines valid until next call */
int      qvp_layout_current(const QvpPage*, QvpLayout* out);                /* the layout the page already has, without computing one; 0 when it has none */
uint32_t qvp_layout_groups(const QvpPage*, float* out /* n × {dx, dy, kx, ky} */, uint32_t cap);   /* where each group of paths is placed */
uint32_t qvp_layout_repeats(const QvpPage*, float* out /* n × {first_path, n_paths, dx, dy, kx, ky} */, uint32_t cap);   /* paths drawn again elsewhere (a sajdah line over two rows) */
uint32_t qvp_layout_path_groups(const QvpPage*, uint32_t* out, uint32_t cap);    /* the group of every path; empty unless reflowed */
uint32_t qvp_layout_omitted_paths(const QvpPage*, uint32_t* out, uint32_t cap);  /* paths this layout does not draw (sheet furniture when reflowed) */
uint32_t qvp_layout_draw_list(const QvpPage*, float band_top, float band_bottom /* <= top = the whole page */, uint32_t* out /* n × {path, placement} */, uint32_t cap);   /* everything this layout draws inside a band of it, in drawing order: one loop draws any page */
uint32_t qvp_layout_placements(const QvpPage*, float* out /* n × {dx, dy, kx, ky} */, uint32_t cap);   /* what a draw list's `placement` indexes: the groups, then the repeats */
uint32_t qvp_layout_row_words(const QvpPage*, uint32_t row, uint32_t* out, uint32_t cap);     /* the words of a reflowed row */
uint32_t qvp_layout_word_row(const QvpPage*, uint32_t word);                    /* the row a word landed on, or QVP_NONE */
float    qvp_reflow_max_zoom(const QvpPage*, const QvpLayoutSpec*);              /* the largest reflow zoom whose rows still hold every word */
uint32_t qvp_zoom_levels(QvpPage*, const QvpLayoutSpec*, const float* nominals, uint32_t n_nominals,
                         float band, float* out, uint32_t n_out);                 /* the zoom each step of a zoom control lands on; returns how many were written */
uint32_t qvp_zoom_steps(QvpPage*, const QvpLayoutSpec*, float* out, uint32_t n_out);  /* the zoom steps this page ships with */
uint32_t qvp_zoom_level_candidates(QvpPage*, const QvpLayoutSpec*, float nominal, float band, float floor,
                         float* out_zoom, float* out_cost, uint32_t n_out);        /* every zoom the search weighs for one step, with its cost */
float    qvp_layout_line_spacing_to_fill(const QvpPage*, const QvpLayoutSpec*, float max /* <=0 unlimited */);   /* the multiplier that fills the padded viewport */
float    qvp_layout_wasted_fraction(const QvpPage*, const QvpLayoutSpec*);       /* share of the padded viewport left empty at fit-to-width */
int      qvp_word_bounds_view(const QvpPage*, uint32_t word_index, float out[4]);

/* styles: layered rules, handles undo exactly ------------------------------------------ */
uint32_t qvp_style_add(QvpPage*, int32_t layer, const QvpSelector*, uint32_t rgba, uint32_t transition_ms);   /* → handle (0 = bad selector) */
uint32_t qvp_style_add_target(QvpPage*, int32_t layer, const QvpTarget*, uint32_t rgba, uint32_t transition_ms);
uint32_t qvp_style_remove(QvpPage*, uint32_t handle);                       /* rules removed */
uint32_t qvp_style_recolor(QvpPage*, uint32_t handle, uint32_t rgba, uint32_t transition_ms);
void     qvp_style_clear(QvpPage*);
void     qvp_style_clear_layer(QvpPage*, int32_t layer);
void     qvp_style_default_color(QvpPage*, uint32_t rgba);
uint32_t qvp_style_hide(QvpPage*, const QvpSelector*);                            /* alpha-0 rule on the top layer */
uint32_t qvp_theme(QvpPage*, const QvpTheme*);                              /* one handle for the whole theme */
uint32_t qvp_style_handles(const QvpPage*, uint32_t* out, uint32_t cap);

/* clock & display list ------------------------------------------------------------------ */
uint8_t  qvp_tick(QvpPage*, double now_ms);                                 /* 1 while animating: keep drawing frames */
const uint32_t* qvp_colors(QvpPage*);                                       /* n_paths colours (current, mid-transition) */
uint32_t qvp_styled_paths(QvpPage*, uint32_t* out_pairs, uint32_t cap);          /* (path, colour) pairs ≠ default ink */
uint32_t qvp_color_of(QvpPage*, uint32_t path);

/* highlights: ink and/or animated bands ------------------------------------------------- */
uint32_t qvp_highlight_add(QvpPage*, const QvpTarget*, const QvpHighlightStyle* /* NULL = default band */);   /* → handle */
uint8_t  qvp_highlight_move(QvpPage*, uint32_t handle, const QvpTarget*);     /* band slides, ink fades */
uint8_t  qvp_highlight_restyle(QvpPage*, uint32_t handle, const QvpHighlightStyle*);
uint8_t  qvp_highlight_remove(QvpPage*, uint32_t handle);                       /* fades out over transition_ms */
void     qvp_highlight_clear(QvpPage*);
uint32_t qvp_highlight_handles(const QvpPage*, uint32_t* out, uint32_t cap);
uint32_t qvp_highlight_words(const QvpPage*, uint32_t handle, uint32_t* out, uint32_t cap);
uint32_t qvp_highlight_boxes_view(const QvpPage*, QvpBox* out, uint32_t cap);   /* viewport px; draw each id as ONE nonzero path, behind the ink */
uint32_t qvp_word_bands(const QvpPage*, const uint32_t* words, uint32_t n, uint8_t height, float pad_x, float pad_y, QvpBox* out, uint32_t cap);
uint32_t qvp_word_bands_view(const QvpPage*, const uint32_t* words, uint32_t n, uint8_t height, float pad_x, float pad_y, QvpBox* out, uint32_t cap);  /* the same, in viewport px */

/* selection (whole-word ranges) --------------------------------------------------------- */
void     qvp_select(QvpPage*, uint32_t anchor, uint32_t focus);             /* QVP_NONE clears */
uint32_t qvp_selection(const QvpPage*, uint32_t* out, uint32_t cap);
void     qvp_selection_text(const QvpPage*, uint8_t form, uint8_t include_citation, QvpStr* out);

/* memorisation -------------------------------------------------------------------------- */
void     qvp_mask(QvpPage*, const QvpTarget*, uint8_t mode);
void     qvp_mask_from(QvpPage*, uint32_t word_index, uint8_t mode);
void     qvp_mask_options(QvpPage*, uint32_t block_color, float pad_x, float pad_y, float radius, uint8_t reverse);
void     qvp_mask_transition(QvpPage*, uint32_t ms);                        /* hide fade; 0 = instant; reset by unmask */
uint32_t qvp_unmask_next(QvpPage*, uint32_t n);
uint32_t qvp_mask_back(QvpPage*, uint32_t n);
uint8_t  qvp_unmask_word(QvpPage*, uint32_t word_index);
uint8_t  qvp_mask_word(QvpPage*, uint32_t word_index);
void     qvp_unmask_all(QvpPage*);
void     qvp_mask_all(QvpPage*);
void     qvp_unmask(QvpPage*);
uint32_t qvp_mask_hidden(const QvpPage*, uint32_t* out, uint32_t cap);
uint32_t qvp_mask_words(const QvpPage*, uint32_t* out, uint32_t cap);
uint32_t qvp_mask_boxes_view(const QvpPage*, QvpBox* out, uint32_t cap);       /* block/blur boxes, viewport px */
uint32_t qvp_reveal_start(QvpPage*, uint32_t lit, uint8_t by_ayah, uint32_t grey, uint32_t ink, uint8_t ayah_marks, uint32_t transition_ms); /* → steps */
uint8_t  qvp_reveal_goto(QvpPage*, int64_t at);                            /* -1 = nothing lit yet */
int64_t  qvp_reveal_position(const QvpPage*);                                    /* -2 = no reveal running */
uint32_t qvp_reveal_step_count(const QvpPage*);
int64_t  qvp_reveal_step_of(const QvpPage*, uint32_t word_index);
void     qvp_reveal_stop(QvpPage*);

/* crop ---------------------------------------------------------------------------------- */
int      qvp_crop_bounds(const QvpPage*, const QvpTarget*, float pad, uint8_t keep_ayah_marks, QvpCropBounds* out);
int      qvp_crop_svg(QvpPage*, const QvpTarget*, float pad, uint8_t keep_ayah_marks, uint32_t background, QvpStr* out);

/* atlas (cross-page lookup; atlas.qva from the converter) -------------------------------- */
QvpAtlas* qvp_atlas_load(const uint8_t* bytes, size_t len);
void      qvp_atlas_free(QvpAtlas*);
int32_t   qvp_atlas_page_of(const QvpAtlas*, uint16_t surah, uint16_t ayah);
int       qvp_atlas_page_range(const QvpAtlas*, uint16_t page, uint16_t out[4]);
uint32_t  qvp_atlas_page_count(const QvpAtlas*);
uint32_t  qvp_atlas_surah_count(const QvpAtlas*);
int       qvp_atlas_surah(const QvpAtlas*, uint16_t number, QvpAtlasSurah* out);
int       qvp_atlas_surah_at(const QvpAtlas*, uint32_t i, QvpAtlasSurah* out);
int       qvp_atlas_division(const QvpAtlas*, uint8_t division /* QVP_DIVISION_* */, uint16_t number, QvpAtlasRubuAlHizb* out);
int32_t   qvp_atlas_division_of(const QvpAtlas*, uint8_t division, uint16_t surah, uint16_t ayah);
int       qvp_atlas_pages_of_juz(const QvpAtlas*, uint16_t number, uint16_t out[2]);
uint32_t  qvp_atlas_search_surahs(const QvpAtlas*, const uint8_t* text, uint32_t len, uint16_t* out, uint32_t cap);
void      qvp_atlas_json(const QvpAtlas*, QvpStr* out);

/* names --------------------------------------------------------------------------------- */
/* The name tables the engine owns; ids run from 0, 255 is "unknown". A wrapper reads names from
   here and never carries a table of its own. */
enum { QVP_NAMES_MARK = 0, QVP_NAMES_KIND, QVP_NAMES_FAMILY, QVP_NAMES_CATEGORY, QVP_NAMES_DECORATION, QVP_NAMES_DIVISION, QVP_NAMES_PLACE };
uint32_t qvp_name_count(uint8_t table);
void     qvp_name(uint8_t table, uint8_t id, QvpStr* out);                 /* empty outside the table */
uint8_t  qvp_name_id(uint8_t table, const uint8_t* s, uint32_t len);       /* 255 = not in the table */
void     qvp_mark_name(uint8_t mark, QvpStr* out);
void     qvp_family_name(uint8_t f, QvpStr* out);
void     qvp_kind_name(uint8_t k, QvpStr* out);
void     qvp_category_name(uint8_t c, QvpStr* out);
uint8_t  qvp_mark_from_name(const uint8_t* s, uint32_t len);
uint8_t  qvp_mark_category(uint8_t mark);
void     qvp_version(QvpStr* out);                                         /* the engine version, e.g. "0.2.0" */
uint32_t qvp_format_version(void);                                         /* the page format version the engine reads */
const char* qvp_engine_name(void);

#ifdef __cplusplus
}
#endif
#endif
