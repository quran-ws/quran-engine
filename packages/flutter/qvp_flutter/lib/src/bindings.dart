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

/// `{ float width, height; uint32_t page, n_lines, n_ayahs, n_words, n_paths, n_decos; }`
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
  external int nDecos;
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

/// `{ uint16_t surah, ayah, word, line_no; uint32_t ayah_idx, line_idx; float x0, y0, x1, y1; QvpStr text; uint32_t first_path, n_paths; }`
final class QvpWordInfoC extends ffi.Struct {
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint16()
  external int word;
  @ffi.Uint16()
  external int lineNo;
  @ffi.Uint32()
  external int ayahIdx;
  @ffi.Uint32()
  external int lineIdx;
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

/// `{ uint16_t surah, ayah; uint8_t fragment, fragments, flags, _pad; uint16_t rubuAlHizb; uint32_t first_word, n_words, ayah_mark_deco; float x0, y0, x1, y1; }`
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
  external int ayahMarkDeco;
  @ffi.Float()
  external double x0;
  @ffi.Float()
  external double y0;
  @ffi.Float()
  external double x1;
  @ffi.Float()
  external double y1;
}

/// `{ uint8_t line_no, is_header; uint32_t first_word, n_words; float x0, y0, x1, y1, band_y0, band_y1, centre; }`
final class QvpLineInfoC extends ffi.Struct {
  @ffi.Uint8()
  external int lineNo;
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

/// `{ uint8_t kind, _pad; uint16_t surah, ayah, _pad2; uint32_t line; float x0, y0, x1, y1; QvpStr text; uint32_t first_path, n_paths; }`
final class QvpDecoInfoC extends ffi.Struct {
  @ffi.Uint8()
  external int kind;
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

/// `{ uint32_t word, path, deco, line; float distance; uint32_t is_exact; }`
final class QvpHitC extends ffi.Struct {
  @ffi.Uint32()
  external int word;
  @ffi.Uint32()
  external int path;
  @ffi.Uint32()
  external int deco;
  @ffi.Uint32()
  external int line;
  @ffi.Float()
  external double distance;
  @ffi.Uint32()
  external int isExact;
}

/// `{ float max_distance, gap_bias; uint32_t prefer_exact; }`
final class QvpHitOptionsC extends ffi.Struct {
  @ffi.Float()
  external double maxDistance;
  @ffi.Float()
  external double gapBias;
  @ffi.Uint32()
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

/// `{ uint32_t line, line_no; float y0, y1, mid, ink_y0, ink_y1; }`
final class QvpLineBandC extends ffi.Struct {
  @ffi.Uint32()
  external int line;
  @ffi.Uint32()
  external int lineNo;
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

/// `{ float viewport_w, viewport_h, pad_top, pad_bottom, pad_left, pad_right, line_spacing, line_gap; uint32_t fill_height, nominal_lines; }`
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
  @ffi.Float()
  external double lineGap;
  @ffi.Uint32()
  external int fillHeight;
  @ffi.Uint32()
  external int nominalLines;
  @ffi.Float()
  external double cropLeft;
  @ffi.Float()
  external double cropRight;
  @ffi.Float()
  external double maxAspectSlack;
}

/// `{ float scale, ox, oy, content_w, content_h, pitch; uint32_t n_lines; const float* lines; }`
final class QvpLayoutC extends ffi.Struct {
  @ffi.Float()
  external double scale;
  @ffi.Float()
  external double ox;
  @ffi.Float()
  external double oy;
  @ffi.Float()
  external double contentW;
  @ffi.Float()
  external double contentH;
  @ffi.Float()
  external double pitch;
  @ffi.Uint32()
  external int nLines;
  external ffi.Pointer<ffi.Float> lines;
  @ffi.Float()
  external double fitScale;
  @ffi.Float()
  external double fitX;
  @ffi.Float()
  external double fitY;
}

/// `{ uint8_t kind; uint32_t a, b, c; const uint32_t* words; uint32_t n_words; }`
final class QvpTargetC extends ffi.Struct {
  @ffi.Uint8()
  external int kind;
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

/// `{ uint8_t kind; uint32_t a, b, c; }`
final class QvpSelectorC extends ffi.Struct {
  @ffi.Uint8()
  external int kind;
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

/// `{ uint16_t number, ayah_count; uint8_t has_banner, has_basmalah, place, _pad; uint32_t banner_deco; QvpStr arabic, latin, english; }`
final class QvpSurahInfoC extends ffi.Struct {
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
  external int bannerDeco;
  external QvpStrC arabic;
  external QvpStrC latin;
  external QvpStrC english;
}

/// `{ uint8_t kind, line; uint16_t n, surah, ayah; uint32_t ayah_idx; }`
final class QvpDivisionC extends ffi.Struct {
  @ffi.Uint8()
  external int kind;
  @ffi.Uint8()
  external int line;
  @ffi.Uint16()
  external int n;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint32()
  external int ayahIdx;
}

/// `{ uint32_t deco; uint16_t surah, ayah; uint32_t line; float cx, cy, r; uint32_t ornament_path, numeral_path; }`
final class QvpAyahMarkC extends ffi.Struct {
  @ffi.Uint32()
  external int deco;
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

/// `{ uint32_t deco; uint16_t surah, ayah, juz, hizb, nisf, rubuAlHizb, rubu_al_hizb_in_hizb, _pad; }`
final class QvpRosetteC extends ffi.Struct {
  @ffi.Uint32()
  external int deco;
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

/// `{ uint32_t deco; uint16_t surah, ayah; uint32_t sign_path; }`
final class QvpSajdahC extends ffi.Struct {
  @ffi.Uint32()
  external int deco;
  @ffi.Uint16()
  external int surah;
  @ffi.Uint16()
  external int ayah;
  @ffi.Uint32()
  external int signPath;
}

/// `{ uint32_t word, index, loose; }`
final class QvpMatchC extends ffi.Struct {
  @ffi.Uint32()
  external int word;
  @ffi.Uint32()
  external int index;
  @ffi.Uint32()
  external int loose;
}

/// `{ float x0, y0, x1, y1; uint32_t n_words, ayah_mark_deco; }`
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
  external int ayahMarkDeco;
}

/// `{ uint16_t n, first_page, ayah_count; uint8_t place, _pad; QvpStr arabic, latin, english; }`
final class QvpAtlasSurahC extends ffi.Struct {
  @ffi.Uint16()
  external int n;
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

/// `{ uint16_t rubuAlHizb, surah, ayah, page; }`
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
  late final int Function(PtrPage, int, ffi.Pointer<QvpDecoInfoC>) decoInfo =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpDecoInfoC>), int Function(PtrPage, int, ffi.Pointer<QvpDecoInfoC>)>('qvp_deco_info');
  late final int Function(PtrPage, int, int, int) findWord =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint16, ffi.Uint16, ffi.Uint16), int Function(PtrPage, int, int, int)>('qvp_find_word');
  late final int Function(PtrPage, PtrTarget, PtrU32, int) resolve =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrTarget, PtrU32, ffi.Uint32), int Function(PtrPage, PtrTarget, PtrU32, int)>('qvp_resolve');
  late final double Function(PtrPage) naturalPitch = lib.lookupFunction<ffi.Float Function(PtrPage), double Function(PtrPage)>('qvp_natural_pitch');

  // metadata
  late final int Function(PtrPage) surahsCount = lib.lookupFunction<ffi.Uint32 Function(PtrPage), int Function(PtrPage)>('qvp_surahs_count');
  late final int Function(PtrPage, int, ffi.Pointer<QvpSurahInfoC>) surahAt =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpSurahInfoC>), int Function(PtrPage, int, ffi.Pointer<QvpSurahInfoC>)>('qvp_surah_at');
  late final int Function(PtrPage, ffi.Pointer<QvpDivisionC>, int) divisions =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpDivisionC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpDivisionC>, int)>('qvp_divisions');
  late final int Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, int) ayahMarks =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpAyahMarkC>, int)>('qvp_ayah_marks');
  late final int Function(PtrPage, ffi.Pointer<QvpRosetteC>, int) rosettes =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpRosetteC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpRosetteC>, int)>('qvp_rosettes');
  late final int Function(PtrPage, ffi.Pointer<QvpSajdahC>, int) sajdahs =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpSajdahC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpSajdahC>, int)>('qvp_sajdahs');
  late final int Function(PtrPage, PtrU32, int) ayahKeys =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_ayah_keys');
  late final int Function(PtrPage, int, int, PtrU32) ayahWordCount =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint16, ffi.Uint16, PtrU32), int Function(PtrPage, int, int, PtrU32)>('qvp_ayah_word_count');
  late final int Function(PtrPage, int, int, int, PtrU32, int) reciteMap =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint16, ffi.Uint16, ffi.Uint32, PtrU32, ffi.Uint32), int Function(PtrPage, int, int, int, PtrU32, int)>('qvp_recite_map');
  late final void Function(PtrPage, int, PtrStr) wordLabel =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, PtrStr), void Function(PtrPage, int, PtrStr)>('qvp_word_label');
  late final void Function(PtrPage, int, PtrStr) ayahLabel =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, PtrStr), void Function(PtrPage, int, PtrStr)>('qvp_ayah_label');

  // text & search
  late final void Function(PtrPage, PtrU32, int, int, PtrU8, int, PtrU8, int, PtrStr) text = lib.lookupFunction<
      ffi.Void Function(PtrPage, PtrU32, ffi.Uint32, ffi.Uint8, PtrU8, ffi.Uint32, PtrU8, ffi.Uint32, PtrStr),
      void Function(PtrPage, PtrU32, int, int, PtrU8, int, PtrU8, int, PtrStr)>('qvp_text');
  late final void Function(PtrPage, PtrTarget, int, PtrU8, int, PtrU8, int, PtrStr) textTarget = lib.lookupFunction<
      ffi.Void Function(PtrPage, PtrTarget, ffi.Uint8, PtrU8, ffi.Uint32, PtrU8, ffi.Uint32, PtrStr),
      void Function(PtrPage, PtrTarget, int, PtrU8, int, PtrU8, int, PtrStr)>('qvp_text_target');
  late final int Function(PtrPage, PtrU8, int, int, int, int, int, int, ffi.Pointer<QvpMatchC>, int) search = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrU8, ffi.Uint32, ffi.Uint8, ffi.Uint8, ffi.Uint32, ffi.Uint32, ffi.Uint32, ffi.Pointer<QvpMatchC>, ffi.Uint32),
      int Function(PtrPage, PtrU8, int, int, int, int, int, int, ffi.Pointer<QvpMatchC>, int)>('qvp_search');
  late final void Function(int, PtrU8, int, PtrStr) arabic =
      lib.lookupFunction<ffi.Void Function(ffi.Uint8, PtrU8, ffi.Uint32, PtrStr), void Function(int, PtrU8, int, PtrStr)>('qvp_arabic');
  late final void Function(PtrPage, PtrU32, int, PtrStr) citation =
      lib.lookupFunction<ffi.Void Function(PtrPage, PtrU32, ffi.Uint32, PtrStr), void Function(PtrPage, PtrU32, int, PtrStr)>('qvp_citation');
  late final int Function(PtrPage, PtrU8, int) attachWords =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, PtrU8, ffi.Uint32), int Function(PtrPage, PtrU8, int)>('qvp_attach_words');
  late final int Function(PtrPage, int) hasForm = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint8), int Function(PtrPage, int)>('qvp_has_form');

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

  // layout
  late final void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpLayoutC>) layout = lib.lookupFunction<
      ffi.Void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpLayoutC>),
      void Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Pointer<QvpLayoutC>)>('qvp_layout');
  late final double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, double) layoutGapToFill = lib.lookupFunction<
      ffi.Float Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, ffi.Float),
      double Function(PtrPage, ffi.Pointer<QvpLayoutSpecC>, double)>('qvp_layout_gap_to_fill');
  late final double Function(double, double, int, double, double, double) gapToFill = lib.lookupFunction<
      ffi.Float Function(ffi.Float, ffi.Float, ffi.Uint32, ffi.Float, ffi.Float, ffi.Float),
      double Function(double, double, int, double, double, double)>('qvp_gap_to_fill');
  late final double Function(double, double, double, double) wastedFraction = lib.lookupFunction<
      ffi.Float Function(ffi.Float, ffi.Float, ffi.Float, ffi.Float), double Function(double, double, double, double)>('qvp_wasted_fraction');
  late final int Function(PtrPage, int, PtrF32) wordBoundsView =
      lib.lookupFunction<ffi.Int32 Function(PtrPage, ffi.Uint32, PtrF32), int Function(PtrPage, int, PtrF32)>('qvp_word_bounds_view');

  // styles
  late final int Function(PtrPage, int, ffi.Pointer<QvpSelectorC>, int, int) styleAdd = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Int32, ffi.Pointer<QvpSelectorC>, ffi.Uint32, ffi.Uint32),
      int Function(PtrPage, int, ffi.Pointer<QvpSelectorC>, int, int)>('qvp_style_add');
  late final int Function(PtrPage, int, PtrTarget, int, int) styleAddTarget = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Int32, PtrTarget, ffi.Uint32, ffi.Uint32), int Function(PtrPage, int, PtrTarget, int, int)>('qvp_style_add_target');
  late final int Function(PtrPage, int) styleRemove = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_style_remove');
  late final int Function(PtrPage, int, int, int) styleRepaint =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32, ffi.Uint32, ffi.Uint32), int Function(PtrPage, int, int, int)>('qvp_style_repaint');
  late final void Function(PtrPage) styleClear = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_style_clear');
  late final void Function(PtrPage, int) styleClearLayer = lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Int32), void Function(PtrPage, int)>('qvp_style_clear_layer');
  late final void Function(PtrPage, int) styleDefault = lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32), void Function(PtrPage, int)>('qvp_style_default');
  late final int Function(PtrPage, ffi.Pointer<QvpSelectorC>) hide =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpSelectorC>), int Function(PtrPage, ffi.Pointer<QvpSelectorC>)>('qvp_hide');
  late final int Function(PtrPage, ffi.Pointer<QvpThemeC>) theme =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpThemeC>), int Function(PtrPage, ffi.Pointer<QvpThemeC>)>('qvp_theme');
  late final int Function(PtrPage, PtrU32, int) styleHandles =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_style_handles');

  // clock & display list
  late final int Function(PtrPage, double) tick = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Double), int Function(PtrPage, double)>('qvp_tick');
  late final PtrU32 Function(PtrPage) paint = lib.lookupFunction<PtrU32 Function(PtrPage), PtrU32 Function(PtrPage)>('qvp_paint');
  late final int Function(PtrPage, PtrU32, int) styled = lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_styled');
  late final int Function(PtrPage, int) colorOf = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_color_of');

  // highlights
  late final int Function(PtrPage, PtrTarget, ffi.Pointer<QvpHighlightStyleC>) highlight = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrTarget, ffi.Pointer<QvpHighlightStyleC>), int Function(PtrPage, PtrTarget, ffi.Pointer<QvpHighlightStyleC>)>('qvp_highlight');
  late final int Function(PtrPage, int, PtrTarget) rehighlight =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32, PtrTarget), int Function(PtrPage, int, PtrTarget)>('qvp_rehighlight');
  late final int Function(PtrPage, int, ffi.Pointer<QvpHighlightStyleC>) restyleHighlight = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Uint32, ffi.Pointer<QvpHighlightStyleC>), int Function(PtrPage, int, ffi.Pointer<QvpHighlightStyleC>)>('qvp_restyle_highlight');
  late final int Function(PtrPage, int) unhighlight = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_unhighlight');
  late final void Function(PtrPage) clearHighlights = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_clear_highlights');
  late final int Function(PtrPage, PtrU32, int) highlightHandles =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_highlight_handles');
  late final int Function(PtrPage, int, PtrU32, int) highlightWords =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32, PtrU32, ffi.Uint32), int Function(PtrPage, int, PtrU32, int)>('qvp_highlight_words');
  late final int Function(PtrPage, ffi.Pointer<QvpBoxC>, int) highlightBoxesView =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpBoxC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpBoxC>, int)>('qvp_highlight_boxes_view');
  late final int Function(PtrPage, PtrU32, int, int, double, double, ffi.Pointer<QvpBoxC>, int) wordBands = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32, ffi.Uint8, ffi.Float, ffi.Float, ffi.Pointer<QvpBoxC>, ffi.Uint32),
      int Function(PtrPage, PtrU32, int, int, double, double, ffi.Pointer<QvpBoxC>, int)>('qvp_word_bands');

  // selection
  late final void Function(PtrPage, int, int) select =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, ffi.Uint32), void Function(PtrPage, int, int)>('qvp_select');
  late final int Function(PtrPage, PtrU32, int) selection =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_selection');
  late final void Function(PtrPage, int, int, PtrStr) selectionText =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint8, ffi.Uint32, PtrStr), void Function(PtrPage, int, int, PtrStr)>('qvp_selection_text');

  // memorisation
  late final void Function(PtrPage, PtrTarget, int) mask = lib.lookupFunction<ffi.Void Function(PtrPage, PtrTarget, ffi.Uint8), void Function(PtrPage, PtrTarget, int)>('qvp_mask');
  late final void Function(PtrPage, int, int) maskFrom =
      lib.lookupFunction<ffi.Void Function(PtrPage, ffi.Uint32, ffi.Uint8), void Function(PtrPage, int, int)>('qvp_mask_from');
  late final void Function(PtrPage, int, double, double, double, int) maskOptions = lib.lookupFunction<
      ffi.Void Function(PtrPage, ffi.Uint32, ffi.Float, ffi.Float, ffi.Float, ffi.Uint32), void Function(PtrPage, int, double, double, double, int)>('qvp_mask_options');
  late final int Function(PtrPage, int) revealNext = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_reveal_next');
  late final int Function(PtrPage, int) hideBack = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_hide_back');
  late final int Function(PtrPage, int) revealWord = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_reveal_word');
  late final int Function(PtrPage, int) hideWord = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_hide_word');
  late final void Function(PtrPage) revealAll = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_reveal_all');
  late final void Function(PtrPage) hideAll = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_hide_all');
  late final void Function(PtrPage) unmask = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_unmask');
  late final int Function(PtrPage, PtrU32, int) maskHidden =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_mask_hidden');
  late final int Function(PtrPage, PtrU32, int) maskWords =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, PtrU32, ffi.Uint32), int Function(PtrPage, PtrU32, int)>('qvp_mask_words');
  late final int Function(PtrPage, ffi.Pointer<QvpBoxC>, int) maskBoxesView =
      lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Pointer<QvpBoxC>, ffi.Uint32), int Function(PtrPage, ffi.Pointer<QvpBoxC>, int)>('qvp_mask_boxes_view');
  late final int Function(PtrPage, int, int, int, int, int, int) revealStart = lib.lookupFunction<
      ffi.Uint32 Function(PtrPage, ffi.Uint32, ffi.Uint32, ffi.Uint32, ffi.Uint32, ffi.Uint32, ffi.Uint32),
      int Function(PtrPage, int, int, int, int, int, int)>('qvp_reveal_start');
  late final int Function(PtrPage, int) revealGoto = lib.lookupFunction<ffi.Uint32 Function(PtrPage, ffi.Int64), int Function(PtrPage, int)>('qvp_reveal_goto');
  late final int Function(PtrPage) revealAt = lib.lookupFunction<ffi.Int64 Function(PtrPage), int Function(PtrPage)>('qvp_reveal_at');
  late final int Function(PtrPage) revealSteps = lib.lookupFunction<ffi.Uint32 Function(PtrPage), int Function(PtrPage)>('qvp_reveal_steps');
  late final int Function(PtrPage, int) revealStepOf = lib.lookupFunction<ffi.Int64 Function(PtrPage, ffi.Uint32), int Function(PtrPage, int)>('qvp_reveal_step_of');
  late final void Function(PtrPage) revealStop = lib.lookupFunction<ffi.Void Function(PtrPage), void Function(PtrPage)>('qvp_reveal_stop');

  // crop
  late final int Function(PtrPage, PtrTarget, double, int, ffi.Pointer<QvpCropBoundsC>) cropBounds = lib.lookupFunction<
      ffi.Int32 Function(PtrPage, PtrTarget, ffi.Float, ffi.Uint32, ffi.Pointer<QvpCropBoundsC>), int Function(PtrPage, PtrTarget, double, int, ffi.Pointer<QvpCropBoundsC>)>('qvp_crop_bounds');
  late final int Function(PtrPage, PtrTarget, double, int, int, PtrStr) cropSvg = lib.lookupFunction<
      ffi.Int32 Function(PtrPage, PtrTarget, ffi.Float, ffi.Uint32, ffi.Uint32, PtrStr), int Function(PtrPage, PtrTarget, double, int, int, PtrStr)>('qvp_crop_svg');

  // atlas
  late final PtrAtlas Function(PtrU8, int) atlasLoad = lib.lookupFunction<PtrAtlas Function(PtrU8, ffi.Size), PtrAtlas Function(PtrU8, int)>('qvp_atlas_load');
  late final void Function(PtrAtlas) atlasFree = lib.lookupFunction<ffi.Void Function(PtrAtlas), void Function(PtrAtlas)>('qvp_atlas_free');
  late final int Function(PtrAtlas, int, int) atlasPageOf =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint16, ffi.Uint16), int Function(PtrAtlas, int, int)>('qvp_atlas_page_of');
  late final int Function(PtrAtlas, int, PtrU16) atlasPageRange =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint16, PtrU16), int Function(PtrAtlas, int, PtrU16)>('qvp_atlas_page_range');
  late final int Function(PtrAtlas) atlasPages = lib.lookupFunction<ffi.Uint32 Function(PtrAtlas), int Function(PtrAtlas)>('qvp_atlas_pages');
  late final int Function(PtrAtlas) atlasSurahs = lib.lookupFunction<ffi.Uint32 Function(PtrAtlas), int Function(PtrAtlas)>('qvp_atlas_surahs');
  late final int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>) atlasSurah = lib.lookupFunction<
      ffi.Int32 Function(PtrAtlas, ffi.Uint16, ffi.Pointer<QvpAtlasSurahC>), int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>)>('qvp_atlas_surah');
  late final int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>) atlasSurahAt = lib.lookupFunction<
      ffi.Int32 Function(PtrAtlas, ffi.Uint32, ffi.Pointer<QvpAtlasSurahC>), int Function(PtrAtlas, int, ffi.Pointer<QvpAtlasSurahC>)>('qvp_atlas_surah_at');
  late final int Function(PtrAtlas, int, int, ffi.Pointer<QvpAtlasRubuAlHizbC>) atlasDivision = lib.lookupFunction<
      ffi.Int32 Function(PtrAtlas, ffi.Uint8, ffi.Uint16, ffi.Pointer<QvpAtlasRubuAlHizbC>), int Function(PtrAtlas, int, int, ffi.Pointer<QvpAtlasRubuAlHizbC>)>('qvp_atlas_division');
  late final int Function(PtrAtlas, int, int, int) atlasDivisionAt =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint8, ffi.Uint16, ffi.Uint16), int Function(PtrAtlas, int, int, int)>('qvp_atlas_division_at');
  late final int Function(PtrAtlas, int, PtrU16) atlasPagesOfJuz =
      lib.lookupFunction<ffi.Int32 Function(PtrAtlas, ffi.Uint16, PtrU16), int Function(PtrAtlas, int, PtrU16)>('qvp_atlas_pages_of_juz');
  late final int Function(PtrAtlas, PtrU8, int, PtrU16, int) atlasFindSurah = lib.lookupFunction<
      ffi.Uint32 Function(PtrAtlas, PtrU8, ffi.Uint32, PtrU16, ffi.Uint32), int Function(PtrAtlas, PtrU8, int, PtrU16, int)>('qvp_atlas_find_surah');
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
  late final int Function() version = lib.lookupFunction<ffi.Uint32 Function(), int Function()>('qvp_version');
  late final ffi.Pointer<ffi.Char> Function() engineName =
      lib.lookupFunction<ffi.Pointer<ffi.Char> Function(), ffi.Pointer<ffi.Char> Function()>('qvp_engine_name');
}
