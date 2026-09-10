// QvpEngine / QvpPage / QvpAtlas — the Dart mirror of web/qvp.js over the C
// ABI in crates/qvp-ffi/include/qvp.h.
//
// Everything that decides (hit-testing, layout, style resolution, highlights,
// masks, search, text) lives in the engine; this file only marshals. Geometry
// is copied out of native memory once per page; every other call goes straight
// to the engine. Method names are identical to the web wrapper so the docs are
// shared.
import 'dart:convert';
import 'dart:ffi' as ffi;
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' show Color;

import 'package:ffi/ffi.dart' as pffi;
import 'package:flutter/foundation.dart';

import 'bindings.dart';

/// `QVP_NONE`: absent index.
const int qvpNone = 0xffffffff;

// ───────────── enums (ints, so they interoperate with the geometry table) ─────────────

/// `QVP_KIND_*`
abstract final class QvpKind {
  static const int body = 0, mark = 1, ayahNumber = 2, ayahMarkOrnament = 3, headerInk = 4, other = 255;
}

/// `QVP_FAMILY_*`
abstract final class QvpFamily {
  static const int none = 0, diacritic = 1, tanwin = 2, dots = 3, waqf = 4, sifr = 5, sajdah = 6, readingSign = 7;
}

/// `QVP_CATEGORY_*`
abstract final class QvpCategory {
  static const int none = 0, harakah = 1, tanwin = 2, letterDot = 3, orthographic = 4, dabt = 5, waqf = 6, readingSign = 7, standalone = 8;
}

/// `QVP_DECO_*`
abstract final class QvpDeco {
  static const int ayahMark = 0, surahName = 1, basmalah = 2, divisionMark = 3, sajdahMark = 4;
}

/// `QVP_FORM_*` — text forms. Strings 'rasmUthmani' | 'rasmImlai' | 'qpc' | 'rasm' | 'search' are accepted everywhere a form is.
abstract final class QvpForm {
  static const int rasmUthmani = 0, rasmImlai = 1, qpc = 2, rasm = 3, search = 4;
  /// Both the Dart spelling and the engine's own `rasm_uthmani` / `rasm_imlai` resolve.
  static const Map<String, int> byName = {
    'rasmUthmani': 0, 'rasm_uthmani': 0,
    'rasmImlai': 1, 'rasm_imlai': 1,
    'qpc': 2, 'rasm': 3, 'search': 4,
  };
  static int of(Object f) => f is int ? f : (byName[f as String] ?? 0);
}

/// `QVP_LAYER_*` — style layers (higher wins).
abstract final class QvpLayer {
  static const int base = 0, theme = 10, highlight = 50, selection = 60, top = 100;
}

/// `QVP_DIV_*`
abstract final class QvpDiv {
  static const int juz = 0, hizb = 1, nisf = 2, rubuAlHizb = 3;
  static const List<String> names = ['juz', 'hizb', 'nisf', 'rubuAlHizb'];
  static int of(Object k) => k is int ? k : names.indexOf(k as String);
}

/// Mark ids ↔ names (`qvp_mark_name`). Index = id; 255 = unknown.
abstract final class QvpMark {
  static const List<String> names = [
    '', 'fathah', 'kasrah', 'dammah', 'tanwin_al_fath', 'tanwin_al_kasr', 'tanwin_al_damm', 'shaddah', 'sukun', 'maddah', 'hamzah', 'hamzat_al_wasl', 'omitted_alif', 'small_waw', 'small_yaa', 'small_noon', 'dot', 'two_dots', 'three_dots', 'rounded_zero', 'rectangular_zero', 'waqf_jaiz_mustawi_al_tarafayn', 'waqf_jaiz_waqf_awla', 'waqf_jaiz_wasl_awla', 'waqf_lazim', 'waqf_al_muanaqah', 'saktah', 'small_meem', 'hizb', 'sajdah', 'sajdah_mark', 'sajdah_line', 'seen_al_qiraah', 'tashil', 'ishmam', 'imalah',
  ];

  /// Mark id from a name or an id; unknown names → 255.
  static int id(Object m) {
    if (m is int) return m;
    final i = names.indexOf(m as String);
    return i <= 0 ? 255 : i;
  }
}

/// Path opcodes in [QvpPage.ops].
abstract final class QvpOp {
  static const int moveTo = 0, lineTo = 1, quadTo = 2, cubicTo = 3, close = 4;
}

// ───────────── colours ─────────────

/// 0xRRGGBBAA from an int (passed through), a `#rgb` / `#rrggbb` / `#rrggbbaa` string, or a Flutter [Color].
/// [alpha] (0..1) replaces the alpha of 6-digit strings and Colors.
int rgba(Object c, [double? alpha]) {
  if (c is int) return c & 0xffffffff;
  if (c is Color) return QvpColor.fromColor(c, alpha);
  var h = (c as String).trim();
  if (h.startsWith('#')) h = h.substring(1);
  if (h.length == 3 || h.length == 4) h = h.split('').map((x) => x + x).join();
  if (h.length == 6) h += alpha != null ? (alpha * 255).round().clamp(0, 255).toRadixString(16).padLeft(2, '0') : 'ff';
  return int.parse(h, radix: 16) & 0xffffffff;
}

/// 0xRRGGBBAA ↔ Flutter [Color].
abstract final class QvpColor {
  static Color toColor(int rgba) => Color.fromARGB(rgba & 0xff, (rgba >> 24) & 0xff, (rgba >> 16) & 0xff, (rgba >> 8) & 0xff);
  static int fromColor(Color c, [double? alpha]) {
    final a = alpha != null ? (alpha * 255).round().clamp(0, 255) : (c.a * 255).round();
    return (((c.r * 255).round() << 24) | ((c.g * 255).round() << 16) | ((c.b * 255).round() << 8) | a) & 0xffffffff;
  }

  /// CSS-style string `#rrggbbaa`.
  static String hex(int rgba) => '#${rgba.toRadixString(16).padLeft(8, '0')}';
  static int withAlpha(int rgba, double alpha) => (rgba & 0xffffff00) | (alpha * 255).round().clamp(0, 255);
}

// ───────────── selectors & targets ─────────────

/// What a style rule applies to (`QvpSelector`, `QVP_SEL_*`). Build with [Sel].
@immutable
final class QvpSelector {
  const QvpSelector(this.kind, [this.a = 0, this.b = 0, this.c = 0]);
  final int kind, a, b, c;
  @override
  String toString() => 'Sel($kind,$a,$b,$c)';
}

/// Selector constructors (same names as web/qvp.js `Sel`).
abstract final class Sel {
  static QvpSelector page() => const QvpSelector(0);
  static QvpSelector path(int i) => QvpSelector(1, i);
  static QvpSelector wordPath(int w, int n) => QvpSelector(2, w, n);

  /// nth mark of the word (0-based)
  static QvpSelector wordMark(int w, int n) => QvpSelector(3, w, n);
  static QvpSelector wordMarkNamed(int w, Object mark, [int n = 0]) => QvpSelector(4, w, QvpMark.id(mark), n);
  static QvpSelector wordBody(int w) => QvpSelector(5, w);
  static QvpSelector wordMarks(int w) => QvpSelector(6, w);
  static QvpSelector word(int w) => QvpSelector(7, w);
  static QvpSelector ayah(int s, int a) => QvpSelector(8, s, a);
  static QvpSelector line(int n) => QvpSelector(9, n);
  static QvpSelector mark(Object m) => QvpSelector(10, QvpMark.id(m));
  static QvpSelector category(int c) => QvpSelector(11, c);
  static QvpSelector family(int f) => QvpSelector(12, f);
  static QvpSelector kind(int k) => QvpSelector(13, k);
  static QvpSelector deco(int k) => QvpSelector(14, k);
  static QvpSelector decoIdx(int i) => QvpSelector(15, i);
}

/// What resolves to a word list (`QvpTarget`, `QVP_TARGET_*`). Build with [T]
/// or pass a string anywhere a target is accepted: 'page', '2:255', '2:255:3',
/// '2:255-257', 'line:7', 'surah:2'. A `List<int>` of word indices is a target too.
@immutable
final class QvpTarget {
  const QvpTarget(this.kind, {this.a = 0, this.b = 0, this.c = 0, this.words, this.wordKey});
  final int kind, a, b, c;
  final List<int>? words;

  /// `[surah, ayah, word]` for the '2:255:3' string form; resolved per page via `findWord`.
  final List<int>? wordKey;

  static QvpTarget parse(String s) {
    if (s == 'page') return T.page();
    RegExpMatch? m;
    if ((m = RegExp(r'^line:(\d+)$').firstMatch(s)) != null) return T.line(int.parse(m!.group(1)!));
    if ((m = RegExp(r'^surah:(\d+)$').firstMatch(s)) != null) return T.surah(int.parse(m!.group(1)!));
    if ((m = RegExp(r'^(\d+):(\d+)-(\d+)$').firstMatch(s)) != null) {
      return T.ayahRange(int.parse(m!.group(1)!), int.parse(m.group(2)!), int.parse(m.group(3)!));
    }
    if ((m = RegExp(r'^(\d+):(\d+):(\d+)$').firstMatch(s)) != null) {
      return QvpTarget(1, wordKey: [int.parse(m!.group(1)!), int.parse(m.group(2)!), int.parse(m.group(3)!)]);
    }
    if ((m = RegExp(r'^(\d+):(\d+)$').firstMatch(s)) != null) return T.ayah(int.parse(m!.group(1)!), int.parse(m.group(2)!));
    throw ArgumentError('bad target $s');
  }

  /// Coerces a String / QvpTarget / `List<int>` to a target.
  static QvpTarget of(Object t) {
    if (t is QvpTarget) return t;
    if (t is String) return parse(t);
    if (t is List<int>) return T.words(t);
    throw ArgumentError('bad target $t');
  }
}

/// Target constructors (same names as web/qvp.js `T`).
abstract final class T {
  static QvpTarget page() => const QvpTarget(0);
  static QvpTarget word(int i) => QvpTarget(1, a: i);
  static QvpTarget words(List<int> ws) => QvpTarget(2, words: ws);
  static QvpTarget ayah(int s, int a) => QvpTarget(3, a: s, b: a);
  static QvpTarget ayahRange(int s, int a, int b) => QvpTarget(4, a: s, b: a, c: b);
  static QvpTarget line(int n) => QvpTarget(5, a: n);
  static QvpTarget surah(int s) => QvpTarget(6, a: s);
  static QvpTarget range(int a, int b) => QvpTarget(7, a: a, b: b);
}

// ───────────── records ─────────────

@immutable
final class QvpWordInfo {
  const QvpWordInfo({
    required this.idx,
    required this.surah,
    required this.ayah,
    required this.word,
    required this.line,
    required this.ayahIdx,
    required this.lineIdx,
    required this.x0,
    required this.y0,
    required this.x1,
    required this.y1,
    required this.text,
    required this.firstPath,
    required this.nPaths,
  });
  final int idx, surah, ayah, word, line, ayahIdx, lineIdx, firstPath, nPaths;
  final double x0, y0, x1, y1;

  /// `rasm_uthmani`, the text of record (inline in the QVP file).
  final String text;

  /// `surah:ayah:word`
  String get wordKey => '$surah:$ayah:$word';

  /// `surah:ayah`
  String get ayahKey => '$surah:$ayah';
}

@immutable
final class QvpAyahInfo {
  const QvpAyahInfo({
    required this.idx,
    required this.surah,
    required this.ayah,
    required this.fragment,
    required this.fragments,
    required this.flags,
    required this.rubuAlHizb,
    required this.firstWord,
    required this.nWords,
    required this.ayahMarkDeco,
    required this.x0,
    required this.y0,
    required this.x1,
    required this.y1,
  });
  final int idx, surah, ayah, fragment, fragments, flags, rubuAlHizb, firstWord, nWords;

  /// Deco index of the ayah mark or [qvpNone].
  final int ayahMarkDeco;
  final double x0, y0, x1, y1;
}

@immutable
final class QvpLineInfo {
  const QvpLineInfo({
    required this.idx,
    required this.lineNo,
    required this.isHeader,
    required this.firstWord,
    required this.nWords,
    required this.x0,
    required this.y0,
    required this.x1,
    required this.y1,
    required this.bandY0,
    required this.bandY1,
    required this.centre,
  });
  final int idx, lineNo, firstWord, nWords;
  final bool isHeader;
  final double x0, y0, x1, y1, bandY0, bandY1, centre;
}

@immutable
final class QvpDecoInfo {
  const QvpDecoInfo({
    required this.idx,
    required this.kind,
    required this.surah,
    required this.ayah,
    required this.line,
    required this.x0,
    required this.y0,
    required this.x1,
    required this.y1,
    required this.text,
    required this.firstPath,
    required this.nPaths,
  });
  final int idx, kind, surah, ayah, line, firstPath, nPaths;
  final double x0, y0, x1, y1;
  final String text;
}

/// Exact hit (`QvpHit`); indices are -1 when absent.
@immutable
final class QvpHit {
  const QvpHit({required this.word, required this.path, required this.deco});
  final int word, path, deco;
  @override
  String toString() => 'QvpHit(word: $word, path: $path, deco: $deco)';
}

/// Gap-aware hit (`QvpHitEx`); indices are -1 when absent.
@immutable
final class QvpHitEx {
  const QvpHitEx({required this.word, required this.path, required this.deco, required this.line, required this.distance, required this.exact, this.wordKey, this.ayahKey});
  final int word, path, deco, line;
  final double distance;
  final bool exact;
  final String? wordKey, ayahKey;
  @override
  String toString() => 'QvpHitEx(word: $word, path: $path, deco: $deco, line: $line, distance: $distance, exact: $exact)';
}

/// Options of the gap-aware hit test.
@immutable
final class QvpHitOptions {
  const QvpHitOptions({this.maxDistance = 0, this.gapBias = 0.6, this.exactFirst = true});

  /// Page units; <= 0 unlimited.
  final double maxDistance;
  final double gapBias;
  final bool exactFirst;
}

/// A band / mask box in layout viewport px (`QvpBox`).
@immutable
final class QvpBox {
  const QvpBox({required this.id, required this.line, required this.x0, required this.y0, required this.x1, required this.y1, required this.color, required this.radius});
  final int id, line, color;
  final double x0, y0, x1, y1, radius;
}

@immutable
final class QvpHitBox {
  const QvpHitBox({required this.word, required this.line, required this.x0, required this.y0, required this.x1, required this.y1, required this.inkX0, required this.inkY0, required this.inkX1, required this.inkY1});
  final int word, line;
  final double x0, y0, x1, y1, inkX0, inkY0, inkX1, inkY1;
}

@immutable
final class QvpLineBand {
  const QvpLineBand({required this.line, required this.lineNo, required this.y0, required this.y1, required this.mid, required this.inkY0, required this.inkY1});
  final int line, lineNo;
  final double y0, y1, mid, inkY0, inkY1;
}

/// Input of [QvpPage.layout]; lengths in viewport px.
@immutable
final class QvpLayoutSpec {
  const QvpLayoutSpec({
    this.viewportW = 0,
    this.viewportH = 0,
    this.padTop = 0,
    this.padBottom = 0,
    this.padLeft = 0,
    this.padRight = 0,
    this.lineSpacing = 1,
    this.lineGap = 0,
    this.fillHeight = false,
    this.nominalLines = 15,
  });
  final double viewportW, viewportH, padTop, padBottom, padLeft, padRight, lineSpacing, lineGap;
  final bool fillHeight;
  final int nominalLines;

  QvpLayoutSpec copyWith({
    double? viewportW,
    double? viewportH,
    double? padTop,
    double? padBottom,
    double? padLeft,
    double? padRight,
    double? lineSpacing,
    double? lineGap,
    bool? fillHeight,
    int? nominalLines,
  }) =>
      QvpLayoutSpec(
        viewportW: viewportW ?? this.viewportW,
        viewportH: viewportH ?? this.viewportH,
        padTop: padTop ?? this.padTop,
        padBottom: padBottom ?? this.padBottom,
        padLeft: padLeft ?? this.padLeft,
        padRight: padRight ?? this.padRight,
        lineSpacing: lineSpacing ?? this.lineSpacing,
        lineGap: lineGap ?? this.lineGap,
        fillHeight: fillHeight ?? this.fillHeight,
        nominalLines: nominalLines ?? this.nominalLines,
      );

  @override
  bool operator ==(Object other) =>
      other is QvpLayoutSpec &&
      other.viewportW == viewportW &&
      other.viewportH == viewportH &&
      other.padTop == padTop &&
      other.padBottom == padBottom &&
      other.padLeft == padLeft &&
      other.padRight == padRight &&
      other.lineSpacing == lineSpacing &&
      other.lineGap == lineGap &&
      other.fillHeight == fillHeight &&
      other.nominalLines == nominalLines;

  @override
  int get hashCode => Object.hash(viewportW, viewportH, padTop, padBottom, padLeft, padRight, lineSpacing, lineGap, fillHeight, nominalLines);
}

/// Output of [QvpPage.layout]. Page → viewport: `vx = ox + x*scale`, `vy = oy + (y + lineDy[line])*scale`.
@immutable
final class QvpLayout {
  const QvpLayout({required this.scale, required this.ox, required this.oy, required this.contentW, required this.contentH, required this.pitch, required this.lineDy, required this.slots});
  final double scale, ox, oy, contentW, contentH, pitch;

  /// Per-line vertical shift in page units (index = line record index).
  final Float32List lineDy;

  /// Per-line `(top, bottom)` slot in viewport px.
  final List<(double, double)> slots;
}

/// One highlight description. Colours accept anything [rgba] does.
@immutable
final class QvpHighlightStyle {
  const QvpHighlightStyle({
    this.mode = 'band',
    this.height = 'pitch',
    this.ink = 0x1a73e8ff,
    this.band = 0xd6a3264d,
    this.padX = 1.2,
    this.padY = 0,
    this.radius = 0,
    this.seam = 0.25,
    this.ms = 0,
    this.layer = QvpLayer.highlight,
  });

  /// 'ink' | 'band' | 'both'
  final String mode;

  /// 'pitch' | 'ink'
  final String height;
  final Object ink, band;
  final double padX, padY, radius, seam;
  final int ms, layer;
}

/// A theme: every colour is optional (0 / null = leave alone); `marks` maps mark name or id → colour.
@immutable
final class QvpTheme {
  const QvpTheme({this.ink, this.diacritics, this.dots, this.waqf, this.sifr, this.ayahMark, this.numeral, this.headers, this.marks = const {}, this.ms = 0});
  final Object? ink, diacritics, dots, waqf, sifr, ayahMark, numeral, headers;
  final Map<Object, Object> marks;
  final int ms;
}

@immutable
final class QvpSurahInfo {
  const QvpSurahInfo({required this.number, required this.ayahCount, required this.hasBanner, required this.hasBasmalah, required this.place, required this.bannerDeco, required this.arabic, required this.latin, required this.english});
  final int number, ayahCount, bannerDeco;
  final bool hasBanner, hasBasmalah;

  /// 'makkah' | 'madinah' | ''
  final String place, arabic, latin, english;
}

@immutable
final class QvpDivision {
  const QvpDivision({required this.kind, required this.line, required this.n, required this.surah, required this.ayah, required this.ayahIdx});

  /// 'juz' | 'hizb' | 'nisf' | 'rubuAlHizb'
  final String kind;
  final int line, n, surah, ayah, ayahIdx;
}

@immutable
final class QvpAyahMark {
  const QvpAyahMark({required this.deco, required this.surah, required this.ayah, required this.line, required this.cx, required this.cy, required this.r, required this.ornamentPath, required this.numeralPath});
  final int deco, surah, ayah, line, ornamentPath, numeralPath;
  final double cx, cy, r;
}

@immutable
final class QvpRosette {
  const QvpRosette({required this.deco, required this.surah, required this.ayah, required this.juz, required this.hizb, required this.nisf, required this.rubuAlHizb, required this.rubuAlHizbInHizb});
  final int deco, surah, ayah, juz, hizb, nisf, rubuAlHizb, rubuAlHizbInHizb;
}

@immutable
final class QvpSajdah {
  const QvpSajdah({required this.deco, required this.surah, required this.ayah, required this.signPath});
  final int deco, surah, ayah, signPath;
}

@immutable
final class QvpMatch {
  const QvpMatch({required this.word, required this.index, required this.loose, required this.wordKey, required this.text});
  final int word, index;
  final bool loose;
  final String wordKey, text;
}

@immutable
final class QvpCropBox {
  const QvpCropBox({required this.x0, required this.y0, required this.x1, required this.y1, required this.nWords, required this.ayahMarkDeco});
  final double x0, y0, x1, y1;
  final int nWords, ayahMarkDeco;
}

@immutable
final class QvpAtlasSurah {
  const QvpAtlasSurah({required this.n, required this.page, required this.ayahCount, required this.place, required this.arabic, required this.latin, required this.english});
  final int n, page, ayahCount;
  final String place, arabic, latin, english;
}

@immutable
final class QvpAtlasRubuAlHizb {
  const QvpAtlasRubuAlHizb({required this.rubuAlHizb, required this.surah, required this.ayah, required this.page});
  final int rubuAlHizb, surah, ayah, page;
  String get ayahKey => '$surah:$ayah';
}

/// What a mushaf's ornaments may be redistributed under. Traced ornaments
/// belong to their publisher; read this before publishing a dressed page.
@immutable
final class QvpOrnamentLicence {
  const QvpOrnamentLicence({required this.id, required this.status, required this.redistributable, required this.attribution});
  final String id, status, attribution;
  final bool redistributable;
}

/// One printed colour of a design. `slot` is the window the design leaves open
/// for the thing it frames, and has no colour of its own (alpha 0).
@immutable
final class QvpOrnamentPart {
  const QvpOrnamentPart({required this.index, required this.name, required this.color, required this.stroke});
  final int index, color;
  final String name;
  final bool stroke;
}

/// One mushaf's ornaments.
@immutable
final class QvpOrnamentStyle {
  const QvpOrnamentStyle({
    required this.index,
    required this.name,
    required this.riwayah,
    required this.hasAyahMark,
    required this.hasSurahHeader,
    required this.hasPageFrame,
    required this.tiles,
    required this.licence,
    required this.parts,
  });
  final int index;
  final String name, riwayah;
  final bool hasAyahMark, hasSurahHeader, hasPageFrame;

  /// The frame is assembled from a corner and two repeat units rather than
  /// stretched whole.
  final bool tiles;
  final QvpOrnamentLicence licence;
  final List<QvpOrnamentPart> parts;
}

/// What dressing the page did.
@immutable
final class QvpDress {
  const QvpDress({
    required this.style,
    required this.ayahMarks,
    required this.surahHeaders,
    required this.frameRepeats,
    required this.frameStretched,
    required this.nDraws,
    required this.revision,
    required this.viewBox,
  });
  final int style, ayahMarks, surahHeaders, frameRepeats, nDraws;

  /// Bumped on every rebuild of the display list; cache a raster against it.
  final int revision;
  final bool frameStretched;

  /// The page's viewBox after the border grew it: x, y, w, h in page units.
  final (double, double, double, double) viewBox;
}

/// What an ornament replaces.
abstract final class QvpOrnamentKind {
  static const int ayahMark = 0;
  static const int surahHeader = 1;
  static const int pageFrame = 2;
}

/// One placed ornament outline, in page units.
@immutable
final class QvpOrnamentDraw {
  const QvpOrnamentDraw({
    required this.opStart,
    required this.opCount,
    required this.ptStart,
    required this.ptCount,
    required this.color,
    required this.kind,
    required this.evenOdd,
    required this.stroke,
    required this.strokeWidth,
    required this.part,
    required this.line,
  });
  final int opStart, opCount, ptStart, ptCount, color, kind, part;
  final bool evenOdd, stroke;
  final double strokeWidth;

  /// The page line it was measured against, or -1 for the border, which is
  /// placed from the page and does not move with a line.
  final int line;
}

/// The ornament display list of a dressed page, in page units.
@immutable
final class QvpDressGeometry {
  const QvpDressGeometry({required this.ops, required this.pts, required this.draws});
  final Uint8List ops;
  final Float32List pts;
  final List<QvpOrnamentDraw> draws;
  bool get isEmpty => draws.isEmpty;
}

/// One entry of [QvpPage.styled].
typedef QvpStyledPath = ({int path, int color});

// ───────────── engine ─────────────

/// A loaded `libqvp_ffi`. Holds the scratch memory every page shares; not thread-safe (use from one isolate).
class QvpEngine {
  QvpEngine(ffi.DynamicLibrary library) : b = QvpBindings(library) {
    _scratch = pffi.calloc<ffi.Uint8>(scratchBytes);
    _str = pffi.calloc<QvpStrC>();
    _target = pffi.calloc<QvpTargetC>();
    _sel = pffi.calloc<QvpSelectorC>();
    _hl = pffi.calloc<QvpHighlightStyleC>();
    _theme = pffi.calloc<QvpThemeC>();
    _spec = pffi.calloc<QvpLayoutSpecC>();
    _layout = pffi.calloc<QvpLayoutC>();
    _hitOpt = pffi.calloc<QvpHitOptionsC>();
    _hit = pffi.calloc<QvpHitC>();
    _hitEx = pffi.calloc<QvpHitExC>();
    _crop = pffi.calloc<QvpCropBoxC>();
  }

  /// Opens the engine library: [path] if given, else `libqvp_ffi.so` from the
  /// plugin on Android, the process on iOS/macOS, else `$QVP_LIB`, else the
  /// platform default name on the loader path.
  factory QvpEngine.open({String? path}) => QvpEngine(resolveLibrary(path: path));

  static ffi.DynamicLibrary resolveLibrary({String? path}) {
    if (path != null) return ffi.DynamicLibrary.open(path);
    if (Platform.isAndroid) return ffi.DynamicLibrary.open('libqvp_ffi.so');
    if (Platform.isIOS || Platform.isMacOS) return ffi.DynamicLibrary.process();
    final env = Platform.environment['QVP_LIB'];
    if (env != null && env.isNotEmpty) return ffi.DynamicLibrary.open(env);
    if (Platform.isWindows) return ffi.DynamicLibrary.open('qvp_ffi.dll');
    return ffi.DynamicLibrary.open('libqvp_ffi.so');
  }

  /// Size of the shared output scratch buffer (array outputs are capped by it).
  static const int scratchBytes = 1 << 18;

  final QvpBindings b;
  late final ffi.Pointer<ffi.Uint8> _scratch;
  late final ffi.Pointer<QvpStrC> _str;
  late final ffi.Pointer<QvpTargetC> _target;
  late final ffi.Pointer<QvpSelectorC> _sel;
  late final ffi.Pointer<QvpHighlightStyleC> _hl;
  late final ffi.Pointer<QvpThemeC> _theme;
  late final ffi.Pointer<QvpLayoutSpecC> _spec;
  late final ffi.Pointer<QvpLayoutC> _layout;
  late final ffi.Pointer<QvpHitOptionsC> _hitOpt;
  late final ffi.Pointer<QvpHitC> _hit;
  late final ffi.Pointer<QvpHitExC> _hitEx;
  late final ffi.Pointer<QvpCropBoxC> _crop;
  bool _disposed = false;

  /// Format version the library was built for.
  int get version => b.version();

  /// `qvp_engine_name()`
  String get engineName => b.engineName().cast<pffi.Utf8>().toDartString();

  /// Scratch as an array of [T] (cap = how many fit).
  ffi.Pointer<N> _out<N extends ffi.NativeType>() => _scratch.cast<N>();

  /// How many elements of [elemSize] bytes fit in the scratch.
  int _cap(int elemSize) => scratchBytes ~/ elemSize;

  /// Decodes a QvpStr right away (the engine's string buffer is reused by the next string call).
  static String str(QvpStrC s) => s.len == 0 || s.ptr == ffi.nullptr ? '' : utf8.decode(s.ptr.asTypedList(s.len));
  String _s() => str(_str.ref);

  /// Runs [f] with [bytes] copied into native memory.
  R withBytes<R>(List<int> bytes, R Function(ffi.Pointer<ffi.Uint8> p, int len) f) {
    final n = bytes.length;
    final p = pffi.malloc<ffi.Uint8>(n == 0 ? 1 : n);
    try {
      if (n > 0) p.asTypedList(n).setAll(0, bytes);
      return f(p, n);
    } finally {
      pffi.malloc.free(p);
    }
  }

  R withString<R>(String s, R Function(ffi.Pointer<ffi.Uint8> p, int len) f) => withBytes(utf8.encode(s), f);

  R withU32<R>(List<int> v, R Function(ffi.Pointer<ffi.Uint32> p, int n) f) {
    final p = pffi.malloc<ffi.Uint32>(v.isEmpty ? 1 : v.length);
    try {
      p.asTypedList(v.length).setAll(0, v);
      return f(p, v.length);
    } finally {
      pffi.malloc.free(p);
    }
  }

  String _name(void Function(int, ffi.Pointer<QvpStrC>) fn, int v) {
    fn(v, _str);
    return _s();
  }

  String markName(int m) => m > 0 && m < QvpMark.names.length ? QvpMark.names[m] : _name(b.markName, m);
  String familyName(int f) => _name(b.familyName, f);
  String kindName(int k) => _name(b.kindName, k);
  String categoryName(int c) => _name(b.categoryName, c);
  int markCategory(Object m) => b.markCategory(QvpMark.id(m));
  int markFromName(String s) => withString(s, (p, n) => b.markFromName(p, n));

  // Arabic text tools
  String _arabic(int kind, String s) => withString(s, (p, n) {
        b.arabic(kind, p, n, _str);
        return _s();
      });
  String strip(String s) => _arabic(0, s);
  String fold(String s) => _arabic(1, s);
  String normalize(String s) => _arabic(2, s);
  String looseKey(String s) => _arabic(3, s);

  /// Decodes a page from QVP bytes (copied by the engine). Throws on a bad file.
  QvpPage loadPage(Uint8List bytes) {
    final h = withBytes(bytes, (p, n) => b.pageLoad(p, n));
    if (h == ffi.nullptr) throw const FormatException('qvp_page_load failed');
    return QvpPage._(this, h);
  }

  /// Decodes an ornament set (`ornaments.qvo`) — the medallions, surah bands
  /// and page borders of other printed mushafs. Throws on a bad file.
  ///
  /// NONE OF THESE OUTLINES IS PART OF A QVP PAGE: they are traced from scans
  /// of other prints, each with a licence of its own.
  QvpOrnaments loadOrnaments(Uint8List bytes) {
    final h = withBytes(bytes, (p, n) => b.ornamentsLoad(p, n));
    if (h == ffi.nullptr) throw const FormatException('qvp_ornaments_load failed');
    return QvpOrnaments._(this, h);
  }

  /// Decodes an atlas (`atlas.qva`). Throws on a bad file.
  QvpAtlas loadAtlas(Uint8List bytes) {
    final h = withBytes(bytes, (p, n) => b.atlasLoad(p, n));
    if (h == ffi.nullptr) throw const FormatException('qvp_atlas_load failed');
    return QvpAtlas._(this, h);
  }

  double gapToFill(double pageW, double pageH, int lines, double viewW, double viewH, [double max = 0]) => b.gapToFill(pageW, pageH, lines, viewW, viewH, max);
  double wastedFraction(double pageW, double pageH, double viewW, double viewH) => b.wastedFraction(pageW, pageH, viewW, viewH);

  /// Releases the scratch memory. Free pages and atlases first.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    for (final p in <ffi.Pointer>[_scratch, _str, _target, _sel, _hl, _theme, _spec, _layout, _hitOpt, _hit, _hitEx, _crop]) {
      pffi.calloc.free(p);
    }
  }

  // struct writers
  ffi.Pointer<QvpSelectorC> _writeSel(QvpSelector s) {
    final r = _sel.ref;
    r.kind = s.kind;
    r.a = s.a;
    r.b = s.b;
    r.c = s.c;
    return _sel;
  }

  ffi.Pointer<QvpHighlightStyleC> _writeHl(QvpHighlightStyle st) {
    final r = _hl.ref;
    r.mode = const {'ink': 0, 'band': 1, 'both': 2}[st.mode] ?? 1;
    r.height = st.height == 'ink' ? 1 : 0;
    r.ink = rgba(st.ink);
    r.band = rgba(st.band);
    r.padX = st.padX;
    r.padY = st.padY;
    r.radius = st.radius;
    r.seam = st.seam;
    r.transitionMs = st.ms;
    r.layer = st.layer;
    return _hl;
  }
}

// ───────────── page ─────────────

/// A decoded page. Geometry ([ops], [pts], [table]) and the info lists are
/// copied out of native memory once; style state, layout, highlights, masks
/// and selection live in the engine. Listeners are notified after every
/// mutating call so a host renderer can schedule a frame.
class QvpPage extends ChangeNotifier {
  QvpPage._(this.engine, ffi.Pointer<QvpPageC> handle) : _h = handle {
    final b = engine.b;
    final info = pffi.calloc<QvpPageInfoC>();
    final geom = pffi.calloc<QvpGeometryC>();
    try {
      b.pageInfo(_h, info);
      final i = info.ref;
      width = i.width;
      height = i.height;
      page = i.page;
      nLines = i.nLines;
      nAyahs = i.nAyahs;
      nWords = i.nWords;
      nPaths = i.nPaths;
      nDecos = i.nDecos;
      b.geometry(_h, geom);
      final g = geom.ref;
      ops = Uint8List.fromList(g.ops.asTypedList(g.opsLen));
      pts = Float32List.fromList(g.pts.asTypedList(g.ptsLen));
      table = Uint32List.fromList(g.table.asTypedList(g.nPaths * 8));
    } finally {
      pffi.calloc.free(info);
      pffi.calloc.free(geom);
    }
    words = List.generate(nWords, _word, growable: false);
    ayahs = List.generate(nAyahs, _ayah, growable: false);
    lines = List.generate(nLines, _line, growable: false);
    decos = List.generate(nDecos, _deco, growable: false);
    naturalPitch = b.naturalPitch(_h);
  }

  final QvpEngine engine;
  ffi.Pointer<QvpPageC> _h;

  late final double width, height, naturalPitch;
  late final int page, nLines, nAyahs, nWords, nPaths, nDecos;

  /// Opcode stream (see [QvpOp]).
  late final Uint8List ops;

  /// x,y pairs consumed in order by the opcodes.
  late final Float32List pts;

  /// `nPaths × 8`: op_start, op_count, pt_start, pt_count, flags, word, line, extra.
  late final Uint32List table;

  late final List<QvpWordInfo> words;
  late final List<QvpAyahInfo> ayahs;
  late final List<QvpLineInfo> lines;
  late final List<QvpDecoInfo> decos;

  /// Layout from the last [layout] call, if any.
  QvpLayout? currentLayout;

  int _defaultInk = 0x231f20ff;

  /// Dart-side mirror of the engine's default ink.
  int get defaultInk => _defaultInk;

  /// Bumped on every mutating call.
  int revision = 0;

  QvpDressGeometry? _dress;
  int _dressRevision = -1;

  bool get isDisposed => _h == ffi.nullptr;

  ffi.Pointer<QvpPageC> get _p {
    assert(_h != ffi.nullptr, 'QvpPage used after dispose()');
    return _h;
  }

  QvpBindings get _b => engine.b;
  QvpEngine get _e => engine;

  void _touch() {
    revision++;
    notifyListeners();
  }

  /// Frees the native page. The copied geometry and info lists stay usable.
  // ── dress: another mushaf's ornaments ──

  /// Put a mushaf's ornaments on this page. Returns the readout, or null when
  /// the set has no such style. Replaces any previous dress.
  ///
  /// [colors] is by part NAME; the design's own printed colour is kept for
  /// every part left out.
  QvpDress? dress(
    QvpOrnaments ornaments, {
    int style = 0,
    double gap = 5,
    bool lineArt = false,
    bool ayahMarks = true,
    bool surahHeaders = true,
    bool pageFrame = true,
    Map<String, Object>? colors,
  }) {
    if (style < 0 || style >= ornaments.styles.length) return null;
    final parts = ornaments.styles[style].parts;
    final pairs = <int>[];
    for (final e in (colors ?? const <String, Object>{}).entries) {
      final k = parts.indexWhere((p) => p.name == e.key);
      if (k >= 0) pairs.addAll([k, rgba(e.value)]);
    }
    final spec = pffi.calloc<QvpDressSpecC>();
    try {
      final ok = engine.withU32(pairs, (cp, n) {
        final r = spec.ref;
        r.style = style;
        r.gap = gap;
        r.lineArt = lineArt ? 1 : 0;
        r.ayahMarks = ayahMarks ? 1 : 0;
        r.surahHeaders = surahHeaders ? 1 : 0;
        r.pageFrame = pageFrame ? 1 : 0;
        r.colors = pairs.isEmpty ? ffi.nullptr : cp;
        r.nColors = pairs.length ~/ 2;
        return engine.b.dress(_p, ornaments._o, spec);
      });
      if (ok == 0) return null;
    } finally {
      pffi.calloc.free(spec);
    }
    _dress = null;
    _touch();
    return dressInfo();
  }

  /// Take the ornaments off: the printed rings come back and the viewBox
  /// returns to the page's own.
  void undress() {
    engine.b.undress(_p);
    _dress = null;
    _touch();
  }

  /// The readout, or null when the page is not dressed.
  QvpDress? dressInfo() {
    final out = pffi.calloc<QvpDressInfoC>();
    try {
      if (engine.b.dressInfo(_p, out) == 0) return null;
      final r = out.ref;
      return QvpDress(
        style: r.style,
        ayahMarks: r.nAyahMarks,
        surahHeaders: r.nSurahHeaders,
        frameRepeats: r.nFrameRepeats,
        frameStretched: r.frameStretched != 0,
        nDraws: r.nDraws,
        revision: r.revision,
        viewBox: (r.viewBox[0], r.viewBox[1], r.viewBox[2], r.viewBox[3]),
      );
    } finally {
      pffi.calloc.free(out);
    }
  }

  /// Bumped whenever the ornament layer is rebuilt — a new dress, or a layout
  /// that respaced the page under the border. 0 when undressed.
  int dressRevision() => dressInfo()?.revision ?? 0;

  /// The ornament display list, in page units. DRAW IT BEHIND THE PAGE INK:
  /// that is what keeps the print's own ayah numerals on top of whatever
  /// replaced the rings around them.
  QvpDressGeometry dressGeometry() {
    // A LAYOUT REBUILDS THIS LAYER: the border is drawn around the laid-out
    // page, so a cache kept only until the next dress() goes stale on a resize.
    final rev = dressRevision();
    final cached = _dress;
    if (cached != null && _dressRevision == rev) return cached;
    _dressRevision = rev;
    final g = pffi.calloc<QvpDressGeometryC>();
    try {
      engine.b.dressGeometry(_p, g);
      final r = g.ref;
      if (r.nDraws == 0) {
        return _dress = QvpDressGeometry(ops: Uint8List(0), pts: Float32List(0), draws: const []);
      }
      final table = r.table.asTypedList(r.nDraws * 9);
      final asFloat = Float32List.view(Uint32List.fromList(table).buffer);
      final draws = List<QvpOrnamentDraw>.generate(r.nDraws, (i) {
        final flags = table[i * 9 + 5];
        final line = table[i * 9 + 8];
        return QvpOrnamentDraw(
          opStart: table[i * 9],
          opCount: table[i * 9 + 1],
          ptStart: table[i * 9 + 2],
          ptCount: table[i * 9 + 3],
          color: table[i * 9 + 4],
          kind: flags & 0xff,
          evenOdd: flags & 0x100 != 0,
          stroke: flags & 0x200 != 0,
          strokeWidth: asFloat[i * 9 + 6],
          part: table[i * 9 + 7],
          line: line == 0xffffffff ? -1 : line,
        );
      }, growable: false);
      return _dress = QvpDressGeometry(
        ops: Uint8List.fromList(r.ops.asTypedList(r.opsLen)),
        pts: Float32List.fromList(r.pts.asTypedList(r.ptsLen)),
        draws: draws,
      );
    } finally {
      pffi.calloc.free(g);
    }
  }

  /// How much bigger the dressed page is than the laid-out content, on each
  /// side, in viewport px through the current layout: (left, top, right, bottom).
  /// Fit `content + overflow` or a border is cropped off; an undressed page
  /// answers zeroes.
  (double, double, double, double) dressOverflow() {
    final p = pffi.calloc<ffi.Float>(4);
    try {
      engine.b.dressOverflow(_p, p);
      final v = p.asTypedList(4);
      return (v[0], v[1], v[2], v[3]);
    } finally {
      pffi.calloc.free(p);
    }
  }

  /// The page's viewBox: (x, y, w, h). A dressed page's border grows it.
  (double, double, double, double) viewBox() {
    final p = pffi.calloc<ffi.Float>(4);
    try {
      engine.b.pageViewBox(_p, p);
      final v = p.asTypedList(4);
      return (v[0], v[1], v[2], v[3]);
    } finally {
      pffi.calloc.free(p);
    }
  }

  /// The box the page's text occupies: (x0, y0, x1, y1) in page units.
  (double, double, double, double) contentBox() {
    final p = pffi.calloc<ffi.Float>(4);
    try {
      engine.b.contentBox(_p, p);
      final v = p.asTypedList(4);
      return (v[0], v[1], v[2], v[3]);
    } finally {
      pffi.calloc.free(p);
    }
  }

  void free() {
    if (_h != ffi.nullptr) {
      engine.b.pageFree(_h);
      _h = ffi.nullptr;
    }
  }

  @override
  void dispose() {
    free();
    super.dispose();
  }

  // ── geometry table ──
  int pathOpStart(int i) => table[i * 8];
  int pathOpCount(int i) => table[i * 8 + 1];
  int pathPtStart(int i) => table[i * 8 + 2];
  int pathPtCount(int i) => table[i * 8 + 3];
  int pathFlags(int i) => table[i * 8 + 4];
  int pathKind(int i) => pathFlags(i) & 0xff;
  int pathMark(int i) => (pathFlags(i) >> 8) & 0xff;
  int pathFamily(int i) => (pathFlags(i) >> 16) & 0xff;
  bool pathEvenOdd(int i) => ((pathFlags(i) >> 24) & 1) == 1;
  bool pathGlyphInstance(int i) => ((pathFlags(i) >> 24) & 2) == 2;
  int pathWord(int i) {
    final w = table[i * 8 + 5];
    return w == qvpNone ? -1 : w;
  }

  int pathLine(int i) => table[i * 8 + 6];
  int pathCategory(int i) => table[i * 8 + 7] & 0xff;
  int pathNthInWord(int i) => (table[i * 8 + 7] >> 8) & 0xff;
  int pathNthMark(int i) {
    final n = (table[i * 8 + 7] >> 16) & 0xff;
    return n == 0xff ? -1 : n;
  }

  // ── info records ──
  QvpWordInfo _word(int i) {
    final o = pffi.calloc<QvpWordInfoC>();
    try {
      if (_b.wordInfo(_h, i, o) == 0) throw StateError('qvp_word_info($i) failed');
      final w = o.ref;
      return QvpWordInfo(
        idx: i,
        surah: w.surah,
        ayah: w.ayah,
        word: w.word,
        line: w.lineNo,
        ayahIdx: w.ayahIdx,
        lineIdx: w.lineIdx,
        x0: w.x0,
        y0: w.y0,
        x1: w.x1,
        y1: w.y1,
        text: QvpEngine.str(w.text),
        firstPath: w.firstPath,
        nPaths: w.nPaths,
      );
    } finally {
      pffi.calloc.free(o);
    }
  }

  QvpAyahInfo _ayah(int i) {
    final o = pffi.calloc<QvpAyahInfoC>();
    try {
      if (_b.ayahInfo(_h, i, o) == 0) throw StateError('qvp_ayah_info($i) failed');
      final a = o.ref;
      return QvpAyahInfo(
        idx: i,
        surah: a.surah,
        ayah: a.ayah,
        fragment: a.fragment,
        fragments: a.fragments,
        flags: a.flags,
        rubuAlHizb: a.rubuAlHizb,
        firstWord: a.firstWord,
        nWords: a.nWords,
        ayahMarkDeco: a.ayahMarkDeco,
        x0: a.x0,
        y0: a.y0,
        x1: a.x1,
        y1: a.y1,
      );
    } finally {
      pffi.calloc.free(o);
    }
  }

  QvpLineInfo _line(int i) {
    final o = pffi.calloc<QvpLineInfoC>();
    try {
      if (_b.lineInfo(_h, i, o) == 0) throw StateError('qvp_line_info($i) failed');
      final l = o.ref;
      return QvpLineInfo(
        idx: i,
        lineNo: l.lineNo,
        isHeader: l.isHeader != 0,
        firstWord: l.firstWord,
        nWords: l.nWords,
        x0: l.x0,
        y0: l.y0,
        x1: l.x1,
        y1: l.y1,
        bandY0: l.bandY0,
        bandY1: l.bandY1,
        centre: l.centre,
      );
    } finally {
      pffi.calloc.free(o);
    }
  }

  QvpDecoInfo _deco(int i) {
    final o = pffi.calloc<QvpDecoInfoC>();
    try {
      if (_b.decoInfo(_h, i, o) == 0) throw StateError('qvp_deco_info($i) failed');
      final d = o.ref;
      return QvpDecoInfo(
        idx: i,
        kind: d.kind,
        surah: d.surah,
        ayah: d.ayah,
        line: d.line,
        x0: d.x0,
        y0: d.y0,
        x1: d.x1,
        y1: d.y1,
        text: QvpEngine.str(d.text),
        firstPath: d.firstPath,
        nPaths: d.nPaths,
      );
    } finally {
      pffi.calloc.free(o);
    }
  }

  /// Text of word [i] in a form ('rasmUthmani' | 'rasmImlai' | 'qpc' | 'rasm' | 'search' or [QvpForm]); '' when the form is not attached.
  String wordForm(int i, [Object form = 'rasmUthmani']) {
    if (_b.wordForm(_p, i, QvpForm.of(form), _e._str) == 0) return '';
    return _e._s();
  }

  /// Word index for (surah, ayah, word) or -1.
  int findWord(int surah, int ayah, int word) {
    final i = _b.findWord(_p, surah, ayah, word);
    return i < 0 ? -1 : i;
  }

  /// `surah:ayah:word` of word [i].
  String wordKey(int i) => words[i].wordKey;

  /// Marshals a target (String / QvpTarget / `List<int>`) and runs [f] with the pointer.
  R _t<R>(Object target, R Function(ffi.Pointer<QvpTargetC> t) f) {
    var t = QvpTarget.of(target);
    if (t.wordKey != null) {
      final i = findWord(t.wordKey![0], t.wordKey![1], t.wordKey![2]);
      t = i >= 0 ? T.word(i) : T.words(const []);
    }
    final r = _e._target.ref;
    r.kind = t.kind;
    r.a = t.a;
    r.b = t.b;
    r.c = t.c;
    r.nWords = 0;
    r.words = ffi.nullptr;
    if (t.words != null) {
      return _e.withU32(t.words!, (p, n) {
        r.words = p;
        r.nWords = n;
        return f(_e._target);
      });
    }
    return f(_e._target);
  }

  List<int> _u32(int n) => List<int>.from(_e._out<ffi.Uint32>().asTypedList(n.clamp(0, _e._cap(ffi.sizeOf<ffi.Uint32>()))), growable: false);

  /// Word indices of a target.
  List<int> resolve(Object target) => _t(target, (t) => _u32(_b.resolve(_p, t, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>()))));

  // ── metadata ──
  List<QvpSurahInfo> surahs() {
    final n = _b.surahsCount(_p);
    final o = pffi.calloc<QvpSurahInfoC>();
    try {
      return List.generate(n, (i) {
        _b.surahAt(_p, i, o);
        final s = o.ref;
        return QvpSurahInfo(
          number: s.number,
          ayahCount: s.ayahCount,
          hasBanner: s.hasBanner != 0,
          hasBasmalah: s.hasBasmalah != 0,
          place: const ['makkah', 'madinah'].elementAtOrNull(s.place) ?? '',
          bannerDeco: s.bannerDeco,
          arabic: QvpEngine.str(s.arabic),
          latin: QvpEngine.str(s.latin),
          english: QvpEngine.str(s.english),
        );
      }, growable: false);
    } finally {
      pffi.calloc.free(o);
    }
  }

  /// Divisions (juz / hizb / nisf / rubuAlHizb) that start on this page.
  List<QvpDivision> divisions() {
    final o = _e._out<QvpDivisionC>(), cap = _e._cap(ffi.sizeOf<QvpDivisionC>());
    final n = _b.divisions(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final d = (o + i).ref;
      return QvpDivision(kind: QvpDiv.names[d.kind.clamp(0, 3)], line: d.line, n: d.n, surah: d.surah, ayah: d.ayah, ayahIdx: d.ayahIdx);
    }, growable: false);
  }

  /// Real ayah medallions.
  List<QvpAyahMark> ayahMarks() {
    final o = _e._out<QvpAyahMarkC>(), cap = _e._cap(ffi.sizeOf<QvpAyahMarkC>());
    final n = _b.ayahMarks(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final m = (o + i).ref;
      return QvpAyahMark(deco: m.deco, surah: m.surah, ayah: m.ayah, line: m.line, cx: m.cx, cy: m.cy, r: m.r, ornamentPath: m.ornamentPath, numeralPath: m.numeralPath);
    }, growable: false);
  }

  /// Drawn hizb rosettes.
  List<QvpRosette> rosettes() {
    final o = _e._out<QvpRosetteC>(), cap = _e._cap(ffi.sizeOf<QvpRosetteC>());
    final n = _b.rosettes(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final r = (o + i).ref;
      return QvpRosette(deco: r.deco, surah: r.surah, ayah: r.ayah, juz: r.juz, hizb: r.hizb, nisf: r.nisf, rubuAlHizb: r.rubuAlHizb, rubuAlHizbInHizb: r.rubuAlHizbInHizb);
    }, growable: false);
  }

  List<QvpSajdah> sajdahs() {
    final o = _e._out<QvpSajdahC>(), cap = _e._cap(ffi.sizeOf<QvpSajdahC>());
    final n = _b.sajdahs(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final s = (o + i).ref;
      return QvpSajdah(deco: s.deco, surah: s.surah, ayah: s.ayah, signPath: s.signPath);
    }, growable: false);
  }

  /// `(surah, ayah)` keys in reading order.
  List<(int, int)> ayahKeys() => _u32(_b.ayahKeys(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>()))).map((k) => (k >> 16, k & 0xffff)).toList(growable: false);

  /// Words of an ayah on this page and whether the ayah is complete here.
  ({int count, bool complete}) ayahWordCount(int surah, int ayah) {
    final o = _e._out<ffi.Uint32>();
    final n = _b.ayahWordCount(_p, surah, ayah, o);
    return (count: n, complete: o.value != 0);
  }

  /// Words for [nSegments] recitation segments, or null when the counts disagree (follow the ayah whole).
  List<int>? reciteMap(int surah, int ayah, int nSegments) {
    final n = _b.reciteMap(_p, surah, ayah, nSegments, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>()));
    return n < 0 ? null : _u32(n);
  }

  /// Accessibility label of word [i].
  String wordLabel(int i) {
    _b.wordLabel(_p, i, _e._str);
    return _e._s();
  }

  /// Accessibility label of ayah record [ai].
  String ayahLabel(int ai) {
    _b.ayahLabel(_p, ai, _e._str);
    return _e._s();
  }

  // ── text & search ──
  /// Text of a target (default: the page).
  String text([Object target = 'page', Object form = 'rasmUthmani', String wordSep = ' ', String lineSep = '\n']) => _e.withString(wordSep, (wp, wn) => _e.withString(lineSep, (lp, ln) => _t(target, (t) {
        _b.textTarget(_p, t, QvpForm.of(form), wp, wn, lp, ln, _e._str);
        return _e._s();
      })));

  /// Search this page. [mode]: 'includes' | 'exact' | 'prefix'.
  List<QvpMatch> search(String query, {Object form = 'search', String mode = 'includes', bool normalize = true, bool loose = true, int limit = 0}) {
    final o = _e._out<QvpMatchC>(), cap = _e._cap(ffi.sizeOf<QvpMatchC>());
    final n = _e.withString(query, (p, len) => _b.search(_p, p, len, QvpForm.of(form), const {'includes': 0, 'exact': 1, 'prefix': 2}[mode] ?? 0, normalize ? 1 : 0, loose ? 1 : 0, limit, o, cap)).clamp(0, cap);
    return List.generate(n, (i) {
      final m = (o + i).ref;
      return QvpMatch(word: m.word, index: m.index, loose: m.loose != 0, wordKey: wordKey(m.word), text: words[m.word].text);
    }, growable: false);
  }

  /// "2:255-257, 3:1" for a word list.
  String citation(List<int> ws) => _e.withU32(ws, (p, n) {
        _b.citation(_p, p, n, _e._str);
        return _e._s();
      });

  /// Attaches a words sidecar (`NNN.words.json` as String, bytes or decoded map); returns words updated, -1 on bad JSON.
  int attachWords(Object sidecar) {
    final List<int> bytes = switch (sidecar) {
      String s => utf8.encode(s),
      Uint8List b => b,
      List<int> b => b,
      _ => utf8.encode(jsonEncode(sidecar)),
    };
    final n = _e.withBytes(bytes, (p, len) => _b.attachWords(_p, p, len));
    if (n > 0) _touch();
    return n;
  }

  bool hasForm(Object form) => _b.hasForm(_p, QvpForm.of(form)) != 0;

  // ── hit testing ──
  static int _idx(int v) => v == qvpNone ? -1 : v;

  QvpHit? _readHit(int ok) {
    if (ok == 0) return null;
    final h = _e._hit.ref;
    return QvpHit(word: _idx(h.word), path: _idx(h.path), deco: _idx(h.deco));
  }

  /// Exact outline hit, page units.
  QvpHit? hitTest(double x, double y) => _readHit(_b.hitTest(_p, x, y, _e._hit));

  /// Exact outline hit, viewport px through the current layout.
  QvpHit? hitTestView(double vx, double vy) => _readHit(_b.hitTestView(_p, vx, vy, _e._hit));

  ffi.Pointer<QvpHitOptionsC> _opt(QvpHitOptions o) {
    final r = _e._hitOpt.ref;
    r.maxDistance = o.maxDistance;
    r.gapBias = o.gapBias;
    r.exactFirst = o.exactFirst ? 1 : 0;
    return _e._hitOpt;
  }

  QvpHitEx? _readHitEx(int ok) {
    if (ok == 0) return null;
    final h = _e._hitEx.ref;
    final w = _idx(h.word);
    return QvpHitEx(
      word: w,
      path: _idx(h.path),
      deco: _idx(h.deco),
      line: h.line,
      distance: h.distance,
      exact: h.exact != 0,
      wordKey: w >= 0 ? words[w].wordKey : null,
      ayahKey: w >= 0 ? words[w].ayahKey : null,
    );
  }

  /// Gap-aware: every point on a printed line resolves to the word the user meant. Page units.
  QvpHitEx? hitTestEx(double x, double y, [QvpHitOptions opt = const QvpHitOptions()]) => _readHitEx(_b.hitTestEx(_p, x, y, _opt(opt), _e._hitEx));

  /// Gap-aware, viewport px through the current layout.
  QvpHitEx? hitTestViewEx(double vx, double vy, [QvpHitOptions opt = const QvpHitOptions()]) => _readHitEx(_b.hitTestViewEx(_p, vx, vy, _opt(opt), _e._hitEx));

  List<QvpLineBand> lineBands() {
    final o = _e._out<QvpLineBandC>(), cap = _e._cap(ffi.sizeOf<QvpLineBandC>());
    final n = _b.lineBands(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final l = (o + i).ref;
      return QvpLineBand(line: l.line, lineNo: l.lineNo, y0: l.y0, y1: l.y1, mid: l.mid, inkY0: l.inkY0, inkY1: l.inkY1);
    }, growable: false);
  }

  List<QvpHitBox> hitBoxes([double gapBias = 0.6]) {
    final o = _e._out<QvpHitBoxC>(), cap = _e._cap(ffi.sizeOf<QvpHitBoxC>());
    final n = _b.hitBoxes(_p, gapBias, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final h = (o + i).ref;
      return QvpHitBox(word: h.word, line: h.line, x0: h.x0, y0: h.y0, x1: h.x1, y1: h.y1, inkX0: h.inkX0, inkY0: h.inkY0, inkX1: h.inkX1, inkY1: h.inkY1);
    }, growable: false);
  }

  // ── layout ──
  /// Computes and stores an engine layout (also used by the *View hit tests and box outputs).
  QvpLayout layout(QvpLayoutSpec spec) {
    final s = _e._spec.ref;
    s.viewportW = spec.viewportW;
    s.viewportH = spec.viewportH;
    s.padTop = spec.padTop;
    s.padBottom = spec.padBottom;
    s.padLeft = spec.padLeft;
    s.padRight = spec.padRight;
    s.lineSpacing = spec.lineSpacing;
    s.lineGap = spec.lineGap;
    s.fillHeight = spec.fillHeight ? 1 : 0;
    s.nominalLines = spec.nominalLines;
    _b.layout(_p, _e._spec, _e._layout);
    final o = _e._layout.ref;
    final n = o.nLines;
    final f = o.lines == ffi.nullptr ? Float32List(0) : o.lines.asTypedList(n * 3);
    final lineDy = Float32List(n);
    final slots = List<(double, double)>.generate(n, (i) {
      lineDy[i] = f[i * 3];
      return (f[i * 3 + 1], f[i * 3 + 2]);
    }, growable: false);
    final l = QvpLayout(scale: o.scale, ox: o.ox, oy: o.oy, contentW: o.contentW, contentH: o.contentH, pitch: o.pitch, lineDy: lineDy, slots: slots);
    currentLayout = l;
    _touch();
    return l;
  }

  /// Word bbox in viewport px through the current layout.
  ({double x0, double y0, double x1, double y1}) wordBoxView(int i) {
    final o = _e._out<ffi.Float>();
    _b.wordBoxView(_p, i, o);
    final f = o.asTypedList(4);
    return (x0: f[0], y0: f[1], x1: f[2], y1: f[3]);
  }

  // ── styles (handles undo exactly) ──
  /// Adds a style rule → handle (0 = bad selector).
  int style(QvpSelector sel, Object color, {int ms = 0, int layer = QvpLayer.base}) {
    final h = _b.styleAdd(_p, layer, _e._writeSel(sel), rgba(color), ms);
    _touch();
    return h;
  }

  int styleTarget(Object target, Object color, {int ms = 0, int layer = QvpLayer.base}) {
    final h = _t(target, (t) => _b.styleAddTarget(_p, layer, t, rgba(color), ms));
    _touch();
    return h;
  }

  /// Removes a rule / theme / hide by handle → rules removed.
  int unstyle(int handle) {
    final n = _b.styleRemove(_p, handle);
    _touch();
    return n;
  }

  int restyle(int handle, Object color, [int ms = 0]) {
    final n = _b.styleRepaint(_p, handle, rgba(color), ms);
    _touch();
    return n;
  }

  /// Alpha-0 rule on the top layer → handle.
  int hide(QvpSelector sel) {
    final h = _b.hide(_p, _e._writeSel(sel));
    _touch();
    return h;
  }

  void clearStyles() {
    _b.styleClear(_p);
    _touch();
  }

  void clearLayer(int layer) {
    _b.styleClearLayer(_p, layer);
    _touch();
  }

  void setDefaultInk(Object color) {
    _defaultInk = rgba(color);
    _b.styleDefault(_p, _defaultInk);
    _touch();
  }

  /// Applies a whole theme under one handle.
  int theme(QvpTheme t) {
    final r = _e._theme.ref;
    int c(Object? v) => v == null ? 0 : rgba(v);
    r.ink = c(t.ink);
    r.diacritics = c(t.diacritics);
    r.dots = c(t.dots);
    r.waqf = c(t.waqf);
    r.sifr = c(t.sifr);
    r.ayahMark = c(t.ayahMark);
    r.numeral = c(t.numeral);
    r.headers = c(t.headers);
    r.transitionMs = t.ms;
    final pairs = <int>[];
    t.marks.forEach((m, col) => pairs..add(QvpMark.id(m))..add(rgba(col)));
    final h = pairs.isEmpty
        ? (() {
            r.marks = ffi.nullptr;
            r.nMarks = 0;
            return _b.theme(_p, _e._theme);
          })()
        : _e.withU32(pairs, (p, n) {
            r.marks = p;
            r.nMarks = n ~/ 2;
            return _b.theme(_p, _e._theme);
          });
    _touch();
    return h;
  }

  List<int> styleHandles() => _u32(_b.styleHandles(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));

  // ── clock & display list ──
  /// Advances animations; true while something is still moving (keep drawing frames).
  bool tick(double nowMs) => _b.tick(_p, nowMs) != 0;

  /// Full display list: one 0xRRGGBBAA per path (copy).
  Uint32List paint() => Uint32List.fromList(_b.paint(_p).asTypedList(nPaths));

  /// Paths whose colour differs from the default ink (mid-transition values included).
  List<QvpStyledPath> styled() {
    final n = _b.styled(_p, ffi.nullptr, 0);
    if (n == 0) return const [];
    final buf = pffi.malloc<ffi.Uint32>(n * 2);
    try {
      _b.styled(_p, buf, n);
      final v = buf.asTypedList(n * 2);
      return List.generate(n, (i) => (path: v[i * 2], color: v[i * 2 + 1]), growable: false);
    } finally {
      pffi.malloc.free(buf);
    }
  }

  int colorOf(int path) => _b.colorOf(_p, path);

  // ── highlights ──
  List<QvpBox> _boxes(int n) {
    final o = _e._out<QvpBoxC>();
    n = n.clamp(0, _e._cap(ffi.sizeOf<QvpBoxC>()));
    return List.generate(n, (i) {
      final b = (o + i).ref;
      return QvpBox(id: b.id, line: b.line, x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, color: b.color, radius: b.radius);
    }, growable: false);
  }

  /// Highlights a target → handle.
  int highlight(Object target, [QvpHighlightStyle style = const QvpHighlightStyle()]) {
    final h = _t(target, (t) => _b.highlight(_p, t, _e._writeHl(style)));
    _touch();
    return h;
  }

  /// Moves a highlight: the band slides, the ink fades.
  bool rehighlight(int handle, Object target) {
    final ok = _t(target, (t) => _b.rehighlight(_p, handle, t)) != 0;
    _touch();
    return ok;
  }

  bool restyleHighlight(int handle, QvpHighlightStyle style) {
    final ok = _b.restyleHighlight(_p, handle, _e._writeHl(style)) != 0;
    _touch();
    return ok;
  }

  /// Fades out over the highlight's transition.
  bool unhighlight(int handle) {
    final ok = _b.unhighlight(_p, handle) != 0;
    _touch();
    return ok;
  }

  void clearHighlights() {
    _b.clearHighlights(_p);
    _touch();
  }

  List<int> highlightHandles() => _u32(_b.highlightHandles(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));
  List<int> highlightWords(int handle) => _u32(_b.highlightWords(_p, handle, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));

  /// Animated band boxes in layout viewport px; draw each id as one nonzero path behind the ink.
  List<QvpBox> highlightBoxes() => _boxes(_b.highlightBoxes(_p, _e._out<QvpBoxC>(), _e._cap(ffi.sizeOf<QvpBoxC>())));

  /// Static band boxes for a word list (no highlight state involved).
  List<QvpBox> bandBoxes(List<int> ws, {String height = 'pitch', double padX = 1.2, double padY = 0}) => _e.withU32(ws, (p, n) => _boxes(_b.bandBoxes(_p, p, n, height == 'ink' ? 1 : 0, padX, padY, _e._out<QvpBoxC>(), _e._cap(ffi.sizeOf<QvpBoxC>()))));

  // ── selection ──
  void select(int anchor, [int? focus]) {
    _b.select(_p, anchor < 0 ? qvpNone : anchor, (focus ?? anchor) < 0 ? qvpNone : (focus ?? anchor));
    _touch();
  }

  void clearSelection() => select(-1, -1);
  List<int> selection() => _u32(_b.selection(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));
  String selectionText([Object form = 'rasmUthmani', bool citation = false]) {
    _b.selectionText(_p, QvpForm.of(form), citation ? 1 : 0, _e._str);
    return _e._s();
  }

  // ── memorisation ──
  static int _maskMode(String m) => const {'hide': 0, 'block': 1, 'blur': 2}[m] ?? 0;

  /// Masks a target. [mode]: 'hide' | 'block' | 'blur'.
  void mask(Object target, [String mode = 'hide']) {
    _t(target, (t) => _b.mask(_p, t, _maskMode(mode)));
    _touch();
  }

  void maskFrom(int wi, [String mode = 'hide']) {
    _b.maskFrom(_p, wi, _maskMode(mode));
    _touch();
  }

  void maskOptions({Object blockColor = '#d9d4c8', double padX = 0.6, double padY = 0.6, double radius = 0.8, bool reverse = false}) {
    _b.maskOptions(_p, rgba(blockColor), padX, padY, radius, reverse ? 1 : 0);
    _touch();
  }

  int revealNext([int n = 1]) {
    final r = _b.revealNext(_p, n);
    _touch();
    return r;
  }

  int hideBack([int n = 1]) {
    final r = _b.hideBack(_p, n);
    _touch();
    return r;
  }

  bool revealWord(int wi) {
    final r = _b.revealWord(_p, wi) != 0;
    _touch();
    return r;
  }

  bool hideWord(int wi) {
    final r = _b.hideWord(_p, wi) != 0;
    _touch();
    return r;
  }

  void revealAll() {
    _b.revealAll(_p);
    _touch();
  }

  void hideAll() {
    _b.hideAll(_p);
    _touch();
  }

  void unmask() {
    _b.unmask(_p);
    _touch();
  }

  List<int> maskHidden() => _u32(_b.maskHidden(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));
  List<int> maskWords() => _u32(_b.maskWords(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));

  /// Block / blur boxes in layout viewport px (drawn last).
  List<QvpBox> maskBoxes() => _boxes(_b.maskBoxes(_p, _e._out<QvpBoxC>(), _e._cap(ffi.sizeOf<QvpBoxC>())));

  /// Greyed page with a lit window → steps.
  int revealStart({int lit = 1, bool byAyah = false, Object grey = '#c9c4b8', Object ink = '#231f20', bool ayahMarks = true, int ms = 0}) {
    final n = _b.revealStart(_p, lit, byAyah ? 1 : 0, rgba(grey), rgba(ink), ayahMarks ? 1 : 0, ms);
    _touch();
    return n;
  }

  /// Lights the window at step [at] (-1 = nothing lit yet).
  bool revealGoto(int at) {
    final ok = _b.revealGoto(_p, at) != 0;
    _touch();
    return ok;
  }

  /// Current step, or null when no reveal is running.
  int? revealAt() {
    final v = _b.revealAt(_p);
    return v == -2 ? null : v;
  }

  int revealSteps() => _b.revealSteps(_p);
  int revealStepOf(int wi) => _b.revealStepOf(_p, wi);
  void revealStop() {
    _b.revealStop(_p);
    _touch();
  }

  // ── crop ──
  QvpCropBox? cropBox(Object target, {double pad = 2, bool keepAyahMarks = true}) {
    final ok = _t(target, (t) => _b.cropBox(_p, t, pad, keepAyahMarks ? 1 : 0, _e._crop));
    if (ok == 0) return null;
    final c = _e._crop.ref;
    return QvpCropBox(x0: c.x0, y0: c.y0, x1: c.x1, y1: c.y1, nWords: c.nWords, ayahMarkDeco: c.ayahMarkDeco);
  }

  /// Standalone SVG of a target (current colours), or null.
  String? cropSvg(Object target, {double pad = 2, bool keepAyahMarks = true, Object? background}) {
    final ok = _t(target, (t) => _b.cropSvg(_p, t, pad, keepAyahMarks ? 1 : 0, background == null ? 0 : rgba(background), _e._str));
    return ok == 0 ? null : _e._s();
  }
}

// ───────────── atlas ─────────────

/// Cross-page lookup (`atlas.qva`).
class QvpAtlas {
  QvpAtlas._(this.engine, this._h);
  final QvpEngine engine;
  ffi.Pointer<QvpAtlasC> _h;
  QvpBindings get _b => engine.b;

  ffi.Pointer<QvpAtlasC> get _a {
    assert(_h != ffi.nullptr, 'QvpAtlas used after dispose()');
    return _h;
  }

  void free() {
    if (_h != ffi.nullptr) {
      _b.atlasFree(_h);
      _h = ffi.nullptr;
    }
  }

  void dispose() => free();

  int get pages => _b.atlasPages(_a);

  /// Page of an ayah, or null.
  int? pageOf(int surah, int ayah) {
    final p = _b.atlasPageOf(_a, surah, ayah);
    return p < 0 ? null : p;
  }

  /// First and last ayah `(surah, ayah)` of a page, or null.
  ({(int, int) first, (int, int) last})? pageRange(int page) {
    final o = engine._out<ffi.Uint16>();
    if (_b.atlasPageRange(_a, page, o) == 0) return null;
    final v = o.asTypedList(4);
    return (first: (v[0], v[1]), last: (v[2], v[3]));
  }

  QvpAtlasSurah _surah(QvpAtlasSurahC s) => QvpAtlasSurah(
        n: s.n,
        page: s.firstPage,
        ayahCount: s.ayahCount,
        place: const ['makkah', 'madinah'].elementAtOrNull(s.place) ?? '',
        arabic: QvpEngine.str(s.arabic),
        latin: QvpEngine.str(s.latin),
        english: QvpEngine.str(s.english),
      );

  QvpAtlasSurah? surah(int n) {
    final o = pffi.calloc<QvpAtlasSurahC>();
    try {
      return _b.atlasSurah(_a, n, o) == 0 ? null : _surah(o.ref);
    } finally {
      pffi.calloc.free(o);
    }
  }

  List<QvpAtlasSurah> surahs() {
    final o = pffi.calloc<QvpAtlasSurahC>();
    try {
      return List.generate(_b.atlasSurahs(_a), (i) {
        _b.atlasSurahAt(_a, i, o);
        return _surah(o.ref);
      }, growable: false);
    } finally {
      pffi.calloc.free(o);
    }
  }

  int? pageOfSurah(int n) => surah(n)?.page;

  /// Start of division [n] of [kind] ('juz' | 'hizb' | 'nisf' | 'rubuAlHizb' or [QvpDiv]).
  QvpAtlasRubuAlHizb? division(Object kind, int n) {
    final o = pffi.calloc<QvpAtlasRubuAlHizbC>();
    try {
      if (_b.atlasDivision(_a, QvpDiv.of(kind), n, o) == 0) return null;
      final r = o.ref;
      return QvpAtlasRubuAlHizb(rubuAlHizb: r.rubuAlHizb, surah: r.surah, ayah: r.ayah, page: r.page);
    } finally {
      pffi.calloc.free(o);
    }
  }

  QvpAtlasRubuAlHizb? juz(int n) => division('juz', n);
  QvpAtlasRubuAlHizb? hizb(int n) => division('hizb', n);
  QvpAtlasRubuAlHizb? rubuAlHizb(int n) => division('rubuAlHizb', n);

  /// Division number containing an ayah, or null.
  int? divisionAt(Object kind, int surah, int ayah) {
    final v = _b.atlasDivisionAt(_a, QvpDiv.of(kind), surah, ayah);
    return v < 0 ? null : v;
  }

  int? juzAt(int surah, int ayah) => divisionAt('juz', surah, ayah);

  /// `[first, last]` page of a juz, or null.
  List<int>? pagesOfJuz(int n) {
    final o = engine._out<ffi.Uint16>();
    if (_b.atlasPagesOfJuz(_a, n, o) == 0) return null;
    final v = o.asTypedList(2);
    return [v[0], v[1]];
  }

  /// Surahs matching a (partial, any-script) name.
  List<QvpAtlasSurah> findSurah(String text) {
    final o = engine._out<ffi.Uint16>(), cap = engine._cap(ffi.sizeOf<ffi.Uint16>());
    final n = engine.withString(text, (p, len) => _b.atlasFindSurah(_a, p, len, o, cap)).clamp(0, cap);
    final ids = List<int>.from(o.asTypedList(n));
    return [for (final k in ids) surah(k)].whereType<QvpAtlasSurah>().toList(growable: false);
  }

  String json() {
    _b.atlasJson(_a, engine._str);
    return engine._s();
  }
}

// ───────────── ornaments ─────────────

/// The ornaments of other printed mushafs, from `ornaments.qvo`.
///
/// NONE OF THESE OUTLINES IS PART OF A QVP PAGE. They are traced from scans of
/// other prints, and each style says what it may be redistributed under — read
/// [QvpOrnamentStyle.licence] before you publish a page wearing them.
class QvpOrnaments {
  QvpOrnaments._(this.engine, this._h) {
    final st = pffi.calloc<QvpOrnamentStyleC>();
    final pt = pffi.calloc<QvpOrnamentPartC>();
    try {
      final n = engine.b.ornamentStyles(_h);
      styles = List<QvpOrnamentStyle>.generate(n, (i) {
        engine.b.ornamentStyle(_h, i, st);
        final r = st.ref;
        final nParts = r.nParts;
        return QvpOrnamentStyle(
          index: i,
          name: QvpEngine.str(r.name),
          riwayah: QvpEngine.str(r.riwayah),
          hasAyahMark: r.assets & 1 != 0,
          hasSurahHeader: r.assets & 2 != 0,
          hasPageFrame: r.assets & 4 != 0,
          tiles: r.assets & 8 != 0,
          licence: QvpOrnamentLicence(
            id: QvpEngine.str(r.licenseId),
            status: QvpEngine.str(r.licenseStatus),
            redistributable: r.redistributable != 0,
            attribution: QvpEngine.str(r.attribution),
          ),
          parts: List<QvpOrnamentPart>.generate(nParts, (k) {
            engine.b.ornamentPart(_h, i, k, pt);
            final p = pt.ref;
            return QvpOrnamentPart(index: k, name: QvpEngine.str(p.name), color: p.color, stroke: p.stroke != 0);
          }, growable: false),
        );
      }, growable: false);
    } finally {
      pffi.calloc.free(st);
      pffi.calloc.free(pt);
    }
  }

  final QvpEngine engine;
  ffi.Pointer<QvpOrnamentsC> _h;
  late final List<QvpOrnamentStyle> styles;

  ffi.Pointer<QvpOrnamentsC> get _o {
    assert(_h != ffi.nullptr, 'QvpOrnaments used after dispose()');
    return _h;
  }

  QvpOrnamentStyle? find(String name) {
    final i = engine.withString(name, (p, n) => engine.b.ornamentFindStyle(_o, p, n));
    return i < 0 ? null : styles[i];
  }

  void free() {
    if (_h != ffi.nullptr) {
      engine.b.ornamentsFree(_h);
      _h = ffi.nullptr;
    }
  }

  void dispose() => free();
}
