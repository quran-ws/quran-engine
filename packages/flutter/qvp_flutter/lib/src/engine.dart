// QvpEngine / QvpPage / QvpAtlas / QvpOrnaments — the Dart mirror of web/qvp.js over the C
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
import 'dart:ui' show Color, Path, PathFillType, Rect;

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

/// `QVP_DECORATION_*`
abstract final class QvpDecorationKind {
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

/// The defaults every wrapper shares (`QVP_DEFAULT_*` in qvp.h; the parity check compares them).
/// Colours are 0xRRGGBBAA, lengths page units.
abstract final class QvpDefaults {
  static const int ink = 0x231f20ff, highlightInk = 0x1a73e8ff, highlightBand = 0xd6a3264d, selectionBand = 0x2d6fd640, maskBlock = 0xd9d4c8ff, revealGrey = 0xc9c4b8ff;
  static const double highlightPadX = 1.2, highlightPadY = 0, highlightSeam = 0.25, gapBias = 0.6, tapDistance = 6, aspectSlack = 1.15, maskPad = 0.6, maskRadius = 0.8, cropPad = 2, ayahMarkScaleScan = 1.5;
  static const int gridLines = 15, revealLit = 1;
  /// The reader's pinch and swipe: the engine's own limits, which every platform holds to.
  static const double minZoom = 0.5, maxZoom = 12, zoomedThreshold = 1.02;
  static const double swipeAxisRatio = 1.5, swipeDistance = 40, swipeVelocity = 500;
  static const double zoomSnapHysteresis = 0.03, zoomQuantum = 0.01;
  /// How far a justified row's gaps may open, as a multiple of the usual air between two
  /// words: what a reflowed page and a passage stretch to unless the host says otherwise.
  static const double reflowMaxStretch = 2;
}

/// The engine's name tables (`QVP_NAMES_*`), loaded from the engine when a [QvpEngine] opens.
/// No table lives in this package.
abstract final class QvpNames {
  static const int mark = 0, kind = 1, family = 2, category = 3, decoration = 4, division = 5, place = 6;
  static final Map<int, List<String>> _tables = {};

  /// Every name of a table, index = id. Empty until a [QvpEngine] has opened.
  static List<String> of(int table) => _tables[table] ?? const [];

  /// A name's id in a table; 255 when the table has no such name.
  static int idOf(int table, Object v) {
    if (v is int) return v;
    final i = of(table).indexOf(v as String);
    return i < 0 ? 255 : i;
  }
}

/// `QVP_DIVISION_*`
abstract final class QvpDiv {
  static const int juz = 0, hizb = 1, nisf = 2, rubuAlHizb = 3;
  static List<String> get names => QvpNames.of(QvpNames.division);
  static int of(Object k) => k is int ? k : QvpNames.idOf(QvpNames.division, k == 'rubuAlHizb' ? 'rubu_al_hizb' : k);
}

/// Mark ids ↔ names, from the engine. Index = id; 255 = unknown.
abstract final class QvpMark {
  static List<String> get names => QvpNames.of(QvpNames.mark);

  /// Mark id from a name or an id; unknown names → 255.
  static int id(Object m) {
    if (m is int) return m;
    final i = QvpNames.idOf(QvpNames.mark, m);
    return i == 0 ? 255 : i;
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

/// What a style rule applies to (`QvpSelector`, `QVP_SELECTOR_*`). Build with [Sel].
@immutable
final class QvpSelector {
  const QvpSelector(this.selector, [this.a = 0, this.b = 0, this.c = 0]);
  final int selector, a, b, c;
  @override
  String toString() => 'Sel($selector,$a,$b,$c)';
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
  static QvpSelector decoration(int k) => QvpSelector(14, k);
  static QvpSelector decorationIndex(int i) => QvpSelector(15, i);
}

/// What resolves to a word list (`QvpTarget`, `QVP_TARGET_*`). Build with [T]
/// or pass a string anywhere a target is accepted: 'page', '2:255', '2:255:3',
/// '2:255-257', 'line:7', 'surah:2'. A `List<int>` of word indices is a target too.
@immutable
final class QvpTarget {
  const QvpTarget(this.target, {this.a = 0, this.b = 0, this.c = 0, this.words, this.wordKey});
  final int target, a, b, c;
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
    required this.index,
    required this.surah,
    required this.ayah,
    required this.word,
    required this.line,
    required this.ayahIndex,
    required this.lineIndex,
    required this.x0,
    required this.y0,
    required this.x1,
    required this.y1,
    required this.text,
    required this.firstPath,
    required this.nPaths,
  });
  final int index, surah, ayah, word, line, ayahIndex, lineIndex, firstPath, nPaths;
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
    required this.index,
    required this.surah,
    required this.ayah,
    required this.fragment,
    required this.fragments,
    required this.flags,
    required this.rubuAlHizb,
    required this.firstWord,
    required this.nWords,
    required this.ayahMarkDecoration,
    required this.x0,
    required this.y0,
    required this.x1,
    required this.y1,
  });
  final int index, surah, ayah, fragment, fragments, flags, rubuAlHizb, firstWord, nWords;

  /// Deco index of the ayah mark or [qvpNone].
  final int ayahMarkDecoration;
  final double x0, y0, x1, y1;
}

@immutable
final class QvpLineInfo {
  const QvpLineInfo({
    required this.index,
    required this.lineNumber,
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
  final int index, lineNumber, firstWord, nWords;
  final bool isHeader;
  final double x0, y0, x1, y1, bandY0, bandY1, centre;
}

@immutable
final class QvpDecorationInfo {
  const QvpDecorationInfo({
    required this.index,
    required this.decoration,
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
  final int index, decoration, surah, ayah, line, firstPath, nPaths;
  final double x0, y0, x1, y1;
  final String text;
}

/// The one hit shape for every hit test (`QvpHit`); the exact variants report distance 0 and
/// `isExact`. Indices are -1 when absent.
@immutable
final class QvpHit {
  const QvpHit({required this.word, required this.path, required this.decoration, required this.line, required this.distance, required this.isExact, this.wordKey, this.ayahKey});
  final int word, path, decoration, line;
  final double distance;
  final bool isExact;
  final String? wordKey, ayahKey;
  @override
  String toString() => 'QvpHit(word: $word, path: $path, decoration: $decoration, line: $line, distance: $distance, isExact: $isExact)';
}

/// Options of the gap-aware hit test.
@immutable
final class QvpHitOptions {
  const QvpHitOptions({this.maxDistance = 0, this.gapBias = QvpDefaults.gapBias, this.preferExact = true});

  /// Page units; <= 0 unlimited.
  final double maxDistance;
  final double gapBias;
  final bool preferExact;
}

/// A band / mask box in layout viewport px (`QvpBox`).
@immutable
final class QvpBox {
  const QvpBox({required this.id, required this.line, required this.x0, required this.y0, required this.x1, required this.y1, required this.color, required this.radius});
  final int id, line, color;
  final double x0, y0, x1, y1, radius;
}

@immutable
final class QvpHitArea {
  const QvpHitArea({required this.word, required this.line, required this.x0, required this.y0, required this.x1, required this.y1, required this.inkX0, required this.inkY0, required this.inkX1, required this.inkY1});
  final int word, line;
  final double x0, y0, x1, y1, inkX0, inkY0, inkX1, inkY1;
}

@immutable
final class QvpLineBand {
  const QvpLineBand({required this.line, required this.lineNumber, required this.y0, required this.y1, required this.mid, required this.inkY0, required this.inkY1});
  final int line, lineNumber;
  final double y0, y1, mid, inkY0, inkY1;
}

/// Input of [QvpPage.layout]; lengths in viewport px. Spacing only opens up:
/// `lineSpacing` < 1 is clamped by the engine. `gridLines` 0 = the page's own grid.
@immutable
/// How a page reflows when the reader zooms in: the ink grows, fewer words fit a row, the rest
/// move down and the page scrolls. Every field but [zoom] is a typographic knob with a default
/// the engine chose; a reader's control only ever moves [zoom].
@immutable
final class QvpReflowSpec {
  const QvpReflowSpec({
    this.zoom = 1,
    this.fill = 'centred',
    this.breaks = 'fitted',
    this.gaps = 'uniform',
    this.wordGap = 1,
    this.maxStretch = QvpDefaults.reflowMaxStretch,
    this.relax = 0.5,
  });

  /// 1 or less is the printed page; [QvpPage.zoomSpec] hands back 0 there, as the C header
  /// does. The largest this page can reach is [QvpPage.reflowMaxZoom].
  final double zoom;
  /// `ragged` | `justified` | `centred`; `greedy` | `even` | `fitted` | `printed`; `printed` | `uniform`.
  final String fill, breaks, gaps;
  final double wordGap, maxStretch, relax;

  @override
  bool operator ==(Object other) =>
      other is QvpReflowSpec &&
      other.zoom == zoom &&
      other.fill == fill &&
      other.breaks == breaks &&
      other.gaps == gaps &&
      other.wordGap == wordGap &&
      other.maxStretch == maxStretch &&
      other.relax == relax;

  @override
  int get hashCode => Object.hash(zoom, fill, breaks, gaps, wordGap, maxStretch, relax);
}

/// What a pinch does to the page.
enum QvpZoomMode {
  /// The pinch lands on the page's own zoom steps and the page reflows onto one. The default:
  /// a reader never ends up at a zoom that breaks the page badly.
  stepped,

  /// The pinch drives the reflow zoom freely, between the printed page and [QvpPage.reflowMaxZoom].
  continuous,

  /// The pinch scales the printed page as it is. The rows never change.
  magnify,
}

/// Where the reader's zoom control stands. A host keeps this beside its [QvpView]; all zero is
/// the default control, stepped and on the printed page.
@immutable
final class QvpZoom {
  const QvpZoom({this.mode = QvpZoomMode.stepped, this.step = 0, this.zoom = 1});

  final QvpZoomMode mode;

  /// 0 is the printed page, 1 upwards the page's own steps. What [QvpZoomMode.stepped] reads.
  final int step;

  /// The size in force, 1 at the printed page, whichever field the mode steers by.
  final double zoom;

  @override
  bool operator ==(Object other) => other is QvpZoom && other.mode == mode && other.step == step && other.zoom == zoom;

  @override
  int get hashCode => Object.hash(mode, step, zoom);
}

/// A point `p` draws at `offset + scale · p`. The reader's pan and zoom, which the engine moves.
@immutable
final class QvpView {
  const QvpView({this.scale = 1, this.offsetX = 0, this.offsetY = 0});
  final double scale, offsetX, offsetY;

  @override
  bool operator ==(Object other) =>
      other is QvpView && other.scale == scale && other.offsetX == offsetX && other.offsetY == offsetY;

  @override
  int get hashCode => Object.hash(scale, offsetX, offsetY);
}

/// What a gesture produced: the control to keep, the view to draw with, and whether the page was
/// laid out again under it.
@immutable
final class QvpZoomChange {
  const QvpZoomChange({required this.zoom, required this.view, required this.relaid});
  final QvpZoom zoom;
  final QvpView view;
  final bool relaid;
}

/// What a sideways drag means on the page in hand.
enum QvpSideways { pan, turnPage }

/// Where a group of paths is drawn: `x' = kx · x + dx`, `y' = ky · y + dy`, in page units.
@immutable
final class QvpPlacement {
  const QvpPlacement(this.dx, this.dy, this.kx, this.ky);
  static const identity = QvpPlacement(0, 0, 1, 1);
  final double dx, dy, kx, ky;
}

/// One stroke of a layout: the path to draw, under the placement of that index.
@immutable
final class QvpDraw {
  const QvpDraw(this.path, this.placement);
  final int path, placement;
}

final class QvpLayoutSpec {
  const QvpLayoutSpec({
    this.viewportW = 0,
    this.viewportH = 0,
    this.padTop = 0,
    this.padBottom = 0,
    this.padLeft = 0,
    this.padRight = 0,
    this.lineSpacing = 1,
    this.fillHeight = false,
    this.gridLines = 0,
    this.cropLeft = 0,
    this.cropRight = 0,
    this.maxAspectSlack = 0,
    this.reflow,
    this.bannerZoom = 0,
    this.surahFrames = true,
  });
  final double viewportW, viewportH, padTop, padBottom, padLeft, padRight, lineSpacing;

  /// Reflow the page onto rows of the viewport's own width, or null for the printed page.
  /// The reader's zoom control writes this: see [QvpPage.zoomSpec].
  final QvpReflowSpec? reflow;
  final bool fillHeight;
  final int gridLines;

  /// Printed side margins to cut, in page units (0 = keep the print's margins).
  final double cropLeft, cropRight;

  /// The content is never wider than `viewportH · pageW / pageH · maxAspectSlack` (0 = no bound).
  final double maxAspectSlack;

  /// How big a banner — a surah name, a basmalah — may get as the reader zooms in, as a
  /// multiple of its PRINTED size. 0 leaves it uncapped: the drawing grows with the words
  /// around it until it fills the row. 1 holds it at the printed size however far the reader
  /// zooms, which is what a host drawing its own frame around the name wants.
  ///
  /// It sits here and not on [reflow] because it must survive the zoom control: a host that
  /// pinches never builds a [QvpReflowSpec] itself — [QvpPage.zoomSpec] does, from this spec.
  final double bannerZoom;

  /// Draw source-native frames around surah names on the printed page. Reflow stays frameless.
  final bool surahFrames;

  QvpLayoutSpec copyWith({
    double? viewportW,
    double? viewportH,
    double? padTop,
    double? padBottom,
    double? padLeft,
    double? padRight,
    double? lineSpacing,
    bool? fillHeight,
    int? gridLines,
    double? cropLeft,
    double? cropRight,
    double? maxAspectSlack,
    QvpReflowSpec? reflow,
    double? bannerZoom,
    bool? surahFrames,
  }) =>
      QvpLayoutSpec(
        viewportW: viewportW ?? this.viewportW,
        viewportH: viewportH ?? this.viewportH,
        padTop: padTop ?? this.padTop,
        padBottom: padBottom ?? this.padBottom,
        padLeft: padLeft ?? this.padLeft,
        padRight: padRight ?? this.padRight,
        lineSpacing: lineSpacing ?? this.lineSpacing,
        fillHeight: fillHeight ?? this.fillHeight,
        gridLines: gridLines ?? this.gridLines,
        cropLeft: cropLeft ?? this.cropLeft,
        cropRight: cropRight ?? this.cropRight,
        maxAspectSlack: maxAspectSlack ?? this.maxAspectSlack,
        reflow: reflow ?? this.reflow,
        bannerZoom: bannerZoom ?? this.bannerZoom,
        surahFrames: surahFrames ?? this.surahFrames,
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
      other.fillHeight == fillHeight &&
      other.gridLines == gridLines &&
      other.cropLeft == cropLeft &&
      other.cropRight == cropRight &&
      other.maxAspectSlack == maxAspectSlack &&
      other.reflow == reflow &&
      other.bannerZoom == bannerZoom &&
      other.surahFrames == surahFrames;

  @override
  int get hashCode => Object.hash(viewportW, viewportH, padTop, padBottom, padLeft, padRight, lineSpacing, fillHeight, gridLines, cropLeft,
      cropRight, maxAspectSlack, reflow, bannerZoom, surahFrames);
}

/// Output of [QvpPage.layout]. Page → viewport: `viewX = offsetX + x*scale`, `viewY = offsetY + (y + lineDy[line])*scale`.
@immutable
final class QvpLayout {
  const QvpLayout({required this.scale, required this.offsetX, required this.offsetY, required this.contentW, required this.contentH, required this.lineSpacing, required this.lineDy, required this.slots, this.fitScale = 1, this.fitX = 0, this.fitY = 0, this.reflowed = false, this.rows = 0});
  final double scale, offsetX, offsetY, contentW, contentH, lineSpacing;

  /// Whether the page was reflowed onto rows of the viewport's width, and how many rows it made.
  /// When it was, [lineDy] and [slots] describe the rows rather than the printed lines.
  final bool reflowed;
  final int rows;

  /// The view transform that shows the whole content in the viewport (shrink to height, never
  /// enlarge, centred). The host's pan and zoom go on top.
  final double fitScale, fitX, fitY;

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
    this.height = 'lineSpacing',
    this.ink = QvpDefaults.highlightInk,
    this.band = QvpDefaults.highlightBand,
    this.padX = QvpDefaults.highlightPadX,
    this.padY = 0,
    this.radius = 0,
    this.seam = QvpDefaults.highlightSeam,
    this.ms = 0,
    this.layer = QvpLayer.highlight,
  });

  /// 'ink' | 'band' | 'both'
  final String mode;

  /// 'lineSpacing' | 'ink'
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
final class QvpSurah {
  const QvpSurah({required this.number, required this.ayahCount, required this.hasBanner, required this.hasBasmalah, required this.place, required this.bannerDecoration, required this.arabic, required this.latin, required this.english});
  final int number, ayahCount, bannerDecoration;
  final bool hasBanner, hasBasmalah;

  /// 'makkah' | 'madinah' | ''
  final String place, arabic, latin, english;
}

@immutable
final class QvpDivision {
  const QvpDivision({required this.division, required this.line, required this.number, required this.surah, required this.ayah, required this.ayahIndex});

  /// 'juz' | 'hizb' | 'nisf' | 'rubuAlHizb'
  final String division;
  final int line, number, surah, ayah, ayahIndex;
}

@immutable
final class QvpSurahHeader {
  const QvpSurahHeader({required this.decoration, required this.surah, required this.line, required this.x0, required this.y0, required this.x1, required this.y1, required this.titleX0, required this.titleY0, required this.titleX1, required this.titleY1});
  final int decoration, surah, line;

  /// The box a frame fills: the page's text block wide, the heading's row tall.
  final double x0, y0, x1, y1;

  /// The title ink inside that box, with any native frame left out.
  final double titleX0, titleY0, titleX1, titleY1;
}

@immutable
final class QvpAyahMark {
  const QvpAyahMark({required this.decoration, required this.surah, required this.ayah, required this.line, required this.cx, required this.cy, required this.r, required this.ornamentPath, required this.numeralPath});
  final int decoration, surah, ayah, line, ornamentPath, numeralPath;
  final double cx, cy, r;
}

@immutable
final class QvpRosette {
  const QvpRosette({required this.decoration, required this.surah, required this.ayah, required this.juz, required this.hizb, required this.nisf, required this.rubuAlHizb, required this.rubuAlHizbInHizb});
  final int decoration, surah, ayah, juz, hizb, nisf, rubuAlHizb, rubuAlHizbInHizb;
}

@immutable
final class QvpSajdah {
  const QvpSajdah({required this.decoration, required this.surah, required this.ayah, required this.signPath});
  final int decoration, surah, ayah, signPath;
}

@immutable
final class QvpMatch {
  const QvpMatch({required this.word, required this.index, required this.isLooseMatch, required this.wordKey, required this.text});
  final int word, index;
  final bool isLooseMatch;
  final String wordKey, text;
}

@immutable
final class QvpCropBounds {
  const QvpCropBounds({required this.x0, required this.y0, required this.x1, required this.y1, required this.nWords, required this.ayahMarkDecoration});
  final double x0, y0, x1, y1;
  final int nWords, ayahMarkDecoration;
}

@immutable
final class QvpAtlasSurah {
  const QvpAtlasSurah({required this.number, required this.page, required this.ayahCount, required this.place, required this.arabic, required this.latin, required this.english});
  final int number, page, ayahCount;
  final String place, arabic, latin, english;
}

@immutable
final class QvpAtlasRubuAlHizb {
  const QvpAtlasRubuAlHizb({required this.rubuAlHizb, required this.surah, required this.ayah, required this.page});
  final int rubuAlHizb, surah, ayah, page;
  String get ayahKey => '$surah:$ayah';
}

/// One entry of [QvpPage.styled].
typedef QvpStyledPath = ({int path, int color});

// ───────────── dress: another mushaf's ornaments ─────────────

/// What an ornament replaces. The order is `QVP_ORNAMENT_*`.
enum QvpOrnamentKind {
  /// The medallion that closes an ayah.
  ayahMark,

  /// The band that carries a surah's name.
  surahHeader,

  /// The border around the page.
  pageFrame,
}

/// Where an ornament's artwork came from. The order is `QVP_LINEAGE_*`.
enum QvpLineage {
  /// A printed mushaf, traced from a scan.
  scan,

  /// A font.
  font,
}

/// What an ornament may be redistributed under. Read it before you publish a page wearing it.
@immutable
final class QvpOrnamentLicence {
  const QvpOrnamentLicence({required this.id, required this.status, required this.redistributable, required this.attribution});

  /// An SPDX id, or empty when the source does not state one.
  final String id;

  /// How sure the catalogue is of [id].
  final String status;
  final bool redistributable;
  final String attribution;
}

/// One printed colour of an ornament's design. A part with alpha 0 is a window the design
/// leaves open (its slot).
@immutable
final class QvpOrnamentPart {
  const QvpOrnamentPart({required this.index, required this.color, required this.stroke, required this.name});
  final int index;

  /// 0xRRGGBBAA.
  final int color;

  /// True for a line-art part, drawn as constant-width strokes.
  final bool stroke;
  final String name;
}

/// One ornament of a [QvpOrnaments] set.
@immutable
final class QvpOrnament {
  const QvpOrnament({
    required this.index,
    required this.kind,
    required this.lineage,
    required this.style,
    required this.riwayah,
    required this.hasSlices,
    required this.licence,
    required this.parts,
  });

  /// Its index in the set, which is what [QvpDressInfo] reports.
  final int index;
  final QvpOrnamentKind kind;
  final QvpLineage lineage;

  /// The mushaf or font it comes from.
  final String style;
  final String riwayah;

  /// True when a border can be tiled along the edges. Without slices it keeps its printed
  /// proportions.
  final bool hasSlices;
  final QvpOrnamentLicence licence;

  /// The design's printed palette, one entry per colour.
  final List<QvpOrnamentPart> parts;
}

/// A style of a [QvpOrnaments] set, with the ornament it has of each kind or null.
@immutable
final class QvpOrnamentStyle {
  const QvpOrnamentStyle({required this.style, required this.riwayah, required this.lineage, this.ayahMark, this.surahHeader, this.pageFrame});
  final String style, riwayah;
  final QvpLineage lineage;
  final QvpOrnament? ayahMark, surahHeader, pageFrame;

  /// The ornament of [kind], or null.
  QvpOrnament? of(QvpOrnamentKind kind) => switch (kind) {
        QvpOrnamentKind.ayahMark => ayahMark,
        QvpOrnamentKind.surahHeader => surahHeader,
        QvpOrnamentKind.pageFrame => pageFrame,
      };

  /// True when the style has all three kinds, so it can dress a page on its own.
  bool get isComplete => ayahMark != null && surahHeader != null && pageFrame != null;
}

/// What [QvpPage.dress] puts on the page. [style] names a mushaf and takes its medallion,
/// band and border together. Each of [ayahMark], [surahHeader] and [pageFrame] is a
/// [QvpOrnament], its index, or a style name, and overrides [style] for that kind. Lengths are
/// page units.
@immutable
final class QvpDressSpec {
  const QvpDressSpec({
    this.style,
    this.ayahMark,
    this.surahHeader,
    this.pageFrame,
    this.gap = 5,
    this.bandGap = 5,
    this.ayahMarkScale = 0,
    this.pageFrameScale = 1,
    this.lineArt = const {},
    this.colors = const [],
  });

  /// A mushaf whose three ornaments are taken together.
  final String? style;

  /// A [QvpOrnament], its index or a style name. Null takes [style].
  final Object? ayahMark, surahHeader, pageFrame;

  /// The air between the text and the border.
  final double gap;

  /// The air between a band and the border.
  final double bandGap;

  /// The medallion's size against the printed ring. 0 takes the design's own:
  /// [QvpDefaults.ayahMarkScaleScan] for a scanned mushaf and 1 for a font marker.
  final double ayahMarkScale;

  /// The border's size against the design's own. The pattern scales as a whole, so a bigger
  /// border fits fewer motifs along each edge. Nothing is stretched.
  final double pageFrameScale;

  /// The kinds drawn as line art, their constant-width strokes alone. Pass
  /// `QvpOrnamentKind.values.toSet()` for all of them.
  final Set<QvpOrnamentKind> lineArt;

  /// Repaints: `(ornament, part, colour)`. The ornament is a [QvpOrnament] or its index, and the
  /// colour is anything [rgba] takes. A part left out keeps the design's own colour.
  final List<(Object, int, Object)> colors;

  QvpDressSpec copyWith({
    Object? ayahMark = _keep,
    Object? surahHeader = _keep,
    Object? pageFrame = _keep,
    double? gap,
    double? bandGap,
    double? ayahMarkScale,
    double? pageFrameScale,
    Set<QvpOrnamentKind>? lineArt,
    List<(Object, int, Object)>? colors,
  }) =>
      QvpDressSpec(
        style: style,
        ayahMark: identical(ayahMark, _keep) ? this.ayahMark : ayahMark,
        surahHeader: identical(surahHeader, _keep) ? this.surahHeader : surahHeader,
        pageFrame: identical(pageFrame, _keep) ? this.pageFrame : pageFrame,
        gap: gap ?? this.gap,
        bandGap: bandGap ?? this.bandGap,
        ayahMarkScale: ayahMarkScale ?? this.ayahMarkScale,
        pageFrameScale: pageFrameScale ?? this.pageFrameScale,
        lineArt: lineArt ?? this.lineArt,
        colors: colors ?? this.colors,
      );

  static const Object _keep = Object();
}

/// The readout of a dressed page.
@immutable
final class QvpDressInfo {
  const QvpDressInfo({
    required this.ayahMark,
    required this.surahHeader,
    required this.pageFrame,
    required this.ayahMarks,
    required this.surahHeaders,
    required this.frameRepeats,
    required this.frameStretched,
    required this.draws,
    required this.revision,
    required this.viewBox,
    required this.overflow,
  });

  /// The ornament index of each kind on the page, or null.
  final int? ayahMark, surahHeader, pageFrame;

  /// How many medallions and bands were placed.
  final int ayahMarks, surahHeaders;

  /// How many times the border's repeat was tiled. 0 when it is stretched.
  final int frameRepeats;

  /// True when the border has no slices and is stretched whole.
  final bool frameStretched;

  /// How many entries [QvpPage.dressGeometry] holds.
  final int draws;

  /// Counts up on every rebuild: a new dress, and every layout. Key an ornament raster on it.
  final int revision;

  /// `x, y, w, h` in page units. A border grows it.
  final List<double> viewBox;

  /// `left, top, right, bottom` in viewport px: how far the border reaches past the laid-out
  /// content. Fit content plus overflow, or the border is cropped off.
  final List<double> overflow;
}

/// [QvpPage.dressView]: the view for a host that scrolls the page, and the dressed box's size.
@immutable
final class QvpDressView {
  const QvpDressView({required this.view, required this.width, required this.height});

  /// Shows the dressed box with its top at 0, centred across the viewport.
  final QvpView view;

  /// The dressed box at the asked scale, in viewport px: the size to give the canvas.
  final double width, height;
}

/// One entry of the ornament display list, in page units. Paint it behind the page ink.
@immutable
final class QvpDressDraw {
  const QvpDressDraw({required this.path, required this.color, required this.kind, required this.evenOdd, required this.stroke, required this.strokeWidth, required this.part, required this.placement});

  /// The outline, with the fill rule already set.
  final Path path;

  /// 0xRRGGBBAA.
  final int color;
  final QvpOrnamentKind kind;
  final bool evenOdd;

  /// True to stroke the path with [strokeWidth], round caps and round joins. False to fill it.
  final bool stroke;

  /// Page units.
  final double strokeWidth;

  /// The part of the ornament's palette it draws.
  final int part;

  /// The index into [QvpPage.layoutPlacements] to draw it under, as a page draw. Null for the
  /// border, which is drawn in layout space.
  final int? placement;
}


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
    for (final t in [QvpNames.mark, QvpNames.kind, QvpNames.family, QvpNames.category, QvpNames.decoration, QvpNames.division, QvpNames.place]) {
      QvpNames._tables[t] = names(t);
    }
    _spec = pffi.calloc<QvpLayoutSpecC>();
    _layout = pffi.calloc<QvpLayoutC>();
    _hitOpt = pffi.calloc<QvpHitOptionsC>();
    _hit = pffi.calloc<QvpHitC>();
    _crop = pffi.calloc<QvpCropBoundsC>();
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
  late final ffi.Pointer<QvpCropBoundsC> _crop;
  bool _disposed = false;

  /// Format version the library was built for.
  /// The engine version, e.g. `0.2.0`.
  String get version {
    b.version(_str);
    return _s();
  }

  /// The page format version the engine reads.
  int get formatVersion => b.formatVersion();

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

  String markName(int m) => _name(b.markName, m);
  String familyName(int f) => _name(b.familyName, f);
  String kindName(int k) => _name(b.kindName, k);
  String categoryName(int c) => _name(b.categoryName, c);
  int markCategory(Object m) => b.markCategory(QvpMark.id(m));
  int markFromName(String s) => withString(s, (p, n) => b.markFromName(p, n));
  int nameCount(int table) => b.nameCount(table);
  String name(int table, int id) => _name((v, out) => b.name(table, v, out), id);
  /// 255 when the table has no such name.
  int nameId(int table, String s) => withString(s, (p, n) => b.nameId(table, p, n));
  /// Every name of a table, index = id.
  List<String> names(int table) => List.generate(nameCount(table), (i) => name(table, i), growable: false);
  String decorationName(int k) => name(QvpNames.decoration, k);
  String divisionName(int d) => name(QvpNames.division, d);
  String placeName(int p) => name(QvpNames.place, p);

  // Arabic text tools
  String _arabic(int op, String s) => withString(s, (p, n) {
        b.arabic(op, p, n, _str);
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

  /// Decodes an atlas (`atlas.qva`). Throws on a bad file.
  QvpAtlas loadAtlas(Uint8List bytes) {
    final h = withBytes(bytes, (p, n) => b.atlasLoad(p, n));
    if (h == ffi.nullptr) throw const FormatException('qvp_atlas_load failed');
    return QvpAtlas._(this, h);
  }

  /// Decodes an ornament set (`ornaments.qvo`). The engine copies the bytes. Throws on a bad file.
  QvpOrnaments loadOrnaments(Uint8List bytes) {
    final h = withBytes(bytes, (p, n) => b.ornamentsLoad(p, n));
    if (h == ffi.nullptr) throw const FormatException('qvp_ornaments_load failed');
    return QvpOrnaments._(this, h);
  }

  /// Scales [view] by [factor] about a point in viewport px and holds the scale between [min]
  /// and [max]. 0 is the engine's own limit, and a ceiling under the floor is the floor.
  QvpView viewZoomAbout(QvpView view, double focalX, double focalY, double factor, {double min = 0, double max = 0}) {
    final v = pffi.calloc<QvpViewC>(), out = pffi.calloc<QvpViewC>();
    try {
      v.ref.scale = view.scale;
      v.ref.offsetX = view.offsetX;
      v.ref.offsetY = view.offsetY;
      b.viewZoomAbout(v, focalX, focalY, factor, min, max, out);
      return QvpView(scale: out.ref.scale, offsetX: out.ref.offsetX, offsetY: out.ref.offsetY);
    } finally {
      pffi.calloc.free(v);
      pffi.calloc.free(out);
    }
  }


  /// Releases the scratch memory. Free pages and atlases first.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    for (final p in <ffi.Pointer>[_scratch, _str, _target, _sel, _hl, _theme, _spec, _layout, _hitOpt, _hit, _crop]) {
      pffi.calloc.free(p);
    }
  }

  // struct writers
  ffi.Pointer<QvpSelectorC> _writeSel(QvpSelector s) {
    final r = _sel.ref;
    r.selector = s.selector;
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
      nDecorations = i.nDecorations;
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
    decorations = List.generate(nDecorations, _deco, growable: false);
    lineSpacing = b.pageLineSpacing(_h);
  }

  final QvpEngine engine;
  ffi.Pointer<QvpPageC> _h;

  late final double width, height, lineSpacing;
  late final int page, nLines, nAyahs, nWords, nPaths, nDecorations;

  /// Opcode stream (see [QvpOp]).
  late final Uint8List ops;

  /// x,y pairs consumed in order by the opcodes.
  late final Float32List pts;

  /// `nPaths × 8`: op_start, op_count, pt_start, pt_count, flags, word, line, extra.
  late final Uint32List table;

  late final List<QvpWordInfo> words;
  late final List<QvpAyahInfo> ayahs;
  late final List<QvpLineInfo> lines;
  late final List<QvpDecorationInfo> decorations;

  /// Layout from the last [layout] call, if any.
  QvpLayout? currentLayout;

  int _defaultInk = QvpDefaults.ink;

  /// Dart-side mirror of the engine's default ink.
  int get defaultInk => _defaultInk;

  /// Bumped on every mutating call.
  int revision = 0;

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
        index: i,
        surah: w.surah,
        ayah: w.ayah,
        word: w.word,
        line: w.lineNumber,
        ayahIndex: w.ayahIndex,
        lineIndex: w.lineIndex,
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
        index: i,
        surah: a.surah,
        ayah: a.ayah,
        fragment: a.fragment,
        fragments: a.fragments,
        flags: a.flags,
        rubuAlHizb: a.rubuAlHizb,
        firstWord: a.firstWord,
        nWords: a.nWords,
        ayahMarkDecoration: a.ayahMarkDecoration,
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
        index: i,
        lineNumber: l.lineNumber,
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

  QvpDecorationInfo _deco(int i) {
    final o = pffi.calloc<QvpDecorationInfoC>();
    try {
      if (_b.decorationInfo(_h, i, o) == 0) throw StateError('qvp_decoration_info($i) failed');
      final d = o.ref;
      return QvpDecorationInfo(
        index: i,
        decoration: d.decoration,
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
    r.target = t.target;
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
  List<int> targetWords(Object target) => _t(target, (t) => _u32(_b.targetWords(_p, t, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>()))));

  // ── metadata ──
  List<QvpSurah> surahs() {
    final n = _b.surahCount(_p);
    final o = pffi.calloc<QvpSurahC>();
    try {
      return List.generate(n, (i) {
        _b.surahAt(_p, i, o);
        final s = o.ref;
        return QvpSurah(
          number: s.number,
          ayahCount: s.ayahCount,
          hasBanner: s.hasBanner != 0,
          hasBasmalah: s.hasBasmalah != 0,
          place: _e.placeName(s.place),
          bannerDecoration: s.bannerDecoration,
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
      return QvpDivision(division: _e.divisionName(d.division), line: d.line, number: d.number, surah: d.surah, ayah: d.ayah, ayahIndex: d.ayahIndex);
    }, growable: false);
  }

  /// The surah headings, in page units: the box a frame fills, and the title ink in it.
  List<QvpSurahHeader> surahHeaders() => _surahHeaders(view: false);

  /// The same, in viewport px through the current layout: what a host frames.
  List<QvpSurahHeader> surahHeadersView() => _surahHeaders(view: true);

  List<QvpSurahHeader> _surahHeaders({required bool view}) {
    final o = _e._out<QvpSurahHeaderC>(), cap = _e._cap(ffi.sizeOf<QvpSurahHeaderC>());
    final n = (view ? _b.surahHeadersView(_p, o, cap) : _b.surahHeaders(_p, o, cap)).clamp(0, cap);
    return List.generate(n, (i) {
      final h = (o + i).ref;
      return QvpSurahHeader(decoration: h.decoration, surah: h.surah, line: h.line, x0: h.x0, y0: h.y0, x1: h.x1, y1: h.y1, titleX0: h.titleX0, titleY0: h.titleY0, titleX1: h.titleX1, titleY1: h.titleY1);
    }, growable: false);
  }

  /// Real ayah medallions.
  List<QvpAyahMark> ayahMarks() {
    final o = _e._out<QvpAyahMarkC>(), cap = _e._cap(ffi.sizeOf<QvpAyahMarkC>());
    final n = _b.ayahMarks(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final m = (o + i).ref;
      return QvpAyahMark(decoration: m.decoration, surah: m.surah, ayah: m.ayah, line: m.line, cx: m.cx, cy: m.cy, r: m.r, ornamentPath: m.ornamentPath, numeralPath: m.numeralPath);
    }, growable: false);
  }

  /// Drawn hizb rosettes.
  List<QvpRosette> rosettes() {
    final o = _e._out<QvpRosetteC>(), cap = _e._cap(ffi.sizeOf<QvpRosetteC>());
    final n = _b.rosettes(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final r = (o + i).ref;
      return QvpRosette(decoration: r.decoration, surah: r.surah, ayah: r.ayah, juz: r.juz, hizb: r.hizb, nisf: r.nisf, rubuAlHizb: r.rubuAlHizb, rubuAlHizbInHizb: r.rubuAlHizbInHizb);
    }, growable: false);
  }

  List<QvpSajdah> sajdahs() {
    final o = _e._out<QvpSajdahC>(), cap = _e._cap(ffi.sizeOf<QvpSajdahC>());
    final n = _b.sajdahs(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final s = (o + i).ref;
      return QvpSajdah(decoration: s.decoration, surah: s.surah, ayah: s.ayah, signPath: s.signPath);
    }, growable: false);
  }

  /// `(surah, ayah)` keys in reading order.
  List<(int, int)> ayahKeys() => _u32(_b.ayahKeys(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>()))).map((k) => (k >> 16, k & 0xffff)).toList(growable: false);

  /// Words of an ayah on this page and whether the ayah is complete here.
  ({int count, bool isComplete}) ayahWordCount(int surah, int ayah) {
    final o = _e._out<ffi.Uint8>();
    final n = _b.ayahWordCount(_p, surah, ayah, o);
    return (count: n, isComplete: o.value != 0);
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

  /// Accessibility label of ayah record [ayahIndex].
  String ayahLabel(int ayahIndex) {
    _b.ayahLabel(_p, ayahIndex, _e._str);
    return _e._s();
  }

  // ── text & search ──
  /// Text of a target (default: the page).
  String text([Object target = 'page', Object form = 'rasmUthmani', String wordSep = ' ', String lineSep = '\n']) => _e.withString(wordSep, (wp, wn) => _e.withString(lineSep, (lp, ln) => _t(target, (t) {
        _b.text(_p, t, QvpForm.of(form), wp, wn, lp, ln, _e._str);
        return _e._s();
      })));

  /// Search this page. [mode]: 'includes' | 'exact' | 'prefix'.
  List<QvpMatch> search(String query, {Object form = 'search', String mode = 'includes', bool normalize = true, bool looseMatch = true, int limit = 0}) {
    final o = _e._out<QvpMatchC>(), cap = _e._cap(ffi.sizeOf<QvpMatchC>());
    final n = _e.withString(query, (p, len) => _b.search(_p, p, len, QvpForm.of(form), const {'includes': 0, 'exact': 1, 'prefix': 2}[mode] ?? 0, normalize ? 1 : 0, looseMatch ? 1 : 0, limit, o, cap)).clamp(0, cap);
    return List.generate(n, (i) {
      final m = (o + i).ref;
      return QvpMatch(word: m.word, index: m.index, isLooseMatch: m.isLooseMatch != 0, wordKey: wordKey(m.word), text: words[m.word].text);
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

  /// Exact outline hit, page units.
  QvpHit? hitTestExact(double x, double y) => _readHit(_b.hitTestExact(_p, x, y, _e._hit));

  /// Exact outline hit, viewport px through the current layout.
  QvpHit? hitTestExactView(double viewX, double viewY) => _readHit(_b.hitTestExactView(_p, viewX, viewY, _e._hit));

  ffi.Pointer<QvpHitOptionsC> _opt(QvpHitOptions o) {
    final r = _e._hitOpt.ref;
    r.maxDistance = o.maxDistance;
    r.gapBias = o.gapBias;
    r.preferExact = o.preferExact ? 1 : 0;
    return _e._hitOpt;
  }

  QvpHit? _readHit(int ok) {
    if (ok == 0) return null;
    final h = _e._hit.ref;
    final w = _idx(h.word);
    return QvpHit(
      word: w,
      path: _idx(h.path),
      decoration: _idx(h.decoration),
      line: h.line,
      distance: h.distance,
      isExact: h.isExact != 0,
      wordKey: w >= 0 ? words[w].wordKey : null,
      ayahKey: w >= 0 ? words[w].ayahKey : null,
    );
  }

  /// Gap-aware: every point on a printed line resolves to the word the user meant. Page units.
  QvpHit? hitTest(double x, double y, [QvpHitOptions opt = const QvpHitOptions()]) => _readHit(_b.hitTest(_p, x, y, _opt(opt), _e._hit));

  /// Gap-aware, viewport px through the current layout.
  QvpHit? hitTestView(double viewX, double viewY, [QvpHitOptions opt = const QvpHitOptions()]) => _readHit(_b.hitTestView(_p, viewX, viewY, _opt(opt), _e._hit));

  List<QvpLineBand> lineBands() {
    final o = _e._out<QvpLineBandC>(), cap = _e._cap(ffi.sizeOf<QvpLineBandC>());
    final n = _b.lineBands(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final l = (o + i).ref;
      return QvpLineBand(line: l.line, lineNumber: l.lineNumber, y0: l.y0, y1: l.y1, mid: l.mid, inkY0: l.inkY0, inkY1: l.inkY1);
    }, growable: false);
  }

  List<QvpHitArea> hitAreas([double gapBias = QvpDefaults.gapBias]) {
    final o = _e._out<QvpHitAreaC>(), cap = _e._cap(ffi.sizeOf<QvpHitAreaC>());
    final n = _b.hitAreas(_p, gapBias, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final h = (o + i).ref;
      return QvpHitArea(word: h.word, line: h.line, x0: h.x0, y0: h.y0, x1: h.x1, y1: h.y1, inkX0: h.inkX0, inkY0: h.inkY0, inkX1: h.inkX1, inkY1: h.inkY1);
    }, growable: false);
  }

  // ── layout ──
  /// Computes and stores an engine layout (also used by the *View hit tests and box outputs).
  QvpLayout layout(QvpLayoutSpec spec) {
    _writeSpec(spec);
    _b.layout(_p, _e._spec, _e._layout);
    final o = _e._layout.ref;
    final n = o.nLines;
    final f = o.lines == ffi.nullptr ? Float32List(0) : o.lines.asTypedList(n * 3);
    final lineDy = Float32List(n);
    final slots = List<(double, double)>.generate(n, (i) {
      lineDy[i] = f[i * 3];
      return (f[i * 3 + 1], f[i * 3 + 2]);
    }, growable: false);
    final l = QvpLayout(scale: o.scale, offsetX: o.offsetX, offsetY: o.offsetY, contentW: o.contentW, contentH: o.contentH, lineSpacing: o.lineSpacing, lineDy: lineDy, slots: slots, fitScale: o.fitScale, fitX: o.fitX, fitY: o.fitY, reflowed: o.reflowed != 0, rows: o.nRows);
    currentLayout = l;
    _touch();
    return l;
  }

  /// Leading (page units) that makes this page fill the padded viewport of [spec]; `max` 0 = unlimited.
  void _writeSpec(QvpLayoutSpec spec) {
    final s = _e._spec.ref;
    s.viewportW = spec.viewportW;
    s.viewportH = spec.viewportH;
    s.padTop = spec.padTop;
    s.padBottom = spec.padBottom;
    s.padLeft = spec.padLeft;
    s.padRight = spec.padRight;
    s.lineSpacing = spec.lineSpacing;
    s.fillHeight = spec.fillHeight ? 1 : 0;
    s.gridLines = spec.gridLines;
    s.cropLeft = spec.cropLeft;
    s.cropRight = spec.cropRight;
    s.maxAspectSlack = spec.maxAspectSlack;
    final r = spec.reflow;
    // reflow_zoom 0 means "no reflow": the printed page, and every other knob ignored
    s.reflowZoom = r == null ? 0 : r.zoom;
    // 255 is "the engine's own default" for the two that have one
    s.reflowFill = r == null ? 255 : (_fill[r.fill] ?? 255);
    s.reflowBreaks = r == null ? 255 : (_breaks[r.breaks] ?? 255);
    s.reflowGaps = r == null ? 1 : (r.gaps == 'printed' ? 0 : 1);
    s.reflowWordGap = r == null ? 0 : r.wordGap;
    s.reflowMaxStretch = r == null ? 0 : r.maxStretch;
    s.reflowRelax = r == null ? -1 : r.relax;
    // Not a reflow knob in the wrappers: the zoom control builds the reflow spec itself, so a
    // cap that lives on it would never reach a page the reader pinched.
    s.bannerZoom = spec.bannerZoom;
    s.surahFrames = spec.surahFrames ? 1 : 0;
  }

  static const _fill = {'ragged': 0, 'justified': 1, 'centred': 2, 'centered': 2};
  static const _breaks = {'greedy': 0, 'even': 1, 'fitted': 2, 'printed': 3};

  // ── the reader's zoom control ──
  // The engine owns the policy: which step a pinch lands on, when it commits, and what holds
  // the reader's place when the page reflows under the fingers. A host reads the gesture and
  // keeps the [QvpZoom] it gets back.

  void _writeZoom(ffi.Pointer<QvpZoomC> z, QvpZoom v) {
    z.ref.mode = v.mode.index;
    z.ref.step = v.step;
    z.ref.zoom = v.zoom;
  }

  void _writeView(ffi.Pointer<QvpViewC> p, QvpView v) {
    p.ref.scale = v.scale;
    p.ref.offsetX = v.offsetX;
    p.ref.offsetY = v.offsetY;
  }

  QvpZoom _readZoom(QvpZoomC z) =>
      QvpZoom(mode: QvpZoomMode.values[z.mode.clamp(0, QvpZoomMode.values.length - 1)], step: z.step, zoom: z.zoom);

  QvpView _readView(QvpViewC v) => QvpView(scale: v.scale, offsetX: v.offsetX, offsetY: v.offsetY);

  /// One frame of a pinch. [factor] is the distance between the fingers against the distance
  /// when they went down — the whole gesture, not the step since the last frame — and [focal] is
  /// the point between them, in viewport px. Pass the [QvpZoom] the gesture began on every
  /// frame: measured against that, a slow pinch walks the steps one at a time.
  QvpZoomChange zoomPinch(QvpLayoutSpec spec, QvpZoom zoom, QvpView view, double factor, double focalX, double focalY) {
    _writeSpec(spec);
    final z = pffi.calloc<QvpZoomC>(), v = pffi.calloc<QvpViewC>(), out = pffi.calloc<QvpZoomChangeC>();
    try {
      _writeZoom(z, zoom);
      _writeView(v, view);
      _b.zoomPinch(_p, _e._spec, z, v, factor, focalX, focalY, out);
      return QvpZoomChange(zoom: _readZoom(out.ref.zoom), view: _readView(out.ref.view), relaid: out.ref.relaid != 0);
    } finally {
      pffi.calloc.free(z);
      pffi.calloc.free(v);
      pffi.calloc.free(out);
    }
  }

  /// The control moved to a step directly: a size button, a double tap, a reset. Step 0 is the
  /// printed page.
  QvpZoomChange zoomToStep(QvpLayoutSpec spec, QvpZoom zoom, int step, QvpView view) {
    _writeSpec(spec);
    final z = pffi.calloc<QvpZoomC>(), v = pffi.calloc<QvpViewC>(), out = pffi.calloc<QvpZoomChangeC>();
    try {
      _writeZoom(z, zoom);
      _writeView(v, view);
      _b.zoomToStep(_p, _e._spec, z, step, v, out);
      return QvpZoomChange(zoom: _readZoom(out.ref.zoom), view: _readView(out.ref.view), relaid: out.ref.relaid != 0);
    } finally {
      pffi.calloc.free(z);
      pffi.calloc.free(v);
      pffi.calloc.free(out);
    }
  }

  /// The same control under another policy, keeping the size the reader is already at.
  QvpZoom zoomMode(QvpLayoutSpec spec, QvpZoom zoom, QvpZoomMode mode) {
    _writeSpec(spec);
    final z = pffi.calloc<QvpZoomC>(), out = pffi.calloc<QvpZoomC>();
    try {
      _writeZoom(z, zoom);
      _b.zoomMode(_p, _e._spec, z, mode.index, out);
      return _readZoom(out.ref);
    } finally {
      pffi.calloc.free(z);
      pffi.calloc.free(out);
    }
  }

  /// The spec this control asks for: what to lay out, draw and hit-test with. The host's other
  /// reflow knobs come back as it set them, on the printed page too, where the zoom is 0 and
  /// they wait for the next pinch.
  QvpLayoutSpec zoomSpec(QvpLayoutSpec spec, QvpZoom zoom) {
    _writeSpec(spec);
    final z = pffi.calloc<QvpZoomC>(), out = pffi.calloc<QvpLayoutSpecC>();
    try {
      _writeZoom(z, zoom);
      _b.zoomSpec(_p, _e._spec, z, out);
      final o = out.ref;
      // Built rather than copied: a control on the printed page asks for no reflow at all, and
      // `copyWith` cannot say "none" — its `??` would keep the reflow the spec came in with.
      final knobs = spec.reflow ?? const QvpReflowSpec();
      // The control owns the zoom alone. Dropping the host's knobs at the printed size would
      // leave the pinch that leaves it to lay the page out with the engine's defaults, so a
      // spec that came with knobs keeps them, at zoom 0, until the next pinch.
      final zoomed = o.reflowZoom > 1.0001;
      return QvpLayoutSpec(
        viewportW: spec.viewportW,
        viewportH: spec.viewportH,
        padTop: spec.padTop,
        padBottom: spec.padBottom,
        padLeft: spec.padLeft,
        padRight: spec.padRight,
        lineSpacing: spec.lineSpacing,
        fillHeight: spec.fillHeight,
        gridLines: spec.gridLines,
        cropLeft: spec.cropLeft,
        cropRight: spec.cropRight,
        maxAspectSlack: spec.maxAspectSlack,
        // Carried over like every other layout knob. A spec rebuilt without it drops the cap on
        // exactly the layout it is for: only a reflowed page grows a banner, and this is the
        // spec every reflowed page is laid out with.
        bannerZoom: spec.bannerZoom,
        surahFrames: spec.surahFrames,
        reflow: zoomed || spec.reflow != null
            ? QvpReflowSpec(
                zoom: zoomed ? o.reflowZoom : 0,
                fill: knobs.fill,
                breaks: knobs.breaks,
                gaps: knobs.gaps,
                wordGap: knobs.wordGap,
                maxStretch: knobs.maxStretch,
                relax: knobs.relax,
              )
            : null,
      );
    } finally {
      pffi.calloc.free(z);
      pffi.calloc.free(out);
    }
  }

  /// The same control on this page: what a page turn keeps. A step carries as a step, because
  /// every page's steps are its own; a free zoom carries as a size, held inside what this page
  /// can reach.
  QvpZoom zoomCarried(QvpLayoutSpec spec, QvpZoom zoom) {
    _writeSpec(spec);
    final z = pffi.calloc<QvpZoomC>(), out = pffi.calloc<QvpZoomC>();
    try {
      _writeZoom(z, zoom);
      _b.zoomCarried(_p, _e._spec, z, out);
      return _readZoom(out.ref);
    } finally {
      pffi.calloc.free(z);
      pffi.calloc.free(out);
    }
  }

  /// The reflow zoom one step of this page's control means. Step 0 is the printed page.
  double zoomAtStep(QvpLayoutSpec spec, int step) {
    _writeSpec(spec);
    return _b.zoomAtStep(_p, _e._spec, step);
  }

  /// The zoom steps this page ships with, above the printed page.
  Float32List zoomSteps(QvpLayoutSpec spec) {
    _writeSpec(spec);
    final o = _e._out<ffi.Float>(), cap = _e._cap(4);
    final n = _b.zoomSteps(_p, _e._spec, o, cap).clamp(0, cap);
    return Float32List.fromList(o.asTypedList(n));
  }

  /// The largest reflow zoom whose rows still hold every word of this page.
  double reflowMaxZoom(QvpLayoutSpec spec) {
    _writeSpec(spec);
    return _b.reflowMaxZoom(_p, _e._spec);
  }

  /// Has the reader zoomed in, by either road: a magnified view or a reflowed page.
  /// [fitScale] is [QvpLayout.fitScale]; 0 means the page is at its fitted size.
  bool isZoomed(QvpZoom zoom, QvpView view, [double fitScale = 0]) {
    final z = pffi.calloc<QvpZoomC>(), v = pffi.calloc<QvpViewC>();
    try {
      _writeZoom(z, zoom);
      _writeView(v, view);
      return _b.zoomIsZoomed(z, v, fitScale) != 0;
    } finally {
      pffi.calloc.free(z);
      pffi.calloc.free(v);
    }
  }

  /// What a sideways drag means on the page in hand: pan it, or turn it.
  QvpSideways sidewaysDrag(QvpZoom zoom, QvpView view, [double fitScale = 0]) {
    final z = pffi.calloc<QvpZoomC>(), v = pffi.calloc<QvpViewC>();
    try {
      _writeZoom(z, zoom);
      _writeView(v, view);
      return _b.sidewaysDrag(_p, z, v, fitScale) == 1 ? QvpSideways.turnPage : QvpSideways.pan;
    } finally {
      pffi.calloc.free(z);
      pffi.calloc.free(v);
    }
  }

  // ── what the layout draws ──

  /// Everything the current layout draws, in drawing order. Each [QvpDraw] names a path and the
  /// index of the placement to draw it under, from [layoutPlacements]. Omitted paths never
  /// appear and a repeated one appears twice, so one loop draws any page, reflowed or not.
  ///
  /// [bandTop] and [bandBottom] cut it to a band of the page, in viewport px; leave them out for
  /// the whole page.
  List<QvpDraw> layoutDrawList({double bandTop = 0, double bandBottom = 0}) {
    final o = _e._out<ffi.Uint32>(), cap = _e._cap(8);
    final n = _b.layoutDrawList(_p, bandTop, bandBottom, o, cap).clamp(0, cap);
    final f = o.asTypedList(n * 2);
    return List.generate(n, (i) => QvpDraw(f[i * 2], f[i * 2 + 1]), growable: false);
  }

  /// What a draw list's `placement` indexes: the groups, then the repeats.
  List<QvpPlacement> layoutPlacements() {
    final o = _e._out<ffi.Float>(), cap = _e._cap(16);
    final n = _b.layoutPlacements(_p, o, cap).clamp(0, cap);
    final f = o.asTypedList(n * 4);
    return List.generate(n, (i) => QvpPlacement(f[i * 4], f[i * 4 + 1], f[i * 4 + 2], f[i * 4 + 3]), growable: false);
  }

  /// The layout the page already has, without computing one, or null when it has none. A zoom
  /// call lays the page out again, so this is how a host reads what it must now draw.
  QvpLayout? readLayout() {
    if (_b.layoutCurrent(_p, _e._layout) == 0) return null;
    final o = _e._layout.ref;
    final n = o.nLines;
    final f = o.lines == ffi.nullptr ? Float32List(0) : o.lines.asTypedList(n * 3);
    final lineDy = Float32List(n);
    final slots = List<(double, double)>.generate(n, (i) {
      lineDy[i] = f[i * 3];
      return (f[i * 3 + 1], f[i * 3 + 2]);
    }, growable: false);
    final l = QvpLayout(scale: o.scale, offsetX: o.offsetX, offsetY: o.offsetY, contentW: o.contentW, contentH: o.contentH, lineSpacing: o.lineSpacing, lineDy: lineDy, slots: slots, fitScale: o.fitScale, fitX: o.fitX, fitY: o.fitY, reflowed: o.reflowed != 0, rows: o.nRows);
    currentLayout = l;
    return l;
  }

  /// The row a word landed on when the page reflowed, or null when it did not.
  int? layoutWordRow(int word) {
    final r = _b.layoutWordRow(_p, word);
    return r == 0xFFFFFFFF ? null : r;
  }

  /// The words of one reflowed row, in drawing order.
  Uint32List layoutRowWords(int row) {
    final o = _e._out<ffi.Uint32>(), cap = _e._cap(4);
    final n = _b.layoutRowWords(_p, row, o, cap).clamp(0, cap);
    return Uint32List.fromList(o.asTypedList(n));
  }

  /// The grid this page is laid out inside: the mushaf's line count and the printed line spacing.
  ({int lines, double lineSpacing}) get grid {
    final g = pffi.calloc<QvpGridC>();
    try {
      _b.pageGrid(_p, g);
      return (lines: g.ref.lines, lineSpacing: g.ref.lineSpacing);
    } finally {
      pffi.calloc.free(g);
    }
  }

  /// The share of the padded viewport of [spec] left empty when the page is fitted to width.
  double layoutWastedFraction(QvpLayoutSpec spec) {
    _writeSpec(spec);
    return _b.layoutWastedFraction(_p, _e._spec);
  }

  /// The page's height laid out for [spec] at the printed pitch, in viewport px: `contentH`
  /// before fill-height adds any leading, padding included. What a host asks before it sizes
  /// the canvas.
  double layoutPrintedHeight(QvpLayoutSpec spec) {
    _writeSpec(spec);
    return _b.layoutPrintedHeight(_p, _e._spec);
  }

  double layoutLineSpacingToFill(QvpLayoutSpec spec, [double max = 0]) {
    _writeSpec(spec);
    return _b.layoutLineSpacingToFill(_p, _e._spec, max);
  }

  /// Word bbox in viewport px through the current layout.
  ({double x0, double y0, double x1, double y1}) wordBoundsView(int i) {
    final o = _e._out<ffi.Float>();
    _b.wordBoundsView(_p, i, o);
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
  int removeStyle(int handle) {
    final n = _b.styleRemove(_p, handle);
    _touch();
    return n;
  }

  int recolorStyle(int handle, Object color, [int ms = 0]) {
    final n = _b.styleRecolor(_p, handle, rgba(color), ms);
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

  void setDefaultColor(Object color) {
    _defaultInk = rgba(color);
    _b.styleDefaultColor(_p, _defaultInk);
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
  Uint32List colors() => Uint32List.fromList(_b.colors(_p).asTypedList(nPaths));

  /// Paths whose colour differs from the default ink (mid-transition values included).
  List<QvpStyledPath> styledPaths() {
    final n = _b.styledPaths(_p, ffi.nullptr, 0);
    if (n == 0) return const [];
    final buf = pffi.malloc<ffi.Uint32>(n * 2);
    try {
      _b.styledPaths(_p, buf, n);
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
  bool moveHighlight(int handle, Object target) {
    final ok = _t(target, (t) => _b.moveHighlight(_p, handle, t)) != 0;
    _touch();
    return ok;
  }

  bool restyleHighlight(int handle, QvpHighlightStyle style) {
    final ok = _b.restyleHighlight(_p, handle, _e._writeHl(style)) != 0;
    _touch();
    return ok;
  }

  /// Fades out over the highlight's transition.
  bool removeHighlight(int handle) {
    final ok = _b.removeHighlight(_p, handle) != 0;
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
  List<QvpBox> highlightBoxesView() => _boxes(_b.highlightBoxesView(_p, _e._out<QvpBoxC>(), _e._cap(ffi.sizeOf<QvpBoxC>())));

  /// Static band boxes for a word list (no highlight state involved).
  List<QvpBox> wordBands(List<int> ws, {String height = 'lineSpacing', double padX = QvpDefaults.highlightPadX, double padY = QvpDefaults.highlightPadY}) => _e.withU32(ws, (p, n) => _boxes(_b.wordBands(_p, p, n, height == 'ink' ? 1 : 0, padX, padY, _e._out<QvpBoxC>(), _e._cap(ffi.sizeOf<QvpBoxC>()))));

  // ── selection ──
  void select(int anchor, [int? focus]) {
    _b.select(_p, anchor < 0 ? qvpNone : anchor, (focus ?? anchor) < 0 ? qvpNone : (focus ?? anchor));
    _touch();
  }

  void clearSelection() => select(-1, -1);
  List<int> selection() => _u32(_b.selection(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));
  String selectionText([Object form = 'rasmUthmani', bool includeCitation = false]) {
    _b.selectionText(_p, QvpForm.of(form), includeCitation ? 1 : 0, _e._str);
    return _e._s();
  }

  // ── memorisation ──
  static int _maskMode(String m) => const {'hide': 0, 'block': 1, 'blur': 2}[m] ?? 0;

  /// Masks a target. [mode]: 'hide' | 'block' | 'blur'.
  void mask(Object target, [String mode = 'hide']) {
    _t(target, (t) => _b.mask(_p, t, _maskMode(mode)));
    _touch();
  }

  void maskFrom(int wordIndex, [String mode = 'hide']) {
    _b.maskFrom(_p, wordIndex, _maskMode(mode));
    _touch();
  }

  void maskOptions({Object blockColor = QvpDefaults.maskBlock, double padX = QvpDefaults.maskPad, double padY = QvpDefaults.maskPad, double radius = QvpDefaults.maskRadius, bool reverse = false}) {
    _b.maskOptions(_p, rgba(blockColor), padX, padY, radius, reverse ? 1 : 0);
    _touch();
  }

  int unmaskNext([int n = 1]) {
    final r = _b.unmaskNext(_p, n);
    _touch();
    return r;
  }

  int maskBack([int n = 1]) {
    final r = _b.maskBack(_p, n);
    _touch();
    return r;
  }

  bool unmaskWord(int wordIndex) {
    final r = _b.unmaskWord(_p, wordIndex) != 0;
    _touch();
    return r;
  }

  bool maskWord(int wordIndex) {
    final r = _b.maskWord(_p, wordIndex) != 0;
    _touch();
    return r;
  }

  void unmaskAll() {
    _b.unmaskAll(_p);
    _touch();
  }

  void maskAll() {
    _b.maskAll(_p);
    _touch();
  }

  void unmask() {
    _b.unmask(_p);
    _touch();
  }

  List<int> maskHidden() => _u32(_b.maskHidden(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));
  List<int> maskWords() => _u32(_b.maskWords(_p, _e._out<ffi.Uint32>(), _e._cap(ffi.sizeOf<ffi.Uint32>())));

  /// Block / blur boxes in layout viewport px (drawn last).
  List<QvpBox> maskBoxesView() => _boxes(_b.maskBoxesView(_p, _e._out<QvpBoxC>(), _e._cap(ffi.sizeOf<QvpBoxC>())));

  /// Greyed page with a lit window → steps.
  int revealStart({int lit = QvpDefaults.revealLit, bool byAyah = false, Object grey = QvpDefaults.revealGrey, Object ink = QvpDefaults.ink, bool ayahMarks = true, int ms = 0}) {
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
  int? revealPosition() {
    final v = _b.revealPosition(_p);
    return v == -2 ? null : v;
  }

  int revealStepCount() => _b.revealStepCount(_p);
  int revealStepOf(int wordIndex) => _b.revealStepOf(_p, wordIndex);
  void revealStop() {
    _b.revealStop(_p);
    _touch();
  }

  // ── dress: another mushaf's ornaments ──

  QvpDressSpec? _dressSpec;
  List<QvpDressDraw>? _dressDraws;
  int _dressDrawsRevision = -1;

  /// The spec the page is dressed with, or null. A view refits when it changes.
  QvpDressSpec? get dressSpec => _dressSpec;

  /// Puts other ornaments on the page, up to one per kind, and returns the readout. Returns
  /// null when the engine refuses the spec: an index out of range, an ornament of another
  /// kind, or nothing to dress. Replaces any previous dress. The printed rings and bands under
  /// the new ornaments are hidden.
  QvpDressInfo? dress(QvpOrnaments set, [QvpDressSpec spec = const QvpDressSpec()]) {
    // Resolve every name before the struct is written.
    int index(Object? v, QvpOrnamentKind kind) {
      final o = v ?? spec.style;
      if (o == null) return qvpNone;
      if (o is int) return o;
      if (o is QvpOrnament) return o.index;
      return set.find(kind, o as String)?.index ?? qvpNone;
    }

    final ayahMark = index(spec.ayahMark, QvpOrnamentKind.ayahMark);
    final surahHeader = index(spec.surahHeader, QvpOrnamentKind.surahHeader);
    final pageFrame = index(spec.pageFrame, QvpOrnamentKind.pageFrame);
    final triples = <int>[
      for (final (o, part, colour) in spec.colors) ...[o is QvpOrnament ? o.index : o as int, part, rgba(colour)],
    ];
    var lineArt = 0;
    for (final k in spec.lineArt) {
      lineArt |= 1 << k.index;
    }
    final c = pffi.calloc<QvpDressSpecC>();
    try {
      return _e.withU32(triples, (cp, n) {
        final r = c.ref;
        r.ayahMark = ayahMark;
        r.surahHeader = surahHeader;
        r.pageFrame = pageFrame;
        r.gap = spec.gap;
        r.bandGap = spec.bandGap;
        r.ayahMarkScale = spec.ayahMarkScale;
        r.pageFrameScale = spec.pageFrameScale;
        r.lineArt = lineArt;
        r.colors = n == 0 ? ffi.nullptr : cp;
        r.nColors = n ~/ 3;
        _dressDraws = null;
        final ok = _b.dress(_p, set._s, c) != 0;
        _dressSpec = ok ? spec : null;
        _touch();
        return ok ? dressInfo() : null;
      });
    } finally {
      pffi.calloc.free(c);
    }
  }

  /// Takes the ornaments off. The printed rings and bands come back.
  void undress() {
    _b.undress(_p);
    _dressSpec = null;
    _dressDraws = null;
    _touch();
  }

  /// Holds [view] so a drag reaches the border of a dressed page and never goes past it. An
  /// undressed page gets the plain clamp. The viewport is in the same px as the layout's.
  QvpView dressClamp(QvpView view, double viewportW, double viewportH) {
    final v = pffi.calloc<QvpViewC>(), out = pffi.calloc<QvpViewC>();
    try {
      _writeView(v, view);
      _b.dressClamp(_p, v, viewportW, viewportH, out);
      return _readView(out.ref);
    } finally {
      pffi.calloc.free(v);
      pffi.calloc.free(out);
    }
  }

  /// For a host that scrolls the page itself: the view that shows the dressed box at [scale],
  /// centred across [viewportW] (at x = 0 when wider) with its top at 0, and the box's size at
  /// that scale, which is the size to give the canvas. An undressed page gets its content
  /// centred at the top and the content's size.
  QvpDressView dressView(double scale, double viewportW) {
    final out = pffi.calloc<QvpViewC>(), size = pffi.calloc<ffi.Float>(2);
    try {
      _b.dressView(_p, scale, viewportW, out, size);
      return QvpDressView(view: _readView(out.ref), width: size[0], height: size[1]);
    } finally {
      pffi.calloc.free(out);
      pffi.calloc.free(size);
    }
  }

  /// The readout, or null when the page is not dressed.
  QvpDressInfo? dressInfo() {
    final o = pffi.calloc<QvpDressInfoC>();
    try {
      if (_b.dressInfo(_p, o) == 0) return null;
      final i = o.ref;
      int? id(int v) => v == qvpNone ? null : v;
      return QvpDressInfo(
        ayahMark: id(i.ayahMark),
        surahHeader: id(i.surahHeader),
        pageFrame: id(i.pageFrame),
        ayahMarks: i.nAyahMarks,
        surahHeaders: i.nSurahHeaders,
        frameRepeats: i.nFrameRepeats,
        frameStretched: i.frameStretched != 0,
        draws: i.nDraws,
        revision: i.revision,
        viewBox: List<double>.generate(4, (k) => i.viewBox[k], growable: false),
        overflow: List<double>.generate(4, (k) => i.overflow[k], growable: false),
      );
    } finally {
      pffi.calloc.free(o);
    }
  }

  /// Counts up whenever the ornament layer is rebuilt: a new dress, or a layout. 0 when the
  /// page is not dressed.
  int dressRevision() => dressInfo()?.revision ?? 0;

  /// The ornament display list in page units. Paint it behind the page ink, each draw under
  /// its [QvpDressDraw.placement] the way a page draw is placed. Kept until the revision
  /// changes.
  List<QvpDressDraw> dressGeometry() {
    final rev = dressRevision();
    final kept = _dressDraws;
    if (kept != null && _dressDrawsRevision == rev) return kept;
    final g = pffi.calloc<QvpDressGeometryC>();
    try {
      _b.dressGeometry(_p, g);
      final r = g.ref;
      final n = r.nDraws;
      if (n == 0 || r.table == ffi.nullptr) {
        _dressDrawsRevision = rev;
        return _dressDraws = const [];
      }
      // Copied at once: the engine's buffers last until the next dress, undress or layout.
      final ops = Uint8List.fromList(r.ops.asTypedList(r.opsLen));
      final pts = Float32List.fromList(r.pts.asTypedList(r.ptsLen));
      final t = Uint32List.fromList(r.table.asTypedList(n * 9));
      final tf = t.buffer.asFloat32List();
      final out = List<QvpDressDraw>.generate(n, (i) {
        final flags = t[i * 9 + 5], placement = t[i * 9 + 8];
        final evenOdd = flags & 0x100 != 0;
        final path = Path()..fillType = evenOdd ? PathFillType.evenOdd : PathFillType.nonZero;
        var o = t[i * 9];
        final oe = o + t[i * 9 + 1];
        var k = t[i * 9 + 2];
        for (; o < oe; o++) {
          switch (ops[o]) {
            case QvpOp.moveTo:
              path.moveTo(pts[k], pts[k + 1]);
              k += 2;
            case QvpOp.lineTo:
              path.lineTo(pts[k], pts[k + 1]);
              k += 2;
            case QvpOp.quadTo:
              path.quadraticBezierTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3]);
              k += 4;
            case QvpOp.cubicTo:
              path.cubicTo(pts[k], pts[k + 1], pts[k + 2], pts[k + 3], pts[k + 4], pts[k + 5]);
              k += 6;
            case QvpOp.close:
              path.close();
          }
        }
        return QvpDressDraw(
          path: path,
          color: t[i * 9 + 4],
          kind: QvpOrnamentKind.values[(flags & 0xff).clamp(0, 2)],
          evenOdd: evenOdd,
          stroke: flags & 0x200 != 0,
          strokeWidth: tf[i * 9 + 6],
          part: t[i * 9 + 7],
          placement: placement == qvpNone ? null : placement,
        );
      }, growable: false);
      _dressDrawsRevision = rev;
      return _dressDraws = out;
    } finally {
      pffi.calloc.free(g);
    }
  }

  // ── crop ──
  QvpCropBounds? cropBounds(Object target, {double pad = QvpDefaults.cropPad, bool keepAyahMarks = true}) {
    final ok = _t(target, (t) => _b.cropBounds(_p, t, pad, keepAyahMarks ? 1 : 0, _e._crop));
    if (ok == 0) return null;
    final c = _e._crop.ref;
    return QvpCropBounds(x0: c.x0, y0: c.y0, x1: c.x1, y1: c.y1, nWords: c.nWords, ayahMarkDecoration: c.ayahMarkDecoration);
  }

  /// Standalone SVG of a target (current colours), or null.
  String? cropSvg(Object target, {double pad = QvpDefaults.cropPad, bool keepAyahMarks = true, Object? background}) {
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

  int get pages => _b.atlasPageCount(_a);

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
        number: s.number,
        page: s.firstPage,
        ayahCount: s.ayahCount,
        place: engine.placeName(s.place),
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

  /// Start of division [n] of [division] ('juz' | 'hizb' | 'nisf' | 'rubuAlHizb' or [QvpDiv]).
  QvpAtlasRubuAlHizb? division(Object division, int n) {
    final o = pffi.calloc<QvpAtlasRubuAlHizbC>();
    try {
      if (_b.atlasDivision(_a, QvpDiv.of(division), n, o) == 0) return null;
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
  int? divisionOf(Object division, int surah, int ayah) {
    final v = _b.atlasDivisionOf(_a, QvpDiv.of(division), surah, ayah);
    return v < 0 ? null : v;
  }

  int? juzOf(int surah, int ayah) => divisionOf('juz', surah, ayah);

  /// `[first, last]` page of a juz, or null.
  List<int>? pagesOfJuz(int n) {
    final o = engine._out<ffi.Uint16>();
    if (_b.atlasPagesOfJuz(_a, n, o) == 0) return null;
    final v = o.asTypedList(2);
    return [v[0], v[1]];
  }

  /// Surahs matching a (partial, any-script) name.
  List<QvpAtlasSurah> searchSurahs(String text) {
    final o = engine._out<ffi.Uint16>(), cap = engine._cap(ffi.sizeOf<ffi.Uint16>());
    final n = engine.withString(text, (p, len) => _b.atlasSearchSurahs(_a, p, len, o, cap)).clamp(0, cap);
    final ids = List<int>.from(o.asTypedList(n));
    return [for (final k in ids) surah(k)].whereType<QvpAtlasSurah>().toList(growable: false);
  }

  String json() {
    _b.atlasJson(_a, engine._str);
    return engine._s();
  }
}

// ───────────── ornaments ─────────────

/// The ornaments of other mushafs and fonts, from `ornaments.qvo`. None of these outlines is
/// part of a QVP page. Each ornament says what it may be redistributed under: read
/// [QvpOrnament.licence] before you publish a page wearing it. Free it after the pages
/// dressed with it.
class QvpOrnaments {
  QvpOrnaments._(this.engine, this._h) {
    count = _b.ornamentCount(_h);
    list = List<QvpOrnament>.generate(count, _ornament, growable: false);
  }

  final QvpEngine engine;
  ffi.Pointer<QvpOrnamentsC> _h;
  QvpBindings get _b => engine.b;

  ffi.Pointer<QvpOrnamentsC> get _s {
    assert(_h != ffi.nullptr, 'QvpOrnaments used after dispose()');
    return _h;
  }

  /// How many ornaments the set holds.
  late final int count;

  /// Every ornament of the set, in its order.
  late final List<QvpOrnament> list;

  bool get isDisposed => _h == ffi.nullptr;

  void free() {
    if (_h != ffi.nullptr) {
      _b.ornamentsFree(_h);
      _h = ffi.nullptr;
    }
  }

  void dispose() => free();

  QvpOrnament _ornament(int i) {
    final o = pffi.calloc<QvpOrnamentC>();
    final p = pffi.calloc<QvpOrnamentPartC>();
    try {
      if (_b.ornamentInfo(_s, i, o) == 0) throw StateError('qvp_ornament_info($i) failed');
      final r = o.ref;
      final parts = <QvpOrnamentPart>[];
      for (var k = 0; k < r.nParts; k++) {
        if (_b.ornamentPart(_s, i, k, p) == 0) continue;
        parts.add(QvpOrnamentPart(index: k, color: p.ref.color, stroke: p.ref.stroke != 0, name: QvpEngine.str(p.ref.name)));
      }
      return QvpOrnament(
        index: i,
        kind: QvpOrnamentKind.values[r.kind.clamp(0, 2)],
        lineage: QvpLineage.values[r.lineage.clamp(0, 1)],
        style: QvpEngine.str(r.style),
        riwayah: QvpEngine.str(r.riwayah),
        hasSlices: r.hasSlices != 0,
        licence: QvpOrnamentLicence(id: QvpEngine.str(r.licenseId), status: QvpEngine.str(r.licenseStatus), redistributable: r.redistributable != 0, attribution: QvpEngine.str(r.attribution)),
        parts: List.unmodifiable(parts),
      );
    } finally {
      pffi.calloc.free(o);
      pffi.calloc.free(p);
    }
  }

  /// The ornament of [kind] in [style], or null.
  QvpOrnament? find(QvpOrnamentKind kind, String style) {
    final i = engine.withString(style, (p, n) => _b.ornamentFind(_s, kind.index, p, n));
    return i < 0 || i >= list.length ? null : list[i];
  }

  /// The ornaments of one kind.
  List<QvpOrnament> ofKind(QvpOrnamentKind kind) => [for (final o in list) if (o.kind == kind) o];

  /// Every style in the set, in the set's order, with the ornament it has of each kind.
  List<QvpOrnamentStyle> styles() {
    final order = <String>[];
    final byStyle = <String, Map<QvpOrnamentKind, QvpOrnament>>{};
    final first = <String, QvpOrnament>{};
    for (final o in list) {
      if (!byStyle.containsKey(o.style)) {
        order.add(o.style);
        byStyle[o.style] = {};
        first[o.style] = o;
      }
      byStyle[o.style]![o.kind] = o;
    }
    return [
      for (final s in order)
        QvpOrnamentStyle(
          style: s,
          riwayah: first[s]!.riwayah,
          lineage: first[s]!.lineage,
          ayahMark: byStyle[s]![QvpOrnamentKind.ayahMark],
          surahHeader: byStyle[s]![QvpOrnamentKind.surahHeader],
          pageFrame: byStyle[s]![QvpOrnamentKind.pageFrame],
        ),
    ];
  }
}

// ───────────── passage ─────────────

/// Where the rows of a passage start. The order is `QVP_ALIGN_*`.
enum QvpPassageAlign {
  /// At the right edge, as Arabic text starts.
  right,

  /// In the middle of the width.
  center,

  /// Across the width: every row but the last opens its word gaps until it reaches both edges,
  /// never past [QvpPassageSpec.maxStretch]. What is left over is split between the two sides,
  /// so the last row, a one-word row and a capped row are centred.
  justified,
}

/// What a layout of a passage asks for. Lengths are layout px.
@immutable
final class QvpPassageSpec {
  /// The defaults put the passage on one row at the printed size.
  const QvpPassageSpec({
    this.width = 0,
    this.scale = 1,
    this.lineSpacing = 1,
    this.padding = 2,
    this.align = QvpPassageAlign.right,
    this.maxRows = 0,
    this.keepAyahMark = false,
    this.ellipsisWidth = 0,
    this.maxStretch = QvpDefaults.reflowMaxStretch,
  });

  /// The width of the rows. Zero or less puts the whole passage on one row, as wide as it is.
  final double width;

  /// Layout px per page unit. It shrinks only when one word and its signs are wider than the
  /// rows. To size a passage by its printed line, divide by [QvpPassage.lineSpacing].
  final double scale;

  /// The distance between two rows' baselines, as a multiple of the printed one. At least 1.
  final double lineSpacing;

  /// The empty margin on every side.
  final double padding;

  /// Where each row starts.
  final QvpPassageAlign align;

  /// The most rows to show. Zero shows every row.
  final int maxRows;

  /// After a cut, place the medallion of the passage's last ayah after the ellipsis.
  final bool keepAyahMark;

  /// The width that the host needs for its ellipsis after a cut.
  final double ellipsisWidth;

  /// How far a justified row's gaps may open, as a multiple of the passage's usual air between
  /// two words. A negative value lifts the cap.
  final double maxStretch;

  @override
  bool operator ==(Object other) =>
      other is QvpPassageSpec &&
      other.width == width &&
      other.scale == scale &&
      other.lineSpacing == lineSpacing &&
      other.padding == padding &&
      other.align == align &&
      other.maxRows == maxRows &&
      other.keepAyahMark == keepAyahMark &&
      other.ellipsisWidth == ellipsisWidth &&
      other.maxStretch == maxStretch;

  @override
  int get hashCode => Object.hash(width, scale, lineSpacing, padding, align, maxRows, keepAyahMark, ellipsisWidth, maxStretch);
}

/// One ayah of a passage, with its text from the page's own word records.
@immutable
final class QvpPassageText {
  const QvpPassageText({required this.surah, required this.ayah, required this.text});
  final int surah, ayah;
  final String text;

  /// `surah:ayah`
  String get ayahKey => '$surah:$ayah';
}

/// One row of a laid-out passage, in layout px: the box of the row's ink, and the height its
/// words sit on.
@immutable
final class QvpPassageRow {
  const QvpPassageRow({required this.x0, required this.y0, required this.x1, required this.y1, required this.baseline});
  final double x0, y0, x1, y1, baseline;
}

/// One word of a laid-out passage with the signs that go with it, in layout px.
@immutable
final class QvpPassageWord {
  const QvpPassageWord({required this.surah, required this.ayah, required this.word, required this.page, required this.row, required this.x0, required this.y0, required this.x1, required this.y1,
      required this.wordX0, required this.wordY0, required this.wordX1, required this.wordY1});

  /// The word's key and the page it is printed on.
  final int surah, ayah, word, page;

  /// The row the word is on, from 0.
  final int row;

  /// The box of the word and its signs.
  final double x0, y0, x1, y1;

  /// The box of the word alone, without its signs: the word that closes an ayah leaves its
  /// medallion out.
  final double wordX0, wordY0, wordX1, wordY1;

  /// `surah:ayah:word`
  String get wordKey => '$surah:$ayah:$word';
}

/// The bounds of one ayah that a layout shows, in layout px.
@immutable
final class QvpPassageAyah {
  const QvpPassageAyah({required this.surah, required this.ayah, required this.x0, required this.y0, required this.x1, required this.y1});
  final int surah, ayah;
  final double x0, y0, x1, y1;
}

/// One path that a passage layout draws: its page, its index on that page, and which of
/// [QvpPassageLayout.placements] it goes under.
@immutable
final class QvpPassageDraw {
  const QvpPassageDraw(this.page, this.path, this.placement);

  /// The page number. Draw the path with that page's geometry.
  final int page;

  /// The path's index on its page, and the index of its placement.
  final int path, placement;
}

/// A laid-out passage. A point `(x, y)` of a path draws at
/// `scale · (kx · x + dx), scale · (ky · y + dy)`, with the path's placement.
@immutable
final class QvpPassageLayout {
  const QvpPassageLayout({
    required this.width,
    required this.height,
    required this.scale,
    required this.lineSpacing,
    required this.rows,
    required this.words,
    required this.ayahs,
    required this.draws,
    required this.placements,
    required this.isTruncated,
    required this.ellipsis,
  });

  /// The size of the layout, padding included, and the scale it was laid out at.
  final double width, height, scale;

  /// The distance between two rows' baselines, page units, before a row moves down for its ink.
  final double lineSpacing;

  /// The rows, top to bottom.
  final List<QvpPassageRow> rows;

  /// The words shown, in reading order.
  final List<QvpPassageWord> words;

  /// The ayahs with at least one word shown.
  final List<QvpPassageAyah> ayahs;

  /// Every path to draw, in drawing order.
  final List<QvpPassageDraw> draws;

  /// What [QvpPassageDraw.placement] indexes.
  final List<QvpPlacement> placements;

  /// The layout shows fewer words than the passage has.
  final bool isTruncated;

  /// Where the host draws its ellipsis after a cut: from the last row's top to its baseline.
  final Rect? ellipsis;
}

/// A contiguous range of complete ayahs, or a surah's basmalah, ready to lay out at any width.
/// It copies what it needs from its pages, so the pages can be disposed after it is made.
class QvpPassage {
  QvpPassage._(this.engine, this._h, int surah, int from) {
    final b = engine.b;
    ayahs = List.generate(b.passageAyahCount(_h), (i) {
      b.passageAyahText(_h, i, engine._str);
      return QvpPassageText(surah: surah, ayah: from + i, text: engine._s());
    }, growable: false);
    lineSpacing = b.passageLineSpacing(_h);
  }

  /// Loads the ayahs [from] to [to] of [surah] from the pages that print them, in any order.
  /// Throws when an ayah is missing or incomplete, a page is given twice, or the range has more
  /// than 4,096 words.
  factory QvpPassage.load(List<QvpPage> pages, int surah, int from, {int? to}) {
    final last = to ?? from;
    _checkU16(surah, 'surah');
    _checkU16(from, 'from');
    _checkU16(last, 'to');
    final engine = _engineOf(pages);
    final h = _withPages(pages, (p, n) => engine.b.passageLoad(p, n, surah, from, last));
    if (h == ffi.nullptr) throw const FormatException('qvp_passage_load failed');
    return QvpPassage._(engine, h, surah, from);
  }

  /// Loads the basmalah printed above [surah] from whichever of the pages prints it. It lays out
  /// as one piece on one row and has no ayahs and no words. Its line spacing is its page's, so
  /// it is sized by the printed line as an ayah is. Throws for surah 1, whose basmalah is its
  /// first ayah, for surah 9, which has none, and when no page given prints it.
  factory QvpPassage.basmalah(List<QvpPage> pages, int surah) {
    _checkU16(surah, 'surah');
    final engine = _engineOf(pages);
    final h = _withPages(pages, (p, n) => engine.b.passageLoadBasmalah(p, n, surah));
    if (h == ffi.nullptr) throw const FormatException('qvp_passage_load_basmalah failed');
    return QvpPassage._(engine, h, surah, 1);
  }

  static void _checkU16(int v, String name) {
    if (v < 1 || v > 0xffff) throw RangeError.range(v, 1, 0xffff, name);
  }

  static QvpEngine _engineOf(List<QvpPage> pages) {
    if (pages.isEmpty) throw ArgumentError.value(pages, 'pages', 'a passage needs at least one page');
    // Every engine call on a disposed page is undefined. The engine checks the range itself.
    for (final p in pages) {
      if (p.isDisposed) throw StateError('QvpPassage: a page was disposed');
    }
    return pages.first.engine;
  }

  /// Runs [f] with the pages' handles in one native array.
  static R _withPages<R>(List<QvpPage> pages, R Function(PtrPages p, int n) f) {
    final arr = pffi.malloc<PtrPage>(pages.length);
    try {
      for (var i = 0; i < pages.length; i++) {
        arr[i] = pages[i]._p;
      }
      return f(arr, pages.length);
    } finally {
      pffi.malloc.free(arr);
    }
  }

  final QvpEngine engine;
  ffi.Pointer<QvpPassageC> _h;

  /// The ayahs in reading order. Empty for a basmalah.
  late final List<QvpPassageText> ayahs;

  /// The printed line spacing of the passage's first word, in page units. Every word of the
  /// passage has that size.
  late final double lineSpacing;

  /// The last layout, if any.
  QvpPassageLayout? currentLayout;

  bool get isDisposed => _h == ffi.nullptr;

  ffi.Pointer<QvpPassageC> get _p {
    assert(_h != ffi.nullptr, 'QvpPassage used after close()');
    return _h;
  }

  QvpBindings get _b => engine.b;
  QvpEngine get _e => engine;

  /// Frees the native passage. Safe to call more than once. [ayahs], [lineSpacing] and
  /// [currentLayout] stay usable.
  void close() {
    if (_h != ffi.nullptr) {
      _b.passageFree(_h);
      _h = ffi.nullptr;
    }
  }

  /// The same as [close], under the name every other object of this package frees with.
  void dispose() => close();

  /// Lays the passage out. Null when the spec is out of range. The last layout then stays.
  QvpPassageLayout? layout(QvpPassageSpec spec) {
    if (spec.maxRows < 0) return null;
    final s = pffi.calloc<QvpPassageSpecC>(), o = pffi.calloc<QvpPassageLayoutC>();
    try {
      final r = s.ref;
      r.width = spec.width;
      r.scale = spec.scale;
      r.lineSpacing = spec.lineSpacing;
      r.padding = spec.padding;
      r.align = spec.align.index;
      r.keepAyahMark = spec.keepAyahMark ? 1 : 0;
      r.maxRows = spec.maxRows;
      r.ellipsisWidth = spec.ellipsisWidth;
      r.maxStretch = spec.maxStretch;
      if (_b.passageLayout(_p, s, o) == 0) return null;
      final l = o.ref;
      final layout = QvpPassageLayout(
        width: l.width,
        height: l.height,
        scale: l.scale,
        lineSpacing: l.lineSpacing,
        rows: _rows(),
        words: _words(),
        ayahs: _ayahs(),
        draws: _draws(l.nDraws),
        placements: _placements(),
        isTruncated: l.isTruncated != 0,
        ellipsis: l.isTruncated != 0 ? Rect.fromLTRB(l.ellipsisX0, l.ellipsisY0, l.ellipsisX1, l.ellipsisY1) : null,
      );
      currentLayout = layout;
      return layout;
    } finally {
      pffi.calloc.free(s);
      pffi.calloc.free(o);
    }
  }

  // The rows, words and ayahs fit the engine's scratch: a passage holds at most 4,096 words.
  List<QvpPassageRow> _rows() {
    final o = _e._out<QvpPassageRowC>(), cap = _e._cap(ffi.sizeOf<QvpPassageRowC>());
    final n = _b.passageRows(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final r = (o + i).ref;
      return QvpPassageRow(x0: r.x0, y0: r.y0, x1: r.x1, y1: r.y1, baseline: r.baseline);
    }, growable: false);
  }

  List<QvpPassageWord> _words() {
    final o = _e._out<QvpPassageWordC>(), cap = _e._cap(ffi.sizeOf<QvpPassageWordC>());
    final n = _b.passageWords(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final w = (o + i).ref;
      return QvpPassageWord(surah: w.surah, ayah: w.ayah, word: w.word, page: w.page, row: w.row, x0: w.x0, y0: w.y0, x1: w.x1, y1: w.y1,
          wordX0: w.wordX0, wordY0: w.wordY0, wordX1: w.wordX1, wordY1: w.wordY1);
    }, growable: false);
  }

  List<QvpPassageAyah> _ayahs() {
    final o = _e._out<QvpPassageAyahC>(), cap = _e._cap(ffi.sizeOf<QvpPassageAyahC>());
    final n = _b.passageAyahs(_p, o, cap).clamp(0, cap);
    return List.generate(n, (i) {
      final a = (o + i).ref;
      return QvpPassageAyah(surah: a.surah, ayah: a.ayah, x0: a.x0, y0: a.y0, x1: a.x1, y1: a.y1);
    }, growable: false);
  }

  // A long passage draws more paths than the scratch holds, so the draw list gets its own
  // buffer, sized by the count the layout reported.
  List<QvpPassageDraw> _draws(int count) {
    final buf = pffi.malloc<ffi.Uint32>(count < 1 ? 3 : count * 3);
    try {
      final n = _b.passageDrawList(_p, buf, count).clamp(0, count);
      final f = buf.asTypedList(n * 3);
      return List.generate(n, (i) => QvpPassageDraw(f[i * 3], f[i * 3 + 1], f[i * 3 + 2]), growable: false);
    } finally {
      pffi.malloc.free(buf);
    }
  }

  List<QvpPlacement> _placements() {
    final o = _e._out<ffi.Float>(), cap = _e._cap(16);
    final total = _b.passagePlacements(_p, o, cap);
    if (total <= cap) return _readPlacements(o, total);
    // More placements than the scratch holds: ask again into a buffer of the right size.
    final buf = pffi.malloc<ffi.Float>(total * 4);
    try {
      return _readPlacements(buf, _b.passagePlacements(_p, buf, total).clamp(0, total));
    } finally {
      pffi.malloc.free(buf);
    }
  }

  static List<QvpPlacement> _readPlacements(ffi.Pointer<ffi.Float> p, int n) {
    final f = p.asTypedList(n * 4);
    return List.generate(n, (i) => QvpPlacement(f[i * 4], f[i * 4 + 1], f[i * 4 + 2], f[i * 4 + 3]), growable: false);
  }
}
