// dart:ffi bindings for crates/qvp-ffi/include/qvp.h.
//
// Every struct mirrors the `#[repr(C)]` definition in crates/qvp-ffi/src/lib.rs
// field by field, in declared order and with the declared width, so dart:ffi
// lays it out exactly like the C compiler does (explicit `_pad` fields are
// declared where the header declares them). Nothing here decides anything:
// it is the raw contract, wrapped by engine.dart.
import 'dart:ffi' as ffi;

/// `typedef struct { const uint8_t* ptr; uint32_t len; } QvpStr;`
final class QvpStrC extends ffi.Struct {
  external ffi.Pointer<ffi.Uint8> ptr;
  @ffi.Uint32()
  external int len;
}

/// `{ float width, height; uint32_t page, n_lines, n_ayahs, n_words, n_paths, n_decorations; }`
final class QvpPageInfoC extends ffi.Struct {
  @ffi.Float()
  external double width;
  @ffi.Float()
  external double height;
  @ffi.Uint32()
  external int page;
  @ffi.Uint32()
  external int nLines;
  @ffi.Uint32()
  external int nAyahs;
  @ffi.Uint32()
  external int nWords;
  @ffi.Uint32()
  external int nPaths;
  @ffi.Uint32()
  external int nDecorations;
}

/// `{ const uint8_t* ops; uint32_t ops_len; const float* pts; uint32_t pts_len; const uint32_t* table; uint32_t n_paths; }`
final class QvpGeometryC extends ffi.Struct {
  external ffi.Pointer<ffi.Uint8> ops;
  @ffi.Uint32()
  external int opsLen;
  external ffi.Pointer<ffi.Float> pts;
  @ffi.Uint32()
  external int ptsLen;
  external ffi.Pointer<ffi.Uint32> table;
  @ffi.Uint32()
  external int nPaths;
}

/// `{ uint16_t surah, ayah, word, line_number; uint32_t ayah_index, line_index; float x0, y0, x1, y1; QvpStr text; uint32_t first_path, n_paths; }`
final class QvpWordInfoC extends ffi.Struct {
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint16()
  external int word;
  @ffi.Uint16()
  external int lineNumber;
  @ffi.Uint32()
  external int ayahIndex;
  @ffi.Uint32()
  external int lineIndex;
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
  external QvpStrC text;
  @ffi.Uint32()
  external int firstPath;
  @ffi.Uint32()
  external int nPaths;
}

/// `{ uint16_t surah, ayah; uint8_t fragment, fragments, flags, _pad; uint16_t rubu_al_hizb; uint32_t first_word, n_words, ayah_mark_decoration; float x0, y0, x1, y1; }`
final class QvpAyahInfoC extends ffi.Struct {
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint8()
  external int fragment;
  @ffi.Uint8()
  external int fragments;
  @ffi.Uint8()
  external int flags;
  @ffi.Uint8()
  external int pad;
  @ffi.Uint16()
  external int rubuAlHizb;
  @ffi.Uint32()
  external int firstWord;
  @ffi.Uint32()
  external int nWords;
  @ffi.Uint32()
  external int ayahMarkDecoration;
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
}

/// `{ uint8_t line_number, is_header; uint32_t first_word, n_words; float x0, y0, x1, y1, band_y0, band_y1, centre; }`
final class QvpLineInfoC extends ffi.Struct {
  @ffi.Uint8()
  external int lineNumber;
  @ffi.Uint8()
  external int isHeader;
  @ffi.Uint32()
  external int firstWord;
  @ffi.Uint32()
  external int nWords;
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
  @ffi.Float()
  external double bandY0;
  @ffi.Float()
  external double bandY1;
  @ffi.Float()
  external double centre;
}

/// `{ uint8_t decoration, _pad; uint16_t surah, ayah, _pad2; uint32_t line; float x0, y0, x1, y1; QvpStr text; uint32_t first_path, n_paths; }`
final class QvpDecorationInfoC extends ffi.Struct {
  @ffi.Uint8()
  external int decoration;
  @ffi.Uint8()
  external int pad;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint16()
  external int pad2;
  @ffi.Uint32()
  external int line;
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
  external QvpStrC text;
  @ffi.Uint32()
  external int firstPath;
  @ffi.Uint32()
  external int nPaths;
}

/// `{ uint32_t word, path, decoration, line; float distance; uint8_t is_exact; }`
final class QvpHitC extends ffi.Struct {
  @ffi.Uint32()
  external int word;
  @ffi.Uint32()
  external int path;
  @ffi.Uint32()
  external int decoration;
  @ffi.Uint32()
  external int line;
  @ffi.Float()
  external double distance;
  @ffi.Uint8()
  external int isExact;
}

/// `{ float max_distance, gap_bias; uint8_t prefer_exact; }`
final class QvpHitOptionsC extends ffi.Struct {
  @ffi.Float()
  external double maxDistance;
  @ffi.Float()
  external double gapBias;
  @ffi.Uint8()
  external int preferExact;
}

/// `{ uint32_t id, line; float x0, y0, x1, y1; uint32_t color; float radius; }`
final class QvpBoxC extends ffi.Struct {
  @ffi.Uint32()
  external int id;
  @ffi.Uint32()
  external int line;
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
  @ffi.Uint32()
  external int color;
  @ffi.Float()
  external double radius;
}

/// `{ uint32_t word, line; float x0, y0, x1, y1, ink_x0, ink_y0, ink_x1, ink_y1; }`
final class QvpHitAreaC extends ffi.Struct {
  @ffi.Uint32()
  external int word;
  @ffi.Uint32()
  external int line;
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
  @ffi.Float()
  external double inkX0;
  @ffi.Float()
  external double inkY0;
  @ffi.Float()
  external double inkX1;
  @ffi.Float()
  external double inkY1;
}

/// `{ uint32_t line, line_number; float y0, y1, mid, ink_y0, ink_y1; }`
final class QvpLineBandC extends ffi.Struct {
  @ffi.Uint32()
  external int line;
  @ffi.Uint32()
  external int lineNumber;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double y1;
  @ffi.Float()
  external double mid;
  @ffi.Float()
  external double inkY0;
  @ffi.Float()
  external double inkY1;
}

/// `{ float viewport_w, viewport_h, pad_top, pad_bottom, pad_left, pad_right, line_spacing; uint8_t fill_height; uint32_t grid_lines; float crop_left, crop_right, max_aspect_slack; }`
final class QvpLayoutSpecC extends ffi.Struct {
  @ffi.Float()
  external double viewportW;
  @ffi.Float()
  external double viewportH;
  @ffi.Float()
  external double padTop;
  @ffi.Float()
  external double padBottom;
  @ffi.Float()
  external double padLeft;
  @ffi.Float()
  external double padRight;
  @ffi.Float()
  external double lineSpacing;
  @ffi.Uint8()
  external int fillHeight;
  @ffi.Uint32()
  external int gridLines;
  @ffi.Float()
  external double cropLeft;
  @ffi.Float()
  external double cropRight;
  @ffi.Float()
  external double maxAspectSlack;
  @ffi.Float()
  external double reflowZoom;
  @ffi.Uint8()
  external int reflowFill;
  @ffi.Uint8()
  external int reflowBreaks;
  @ffi.Uint8()
  external int reflowGaps;
  @ffi.Float()
  external double reflowWordGap;
  @ffi.Float()
  external double reflowMaxStretch;
  @ffi.Float()
  external double reflowRelax;
}

/// `{ uint32_t lines; float line_spacing; }`
final class QvpGridC extends ffi.Struct {
  @ffi.Uint32()
  external int lines;
  @ffi.Float()
  external double lineSpacing;
}

/// `{ float scale, offset_x, offset_y, content_w, content_h, line_spacing; uint32_t n_lines; const float* lines; float fit_scale, fit_x, fit_y; }`
final class QvpLayoutC extends ffi.Struct {
  @ffi.Float()
  external double scale;
  @ffi.Float()
  external double offsetX;
  @ffi.Float()
  external double offsetY;
  @ffi.Float()
  external double contentW;
  @ffi.Float()
  external double contentH;
  @ffi.Float()
  external double lineSpacing;
  @ffi.Uint32()
  external int nLines;
  external ffi.Pointer<ffi.Float> lines;
  @ffi.Float()
  external double fitScale;
  @ffi.Float()
  external double fitX;
  @ffi.Float()
  external double fitY;
  @ffi.Uint32()
  external int reflowed;
  @ffi.Uint32()
  external int nRows;
}

/// `typedef struct { float scale, offset_x, offset_y; } QvpView;`
final class QvpViewC extends ffi.Struct {
  @ffi.Float()
  external double scale;
  @ffi.Float()
  external double offsetX;
  @ffi.Float()
  external double offsetY;
}

/// `typedef struct { uint32_t mode, step; float zoom; } QvpZoom;`
final class QvpZoomC extends ffi.Struct {
  @ffi.Uint32()
  external int mode;
  @ffi.Uint32()
  external int step;
  @ffi.Float()
  external double zoom;
}

/// `typedef struct { QvpZoom zoom; QvpView view; uint32_t relaid; } QvpZoomChange;`
final class QvpZoomChangeC extends ffi.Struct {
  external QvpZoomC zoom;
  external QvpViewC view;
  @ffi.Uint32()
  external int relaid;
}

/// `{ uint8_t target; uint32_t a, b, c; const uint32_t* words; uint32_t n_words; }`
final class QvpTargetC extends ffi.Struct {
  @ffi.Uint8()
  external int target;
  @ffi.Uint32()
  external int a;
  @ffi.Uint32()
  external int b;
  @ffi.Uint32()
  external int c;
  external ffi.Pointer<ffi.Uint32> words;
  @ffi.Uint32()
  external int nWords;
}

/// `{ uint8_t selector; uint32_t a, b, c; }`
final class QvpSelectorC extends ffi.Struct {
  @ffi.Uint8()
  external int selector;
  @ffi.Uint32()
  external int a;
  @ffi.Uint32()
  external int b;
  @ffi.Uint32()
  external int c;
}

/// `{ uint8_t mode, height; uint32_t ink, band; float pad_x, pad_y, radius, seam; uint32_t transition_ms; int32_t layer; }`
final class QvpHighlightStyleC extends ffi.Struct {
  @ffi.Uint8()
  external int mode;
  @ffi.Uint8()
  external int height;
  @ffi.Uint32()
  external int ink;
  @ffi.Uint32()
  external int band;
  @ffi.Float()
  external double padX;
  @ffi.Float()
  external double padY;
  @ffi.Float()
  external double radius;
  @ffi.Float()
  external double seam;
  @ffi.Uint32()
  external int transitionMs;
  @ffi.Int32()
  external int layer;
}

/// `{ uint32_t ink, diacritics, dots, waqf, sifr, ayahMark, numeral, headers, transition_ms; const uint32_t* marks; uint32_t n_marks; }`
final class QvpThemeC extends ffi.Struct {
  @ffi.Uint32()
  external int ink;
  @ffi.Uint32()
  external int diacritics;
  @ffi.Uint32()
  external int dots;
  @ffi.Uint32()
  external int waqf;
  @ffi.Uint32()
  external int sifr;
  @ffi.Uint32()
  external int ayahMark;
  @ffi.Uint32()
  external int numeral;
  @ffi.Uint32()
  external int headers;
  @ffi.Uint32()
  external int transitionMs;
  external ffi.Pointer<ffi.Uint32> marks;
  @ffi.Uint32()
  external int nMarks;
}

/// `{ uint16_t number, ayah_count; uint8_t has_banner, has_basmalah, place, _pad; uint32_t banner_decoration; QvpStr arabic, latin, english; }`
final class QvpSurahC extends ffi.Struct {
  @ffi.Uint16()
  external int number;
  @ffi.Uint16()
  external int ayahCount;
  @ffi.Uint8()
  external int hasBanner;
  @ffi.Uint8()
  external int hasBasmalah;
  @ffi.Uint8()
  external int place;
  @ffi.Uint8()
  external int pad;
  @ffi.Uint32()
  external int bannerDecoration;
  external QvpStrC arabic;
  external QvpStrC latin;
  external QvpStrC english;
}

/// `{ uint8_t division, line; uint16_t number, surah, ayah; uint32_t ayah_index; }`
final class QvpDivisionC extends ffi.Struct {
  @ffi.Uint8()
  external int division;
  @ffi.Uint8()
  external int line;
  @ffi.Uint16()
  external int number;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint32()
  external int ayahIndex;
}

/// `{ uint32_t decoration; uint16_t surah, ayah; uint32_t line; float cx, cy, r; uint32_t ornament_path, numeral_path; }`
final class QvpAyahMarkC extends ffi.Struct {
  @ffi.Uint32()
  external int decoration;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint32()
  external int line;
  @ffi.Float()
  external double cx;
  @ffi.Float()
  external double cy;
  @ffi.Float()
  external double r;
  @ffi.Uint32()
  external int ornamentPath;
  @ffi.Uint32()
  external int numeralPath;
}

/// `{ uint32_t decoration; uint16_t surah, ayah, juz, hizb, nisf, rubu_al_hizb, rubu_al_hizb_in_hizb, _pad; }`
final class QvpRosetteC extends ffi.Struct {
  @ffi.Uint32()
  external int decoration;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint16()
  external int juz;
  @ffi.Uint16()
  external int hizb;
  @ffi.Uint16()
  external int nisf;
  @ffi.Uint16()
  external int rubuAlHizb;
  @ffi.Uint16()
  external int rubuAlHizbInHizb;
  @ffi.Uint16()
  external int pad;
}

/// `{ uint32_t decoration; uint16_t surah, ayah; uint32_t sign_path; }`
final class QvpSajdahC extends ffi.Struct {
  @ffi.Uint32()
  external int decoration;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint32()
  external int signPath;
}

/// `{ uint32_t word, index; uint8_t is_loose_match; }`
final class QvpMatchC extends ffi.Struct {
  @ffi.Uint32()
  external int word;
  @ffi.Uint32()
  external int index;
  @ffi.Uint8()
  external int isLooseMatch;
}

/// `{ float x0, y0, x1, y1; uint32_t n_words, ayah_mark_decoration; }`
final class QvpCropBoundsC extends ffi.Struct {
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
  @ffi.Uint32()
  external int nWords;
  @ffi.Uint32()
  external int ayahMarkDecoration;
}

/// `{ uint16_t number, first_page, ayah_count; uint8_t place, _pad; QvpStr arabic, latin, english; }`
final class QvpAtlasSurahC extends ffi.Struct {
  @ffi.Uint16()
  external int number;
  @ffi.Uint16()
  external int firstPage;
  @ffi.Uint16()
  external int ayahCount;
  @ffi.Uint8()
  external int place;
  @ffi.Uint8()
  external int pad;
  external QvpStrC arabic;
  external QvpStrC latin;
  external QvpStrC english;
}

/// `{ uint16_t rubu_al_hizb, surah, ayah, page; }`
final class QvpAtlasRubuAlHizbC extends ffi.Struct {
  @ffi.Uint16()
  external int rubuAlHizb;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint16()
  external int page;
}

/// Opaque `QvpPage`.
final class QvpPageC extends ffi.Opaque {}

/// Opaque `QvpAtlas`.
final class QvpAtlasC extends ffi.Opaque {}

// ───────────── C function typedefs (native ↔ dart) ─────────────

typedef PtrPage = ffi.Pointer<QvpPageC>;
typedef PtrAtlas = ffi.Pointer<QvpAtlasC>;
typedef PtrU8 = ffi.Pointer<ffi.Uint8>;
typedef PtrU16 = ffi.Pointer<ffi.Uint16>;
typedef PtrU32 = ffi.Pointer<ffi.Uint32>;
typedef PtrF32 = ffi.Pointer<ffi.Float>;
typedef PtrStr = ffi.Pointer<QvpStrC>;
typedef PtrTarget = ffi.Pointer<QvpTargetC>;

/// Resolved C functions of one loaded `libqvp_ffi`. Field names are the C
/// names without the `qvp_` prefix, camel-cased.
final class QvpBindings {
  QvpBindings(this.lib);

  final ffi.DynamicLibrary lib;

  // memory
  late final PtrU8 Function(int) alloc = lib.lookupFunction<PtrU8 Function(ffi.Size), PtrU8 Function(int)>('qvp_alloc');
  late final void Function(PtrU8, int) dealloc = lib.lookupFunction<ffi.Void Function(PtrU8, ffi.Size), void Function(PtrU8, int)>('qvp_dealloc');

  // page
  late final PtrPage Function(PtrU8, int) pageLoad = lib.lookupFunction<PtrPage Function(PtrU8, ffi.Size), PtrPage Function(PtrU8, int)>('qvp_page_load');
  late final void Function(PtrPage) pageFree = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_page_free');
  late final void Function(PtrPage, ffi.Pointer<QvpPageInfoC>) pageInfo =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Pointer<QvpPageInfoC>), void Function(PtrPage, ffi.Pointer<QvpPageInfoC>)>('qvp_page_info');
  late final void Function(PtrPage, ffi.Pointer<QvpGeometryC>) geometry =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Pointer<QvpGeometryC>), void Function(PtrPage, ffi.Pointer<QvpGeometryC>)>('qvp_geometry');
  late final int Function(PtrPage, int, ffi.Pointer<QvpWordInfoC>) wordInfo =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpWordInfoC>), int Function(PtrPage, int, ffi.Pointer<QvpWordInfoC>)>('qvp_word_info');
  late final int Function(PtrPage, int, int, PtrStr) wordForm =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Uint8, PtrStr), int Function(PtrPage, int, int, PtrStr)>('qvp_word_form');
  late final int Function(PtrPage, int, ffi.Pointer<QvpAyahInfoC>) ayahInfo =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpAyahInfoC>), int Function(PtrPage, int, ffi.Pointer<QvpAyahInfoC>)>('qvp_ayah_info');
  late final int Function(PtrPage, int, ffi.Pointer<QvpLineInfoC>) lineInfo =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpLineInfoC>), int Function(PtrPage, int, ffi.Pointer<QvpLineInfoC>)>('qvp_line_info');
  late final int Function(PtrPage, int, ffi.Pointer<QvpDecorationInfoC>) decorationInfo =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpDecorationInfoC>), int Function(PtrPage, int, ffi.Pointer<QvpDecorationInfoC>)>('qvp_decoration_info');
  late final int Function(PtrPage, int, int, int) findWord =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint16, ffi.Uint16, ffi.Uint16), int Function(PtrPage, int, int, int)>('qvp_find_word');
  late final int Function(PtrPage, PtrTarget, PtrU32, int) targetWords =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrTarget, PtrU32, ffi.Uint32), int Function(PtrPage, PtrTarget, PtrU32, int)>('qvp_target_words');
  late final double Function(PtrPage) pageLineSpacing = lib.lookupFunction<ffi.Float Function(PtrPage), double Function(PtrPage)>('qvp_page_line_spacing');
  late final void Function(PtrPage, ffi.Pointer<QvpGridC>) pageGrid = lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Pointer<QvpGridC>), void Function(PtrPage, ffi.Pointer<QvpGridC>)>('qvp_page_grid');

  // metadata
  late final int Function(PtrPage) surahCount = lib.lookupFunction<ffi.Uint32 Function(PtrPage), int Function(PtrPage)>('qvp_surah_count');
  late final int Function(PtrPage, int, ffi.Pointer<QvpSurahC>) surahAt =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpSurahC>), int Function(PtrPage, int, ffi.Pointer<QvpSurahC>)>('qvp_surah_at');
  late final int Function(PtrPage, ffi.Pointer<QvpDivisionC>, int) divisions =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpDivisionC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpDivisionC>, int)>('qvp_divisions');
  late final int Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, int) ayahMarks =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, int)>('qvp_ayah_marks');
  late final int Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, int) ayahMarksView =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, int)>('qvp_ayah_marks_view');
  late final int Function(PtrPage, ffi.Pointer<QvpRosetteC>, int) rosettes =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpRosetteC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpRosetteC>, int)>('qvp_rosettes');
  late final int Function(PtrPage, ffi.Pointer<QvpSajdahC>, int) sajdahs =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpSajdahC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpSajdahC>, int)>('qvp_sajdahs');
  late final int Function(PtrPage, PtrU32, int) ayahKeys =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_ayah_keys');
  late final int Function(PtrPage, int, int, PtrU8) ayahWordCount =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint16, ffi.Uint16, PtrU8), int Function(PtrPage, int, int, PtrU8)>('qvp_ayah_word_count');
  late final int Function(PtrPage, int, int, int, PtrU32, int) reciteMap =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint16, ffi.Uint16, ffi.Uint32, PtrU32, ffi.Uint32), int Function(PtrPage, int, int, int, PtrU32, int)>('qvp_recite_map');
  late final void Function(PtrPage, int, PtrStr) wordLabel =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, PtrStr), void Function(PtrPage, int, PtrStr)>('qvp_word_label');
  late final void Function(PtrPage, int, PtrStr) ayahLabel =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, PtrStr), void Function(PtrPage, int, PtrStr)>('qvp_ayah_label');

  // text & search
  late final void Function(PtrPage, PtrTarget, int, PtrU8, int, PtrU8, int, PtrStr) text = lib.lookupFunction<
      ffi.Void Function(PtrPage, PtrTarget, ffi.Uint8, PtrU8, ffi.Uint32, PtrU8, ffi.Uint32, PtrStr),
      void Function(PtrPage, PtrTarget, int, PtrU8, int, PtrU8, int, PtrStr)>('qvp_text');
  late final int Function(PtrPage, PtrU8, int, int, int, int, int, int, ffi.Pointer<QvpMatchC>, int) search = lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU8, ffi.Uint32, ffi.Uint8, ffi.Uint8, ffi.Uint8, ffi.Uint8, ffi.Uint32, ffi.Pointer<QvpMatchC>, ffi.Uint32),
      int Function(PtrPage, PtrU8, int, int, int, int, int, int, ffi.Pointer<QvpMatchC>, int)>('qvp_search');
  late final void Function(int, PtrU8, int, PtrStr) arabic =
      lib.lookupFunction<ffi.Void Function(ffi.Uint8, PtrU8, ffi.Uint32, PtrStr), void Function(int, PtrU8, int, PtrStr)>('qvp_arabic');
  late final void Function(PtrPage, PtrU32, int, PtrStr) citation =
      lib.lookupFunction<ffi.Void Function(PtrPage, PtrU32, ffi.Uint32, PtrStr), void Function(PtrPage, PtrU32, int, PtrStr)>('qvp_citation');
  late final int Function(PtrPage, PtrU8, int) attachWords =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, PtrU8, ffi.Uint32), int Function(PtrPage, PtrU8, int)>('qvp_attach_words');
  late final int Function(PtrPage, int) hasForm = lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Uint8), int Function(PtrPage, int)>('qvp_has_form');

  // hit testing
  late final int Function(PtrPage, double, double, ffi.Pointer<QvpHitC>) hitTestExact = lib.lookupFunction<
      ffi.Int32 Function(PtrPage, ffi.Float, ffi.Float, ffi.Pointer<QvpHitC>), int Function(PtrPage, double, double, ffi.Pointer<QvpHitC>)>('qvp_hit_test_exact');
  late final int Function(PtrPage, double, double, ffi.Pointer<QvpHitC>) hitTestExactView = lib.lookupFunction<
      ffi.Int32 Function(PtrPage, ffi.Float, ffi.Float, ffi.Pointer<QvpHitC>), int Function(PtrPage, double, double, ffi.Pointer<QvpHitC>)>('qvp_hit_test_exact_view');
  late final int Function(PtrPage, double, double, ffi.Pointer<QvpHitOptionsC>, ffi.Pointer<QvpHitC>) hitTest = lib.lookupFunction<
      ffi.Int32 Function(PtrPage, ffi.Float, ffi.Float, ffi.Pointer<QvpHitOptionsC>, ffi.Pointer<QvpHitC>),
      int Function(PtrPage, double, double, ffi.Pointer<QvpHitOptionsC>, ffi.Pointer<QvpHitC>)>('qvp_hit_test');
  late final int Function(PtrPage, double, double, ffi.Pointer<QvpHitOptionsC>, ffi.Pointer<QvpHitC>) hitTestView = lib.lookupFunction<
      ffi.Int32 Function(PtrPage, ffi.Float, ffi.Float, ffi.Pointer<QvpHitOptionsC>, ffi.Pointer<QvpHitC>),
      int Function(PtrPage, double, double, ffi.Pointer<QvpHitOptionsC>, ffi.Pointer<QvpHitC>)>('qvp_hit_test_view');
  late final int Function(PtrPage, ffi.Pointer<QvpLineBandC>, int) lineBands =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpLineBandC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpLineBandC>, int)>('qvp_line_bands');
  late final int Function(PtrPage, double, ffi.Pointer<QvpHitAreaC>, int) hitAreas = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Float, ffi.Pointer<QvpHitAreaC>, ffi.Uint32), int Function(PtrPage, double, ffi.Pointer<QvpHitAreaC>, int)>('qvp_hit_areas');
  late final int Function(PtrPage, double, ffi.Pointer<QvpHitAreaC>, int) hitAreasView = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Float, ffi.Pointer<QvpHitAreaC>, ffi.Uint32), int Function(PtrPage, double, ffi.Pointer<QvpHitAreaC>, int)>('qvp_hit_areas_view');

  // layout
  late final void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpLayoutC>) layout = lib.lookupFunction<
      ffi.Void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpLayoutC>),
      void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpLayoutC>)>('qvp_layout');
  late final double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, double) layoutLineSpacingToFill = lib.lookupFunction<
      ffi.Float Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Float),
      double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, double)>('qvp_layout_line_spacing_to_fill');
  late final double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>) layoutWastedFraction = lib.lookupFunction<ffi.Float Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>), double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>)>('qvp_layout_wasted_fraction');
  late final int Function(PtrPage, int, PtrF32) wordBoundsView =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, PtrF32), int Function(PtrPage, int, PtrF32)>('qvp_word_bounds_view');

  // styles
  late final int Function(PtrPage, int, ffi.Pointer<QvpSelectorC>, int, int) styleAdd = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Int32, ffi.Pointer<QvpSelectorC>, ffi.Uint32, ffi.Uint32),
      int Function(PtrPage, int, ffi.Pointer<QvpSelectorC>, int, int)>('qvp_style_add');
  late final int Function(PtrPage, int, PtrTarget, int, int) styleAddTarget = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Int32, PtrTarget, ffi.Uint32, ffi.Uint32), int Function(PtrPage, int, PtrTarget, int, int)>('qvp_style_add_target');
  late final int Function(PtrPage, int) styleRemove = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_style_remove');
  late final int Function(PtrPage, int, int, int) styleRecolor =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32, ffi.Uint32, ffi.Uint32), int Function(PtrPage, int, int, int)>('qvp_style_recolor');
  late final void Function(PtrPage) styleClear = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_style_clear');
  late final void Function(PtrPage, int) styleClearLayer = lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Int32), void Function(PtrPage, int)>('qvp_style_clear_layer');
  late final void Function(PtrPage, int) styleDefaultColor = lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32), void Function(PtrPage, int)>('qvp_style_default_color');
  late final int Function(PtrPage, ffi.Pointer<QvpSelectorC>) hide =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpSelectorC>), int Function(PtrPage, ffi.Pointer<QvpSelectorC>)>('qvp_style_hide');
  late final int Function(PtrPage, ffi.Pointer<QvpThemeC>) theme =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpThemeC>), int Function(PtrPage, ffi.Pointer<QvpThemeC>)>('qvp_theme');
  late final int Function(PtrPage, PtrU32, int) styleHandles =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_style_handles');

  // clock & display list
  late final int Function(PtrPage, double) tick = lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Double), int Function(PtrPage, double)>('qvp_tick');
  late final PtrU32 Function(PtrPage) colors = lib.lookupFunction<PtrU32 Function(PtrPage), PtrU32 Function(PtrPage)>('qvp_colors');
  late final int Function(PtrPage, PtrU32, int) styledPaths = lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_styled_paths');
  late final int Function(PtrPage, int) colorOf = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_color_of');

  // highlights
  late final int Function(PtrPage, PtrTarget, ffi.Pointer<QvpHighlightStyleC>) highlight = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrTarget, ffi.Pointer<QvpHighlightStyleC>), int Function(PtrPage, PtrTarget, ffi.Pointer<QvpHighlightStyleC>)>('qvp_highlight_add');
  late final int Function(PtrPage, int, PtrTarget) moveHighlight =
      lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Uint32, PtrTarget), int Function(PtrPage, int, PtrTarget)>('qvp_highlight_move');
  late final int Function(PtrPage, int, ffi.Pointer<QvpHighlightStyleC>) restyleHighlight = lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpHighlightStyleC>), int Function(PtrPage, int, ffi.Pointer<QvpHighlightStyleC>)>('qvp_highlight_restyle');
  late final int Function(PtrPage, int) removeHighlight = lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_highlight_remove');
  late final void Function(PtrPage) clearHighlights = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_highlight_clear');
  late final int Function(PtrPage, PtrU32, int) highlightHandles =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_highlight_handles');
  late final int Function(PtrPage, int, PtrU32, int) highlightWords =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32, PtrU32, ffi.Uint32), int Function(PtrPage, int, PtrU32, int)>('qvp_highlight_words');
  late final int Function(PtrPage, ffi.Pointer<QvpBoxC>, int) highlightBoxesView =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpBoxC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpBoxC>, int)>('qvp_highlight_boxes_view');
  late final int Function(PtrPage, PtrU32, int, int, double, double, ffi.Pointer<QvpBoxC>, int) wordBands = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32, ffi.Uint8, ffi.Float, ffi.Float, ffi.Pointer<QvpBoxC>, ffi.Uint32),
      int Function(PtrPage, PtrU32, int, int, double, double, ffi.Pointer<QvpBoxC>, int)>('qvp_word_bands');
  late final int Function(PtrPage, PtrU32, int, int, double, double, ffi.Pointer<QvpBoxC>, int) wordBandsView = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32, ffi.Uint8, ffi.Float, ffi.Float, ffi.Pointer<QvpBoxC>, ffi.Uint32),
      int Function(PtrPage, PtrU32, int, int, double, double, ffi.Pointer<QvpBoxC>, int)>('qvp_word_bands_view');

  // selection
  late final void Function(PtrPage, int, int) select =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, ffi.Uint32), void Function(PtrPage, int, int)>('qvp_select');
  late final int Function(PtrPage, PtrU32, int) selection =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_selection');
  late final void Function(PtrPage, int, int, PtrStr) selectionText =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint8, ffi.Uint8, PtrStr), void Function(PtrPage, int, int, PtrStr)>('qvp_selection_text');

  // memorisation
  late final void Function(PtrPage, PtrTarget, int) mask = lib.lookupFunction<ffi.Void Function(PtrPage, PtrTarget, ffi.Uint8), void Function(PtrPage, PtrTarget, int)>('qvp_mask');
  late final void Function(PtrPage, int, int) maskFrom =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, ffi.Uint8), void Function(PtrPage, int, int)>('qvp_mask_from');
  late final void Function(PtrPage, int, double, double, double, int) maskOptions = lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, ffi.Float, ffi.Float, ffi.Float, ffi.Uint8), void Function(PtrPage, int, double, double, double, int)>('qvp_mask_options');
  late final int Function(PtrPage, int) unmaskNext = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_unmask_next');
  late final int Function(PtrPage, int) maskBack = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_mask_back');
  late final int Function(PtrPage, int) unmaskWord = lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_unmask_word');
  late final int Function(PtrPage, int) maskWord = lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_mask_word');
  late final void Function(PtrPage) unmaskAll = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_unmask_all');
  late final void Function(PtrPage) maskAll = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_mask_all');
  late final void Function(PtrPage) unmask = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_unmask');
  late final int Function(PtrPage, PtrU32, int) maskHidden =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_mask_hidden');
  late final int Function(PtrPage, PtrU32, int) maskWords =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_mask_words');
  late final int Function(PtrPage, ffi.Pointer<QvpBoxC>, int) maskBoxesView =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpBoxC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpBoxC>, int)>('qvp_mask_boxes_view');
  late final int Function(PtrPage, int, int, int, int, int, int) revealStart = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32, ffi.Uint8, ffi.Uint32, ffi.Uint32, ffi.Uint8, ffi.Uint32),
      int Function(PtrPage, int, int, int, int, int, int)>('qvp_reveal_start');
  late final int Function(PtrPage, int) revealGoto = lib.lookupFunction<ffi.Uint8 Function(PtrPage, ffi.Int64), int Function(PtrPage, int)>('qvp_reveal_goto');
  late final int Function(PtrPage) revealPosition = lib.lookupFunction<ffi.Int64 Function(PtrPage), int Function(PtrPage)>('qvp_reveal_position');
  late final int Function(PtrPage) revealStepCount = lib.lookupFunction<ffi.Uint32 Function(PtrPage), int Function(PtrPage)>('qvp_reveal_step_count');
  late final int Function(PtrPage, int) revealStepOf = lib.lookupFunction<ffi.Int64 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_reveal_step_of');
  late final void Function(PtrPage) revealStop = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_reveal_stop');

  // crop
  late final int Function(PtrPage, PtrTarget, double, int, ffi.Pointer<QvpCropBoundsC>) cropBounds = lib.lookupFunction<ffi.Int32 Function(PtrPage, PtrTarget, ffi.Float, ffi.Uint8, ffi.Pointer<QvpCropBoundsC>), int Function(PtrPage, PtrTarget, double, int, ffi.Pointer<QvpCropBoundsC>)>('qvp_crop_bounds');
  late final int Function(PtrPage, PtrTarget, double, int, int, PtrStr) cropSvg = lib.lookupFunction<ffi.Int32 Function(PtrPage, PtrTarget, ffi.Float, ffi.Uint8, ffi.Uint32, PtrStr), int Function(PtrPage, PtrTarget, double, int, int, PtrStr)>('qvp_crop_svg');

  // atlas
  late final PtrAtlas Function(PtrU8, int) atlasLoad = lib.lookupFunction<PtrAtlas Function(PtrU8, ffi.Size), PtrAtlas Function(PtrU8, int)>('qvp_atlas_load');
  late final void Function(PtrAtlas) atlasFree = lib.lookupFunction<ffi.Void Function(PtrAtlas), void Function(PtrAtlas)>('qvp_atlas_free');
  late final int Function(PtrAtlas, int, int) atlasPageOf =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint16, ffi.Uint16), int Function(PtrAtlas, int, int)>('qvp_atlas_page_of');
  late final int Function(PtrAtlas, int, PtrU16) atlasPageRange =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint16, PtrU16), int Function(PtrAtlas, int, PtrU16)>('qvp_atlas_page_range');
  late final int Function(PtrAtlas) atlasPageCount = lib.lookupFunction<ffi.Uint32 Function(PtrAtlas), int Function(PtrAtlas)>('qvp_atlas_page_count');
  late final int Function(PtrAtlas) atlasSurahs = lib.lookupFunction<ffi.Uint32 Function(PtrAtlas), int Function(PtrAtlas)>('qvp_atlas_surah_count');
  late final int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>) atlasSurah = lib.lookupFunction<
      ffi.Int32 Function(PtrAtlas, ffi.Uint16, ffi.Pointer<QvpAtlasSurahC>), int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>)>('qvp_atlas_surah');
  late final int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>) atlasSurahAt = lib.lookupFunction<
      ffi.Int32 Function(PtrAtlas, ffi.Uint32, ffi.Pointer<QvpAtlasSurahC>), int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>)>('qvp_atlas_surah_at');
  late final int Function(PtrAtlas, int, int, ffi.Pointer<QvpAtlasRubuAlHizbC>) atlasDivision = lib.lookupFunction<
      ffi.Int32 Function(PtrAtlas, ffi.Uint8, ffi.Uint16, ffi.Pointer<QvpAtlasRubuAlHizbC>), int Function(PtrAtlas, int, int, ffi.Pointer<QvpAtlasRubuAlHizbC>)>('qvp_atlas_division');
  late final int Function(PtrAtlas, int, int, int) atlasDivisionOf =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint8, ffi.Uint16, ffi.Uint16), int Function(PtrAtlas, int, int, int)>('qvp_atlas_division_of');
  late final int Function(PtrAtlas, int, PtrU16) atlasPagesOfJuz =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint16, PtrU16), int Function(PtrAtlas, int, PtrU16)>('qvp_atlas_pages_of_juz');
  late final int Function(PtrAtlas, PtrU8, int, PtrU16, int) atlasSearchSurahs = lib.lookupFunction<
      ffi.Uint32 Function(PtrAtlas, PtrU8, ffi.Uint32, PtrU16, ffi.Uint32), int Function(PtrAtlas, PtrU8, int, PtrU16, int)>('qvp_atlas_search_surahs');
  late final void Function(PtrAtlas, PtrStr) atlasJson = lib.lookupFunction<ffi.Void Function(PtrAtlas, PtrStr), void Function(PtrAtlas, PtrStr)>('qvp_atlas_json');

  // names
  late final void Function(int, PtrStr) markName = lib.lookupFunction<ffi.Void Function(ffi.Uint8, PtrStr), void Function(int, PtrStr)>('qvp_mark_name');
  late final void Function(int, PtrStr) familyName = lib.lookupFunction<ffi.Void Function(ffi.Uint8, PtrStr), void Function(int, PtrStr)>('qvp_family_name');
  late final void Function(int, PtrStr) kindName = lib.lookupFunction<ffi.Void Function(ffi.Uint8, PtrStr), void Function(int, PtrStr)>('qvp_kind_name');
  late final void Function(int, PtrStr) categoryName = lib.lookupFunction<ffi.Void Function(ffi.Uint8, PtrStr), void Function(int, PtrStr)>('qvp_category_name');
  late final int Function(PtrU8, int) markFromName = lib.lookupFunction<ffi.Uint8 Function(PtrU8, ffi.Uint32), int Function(PtrU8, int)>('qvp_mark_from_name');
  late final int Function(int) markCategory = lib.lookupFunction<ffi.Uint8 Function(ffi.Uint8), int Function(int)>('qvp_mark_category');
  late final int Function(int) nameCount = lib.lookupFunction<ffi.Uint32 Function(ffi.Uint8), int Function(int)>('qvp_name_count');
  late final void Function(int, int, PtrStr) name = lib.lookupFunction<ffi.Void Function(ffi.Uint8, ffi.Uint8, PtrStr), void Function(int, int, PtrStr)>('qvp_name');
  late final int Function(int, PtrU8, int) nameId = lib.lookupFunction<ffi.Uint8 Function(ffi.Uint8, PtrU8, ffi.Uint32), int Function(int, PtrU8, int)>('qvp_name_id');
  late final void Function(PtrStr) version = lib.lookupFunction<ffi.Void Function(PtrStr), void Function(PtrStr)>('qvp_version');
  late final int Function() formatVersion = lib.lookupFunction<ffi.Uint32 Function(), int Function()>('qvp_format_version');
  late final ffi.Pointer<ffi.Char> Function() engineName =
      lib.lookupFunction<ffi.Pointer<ffi.Char> Function(), ffi.Pointer<ffi.Char> Function()>('qvp_engine_name');

  // ── the reader's pan and zoom, and the control that owns them ──
  late final void Function(ffi.Pointer<QvpViewC>, double, double, double, double, double, ffi.Pointer<QvpViewC>) viewZoomAbout =
      lib.lookupFunction<
          ffi.Void Function(ffi.Pointer<QvpViewC>, ffi.Float, ffi.Float, ffi.Float, ffi.Float, ffi.Float, ffi.Pointer<QvpViewC>),
          void Function(ffi.Pointer<QvpViewC>, double, double, double, double, double, ffi.Pointer<QvpViewC>)>('qvp_view_zoom_about');
  late final void Function(ffi.Pointer<QvpViewC>, double, double, ffi.Pointer<QvpViewC>) viewPan = lib.lookupFunction<
      ffi.Void Function(ffi.Pointer<QvpViewC>, ffi.Float, ffi.Float, ffi.Pointer<QvpViewC>),
      void Function(ffi.Pointer<QvpViewC>, double, double, ffi.Pointer<QvpViewC>)>('qvp_view_pan');
  late final void Function(ffi.Pointer<QvpViewC>, double, double, double, double, ffi.Pointer<QvpViewC>) viewClamp =
      lib.lookupFunction<
          ffi.Void Function(ffi.Pointer<QvpViewC>, ffi.Float, ffi.Float, ffi.Float, ffi.Float, ffi.Pointer<QvpViewC>),
          void Function(ffi.Pointer<QvpViewC>, double, double, double, double, ffi.Pointer<QvpViewC>)>('qvp_view_clamp');
  late final void Function(PtrPage, ffi.Pointer<QvpViewC>, int, double, double, double, double, double, double, ffi.Pointer<QvpViewC>)
      viewAnchor = lib.lookupFunction<
          ffi.Void Function(PtrPage, ffi.Pointer<QvpViewC>, ffi.Uint32, ffi.Float, ffi.Float, ffi.Float, ffi.Float, ffi.Float, ffi.Float,
              ffi.Pointer<QvpViewC>),
          void Function(PtrPage, ffi.Pointer<QvpViewC>, int, double, double, double, double, double, double,
              ffi.Pointer<QvpViewC>)>('qvp_view_anchor');
  late final void Function(PtrPage, ffi.Pointer<QvpViewC>, double, double, PtrF32) viewToLayout = lib.lookupFunction<
      ffi.Void Function(PtrPage, ffi.Pointer<QvpViewC>, ffi.Float, ffi.Float, PtrF32),
      void Function(PtrPage, ffi.Pointer<QvpViewC>, double, double, PtrF32)>('qvp_view_to_layout');
  late final int Function(double, double, double, double) viewSwipe = lib.lookupFunction<
      ffi.Int32 Function(ffi.Float, ffi.Float, ffi.Float, ffi.Float),
      int Function(double, double, double, double)>('qvp_view_swipe');
  late final int Function(double, double, double, double) swipePages = lib.lookupFunction<
      ffi.Int32 Function(ffi.Float, ffi.Float, ffi.Float, ffi.Float),
      int Function(double, double, double, double)>('qvp_swipe_pages');

  late final void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, int, ffi.Pointer<QvpZoomC>) zoomMode =
      lib.lookupFunction<
          ffi.Void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Uint32, ffi.Pointer<QvpZoomC>),
          void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, int, ffi.Pointer<QvpZoomC>)>('qvp_zoom_mode');
  late final void Function(
      PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, double, double, double,
      ffi.Pointer<QvpZoomChangeC>) zoomPinch = lib.lookupFunction<
      ffi.Void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, ffi.Float, ffi.Float,
          ffi.Float, ffi.Pointer<QvpZoomChangeC>),
      void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, double, double, double,
          ffi.Pointer<QvpZoomChangeC>)>('qvp_zoom_pinch');
  late final void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, int, ffi.Pointer<QvpViewC>,
      ffi.Pointer<QvpZoomChangeC>) zoomToStep = lib.lookupFunction<
      ffi.Void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Uint32, ffi.Pointer<QvpViewC>,
          ffi.Pointer<QvpZoomChangeC>),
      void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, int, ffi.Pointer<QvpViewC>,
          ffi.Pointer<QvpZoomChangeC>)>('qvp_zoom_to_step');
  late final void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpLayoutSpecC>) zoomSpec =
      lib.lookupFunction<
          ffi.Void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpLayoutSpecC>),
          void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpLayoutSpecC>)>('qvp_zoom_spec');
  late final void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpZoomC>) zoomCarried =
      lib.lookupFunction<
          ffi.Void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpZoomC>),
          void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpZoomC>)>('qvp_zoom_carried');
  late final double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, int) zoomAtStep = lib.lookupFunction<
      ffi.Float Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Uint32),
      double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, int)>('qvp_zoom_at_step');
  late final int Function(ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, double) zoomIsZoomed = lib.lookupFunction<
      ffi.Int Function(ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, ffi.Float),
      int Function(ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, double)>('qvp_zoom_is_zoomed');
  late final int Function(PtrPage, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, double) sidewaysDrag = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, ffi.Float),
      int Function(PtrPage, ffi.Pointer<QvpZoomC>, ffi.Pointer<QvpViewC>, double)>('qvp_sideways_drag');
  late final int Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, PtrF32, int) zoomSteps = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, PtrF32, ffi.Uint32),
      int Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, PtrF32, int)>('qvp_zoom_steps');
  late final int Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, PtrF32, int, double, PtrF32, int) zoomLevels =
      lib.lookupFunction<
          ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, PtrF32, ffi.Uint32, ffi.Float, PtrF32, ffi.Uint32),
          int Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, PtrF32, int, double, PtrF32, int)>('qvp_zoom_levels');
  late final int Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, double, double, double, PtrF32, PtrF32, int) zoomLevelCandidates =
      lib.lookupFunction<
          ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Float, ffi.Float, ffi.Float, PtrF32, PtrF32, ffi.Uint32),
          int Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, double, double, double, PtrF32, PtrF32,
              int)>('qvp_zoom_level_candidates');
  late final double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>) reflowMaxZoom = lib.lookupFunction<
      ffi.Float Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>),
      double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>)>('qvp_reflow_max_zoom');

  // ── what a layout draws ──
  late final int Function(PtrPage, ffi.Pointer<QvpLayoutC>) layoutCurrent = lib.lookupFunction<
      ffi.Int Function(PtrPage, ffi.Pointer<QvpLayoutC>),
      int Function(PtrPage, ffi.Pointer<QvpLayoutC>)>('qvp_layout_current');
  late final int Function(PtrPage, double, double, PtrU32, int) layoutDrawList = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Float, ffi.Float, PtrU32, ffi.Uint32),
      int Function(PtrPage, double, double, PtrU32, int)>('qvp_layout_draw_list');
  late final int Function(PtrPage, PtrF32, int) layoutPlacements = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrF32, ffi.Uint32), int Function(PtrPage, PtrF32, int)>('qvp_layout_placements');
  late final int Function(PtrPage, PtrF32, int) layoutGroups = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrF32, ffi.Uint32), int Function(PtrPage, PtrF32, int)>('qvp_layout_groups');
  late final int Function(PtrPage, PtrF32, int) layoutRepeats = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrF32, ffi.Uint32), int Function(PtrPage, PtrF32, int)>('qvp_layout_repeats');
  late final int Function(PtrPage, PtrU32, int) layoutPathGroups = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_layout_path_groups');
  late final int Function(PtrPage, PtrU32, int) layoutOmittedPaths = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_layout_omitted_paths');
  late final int Function(PtrPage, int, PtrU32, int) layoutRowWords = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Uint32, PtrU32, ffi.Uint32), int Function(PtrPage, int, PtrU32, int)>('qvp_layout_row_words');
  late final int Function(PtrPage, int) layoutWordRow = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_layout_word_row');
}
