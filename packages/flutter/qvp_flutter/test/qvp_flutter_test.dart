// dart:ffi tests against the host build of libqvp_ffi.
//
// Library: $QVP_LIB, else <repo>/target/release/libqvp_ffi.so
// (build with `cargo build -p qvp-ffi --release`). On Windows the probe also
// accepts <repo>/target/dist/qvp_ffi.dll and the plugin directory's copy.
// Data: <repo>/dist/pages/042.qvp, 042.words.json, atlas.qva
// (`cargo run -p qvp-convert --release -- batch pages dist/pages`).
import 'dart:convert';
import 'dart:math' as math;
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qvp_flutter/qvp_flutter.dart';

/// The repository root: `QVP_REPO` if set, else the nearest ancestor of the working
/// directory that holds the engine header.
String _findRepo() {
  final env = Platform.environment['QVP_REPO'];
  if (env != null) return env;
  var d = Directory.current;
  for (var i = 0; i < 6; i++) {
    if (File('${d.path}/crates/qvp-ffi/include/qvp.h').existsSync()) {
      return d.path;
    }
    d = d.parent;
  }
  throw StateError('repository root not found; set QVP_REPO');
}

/// The first path that exists, else the last one.
String _firstExisting(List<String> paths) =>
    paths.firstWhere((p) => File(p).existsSync(), orElse: () => paths.last);

void main() {
  final repo = _findRepo();
  final libPath =
      Platform.environment['QVP_LIB'] ??
      (Platform.isWindows
          ? _firstExisting([
              '$repo/target/dist/qvp_ffi.dll',
              '$repo/target/release/qvp_ffi.dll',
              '$repo/packages/flutter/qvp_flutter/windows/qvp_ffi.dll',
            ])
          : Platform.isMacOS
          ? '$repo/target/release/libqvp_ffi.dylib'
          : '$repo/target/release/libqvp_ffi.so');
  final pagePath = '$repo/dist/pages/042.qvp';
  final wordsPath = '$repo/dist/pages/042.words.json';
  final atlasPath = '$repo/dist/pages/atlas.qva';

  late QvpEngine engine;
  late QvpPage page;

  setUpAll(() {
    expect(
      File(libPath).existsSync(),
      isTrue,
      reason: 'engine library not found at $libPath',
    );
    expect(
      File(pagePath).existsSync(),
      isTrue,
      reason: 'page not found at $pagePath',
    );
    engine = QvpEngine.open(path: libPath);
    page = engine.loadPage(File(pagePath).readAsBytesSync());
  });

  tearDownAll(() {
    page.dispose();
    engine.dispose();
  });

  test('engine: version, names, arabic tools', () {
    expect(engine.formatVersion, greaterThan(0));
    expect(engine.version, matches(RegExp(r'^\d+\.\d+\.\d+$')));
    expect(engine.engineName, 'qvp');
    expect(engine.kindName(QvpKind.mark), 'mark');
    expect(engine.markName(1), 'fathah');
    expect(engine.markName(7), engine.markName(QvpMark.id('shaddah')));
    expect(engine.familyName(QvpFamily.diacritic), isNotEmpty);
    expect(engine.categoryName(QvpCategory.harakah), isNotEmpty);
    expect(engine.strip('بِسْمِ'), 'بسم');
    expect(engine.normalize('ٱللَّهِ'), isNotEmpty);
    expect(engine.looseKey('الله'), isNotEmpty);
    expect(rgba('#1a73e8'), 0x1a73e8ff);
    expect(rgba('#d6a326', 0.3), 0xd6a3264d);
    // alpha rounds to the nearest step: 0.5 is 0x80 in every wrapper
    expect(rgba('#1a73e8', 0.5), 0x1a73e880);
    expect(QvpColor.withAlpha(0x1a73e8ff, 0.5), 0x1a73e880);
    // a NaN alpha is 0 and an out-of-range one clamps, in every wrapper
    expect(rgba('#1a73e8', double.nan), 0x1a73e800);
    expect(QvpColor.withAlpha(0x1a73e800, 2), 0x1a73e8ff);
    expect(QvpColor.fromColor(QvpColor.toColor(0x1a73e8cc)), 0x1a73e8cc);
  });

  test('page 042: 147 words / 1061 paths / 15 lines, geometry copied', () {
    expect(page.page, 42);
    expect(page.nWords, 147);
    expect(page.nPaths, 1061);
    expect(page.nLines, 15);
    expect(page.width, closeTo(345, 0.01));
    expect(page.height, closeTo(550, 0.01));
    expect(page.words.length, 147);
    expect(page.lines.length, 15);
    expect(page.ayahs.length, page.nAyahs);
    expect(page.decorations.length, page.nDecorations);
    expect(page.table.length, 1061 * 8);
    for (var i = 0; i < page.nPaths; i++) {
      expect(
        page.pathOpStart(i) + page.pathOpCount(i),
        lessThanOrEqualTo(page.ops.length),
      );
      expect(
        page.pathPtStart(i) + page.pathPtCount(i),
        lessThanOrEqualTo(page.pts.length),
      );
      expect(page.pathLine(i), lessThan(page.nLines));
    }
    final w0 = page.words[0];
    expect(w0.text, isNotEmpty);
    expect(w0.nPaths, greaterThan(0));
    expect(page.findWord(w0.surah, w0.ayah, w0.word), 0);
    expect(page.wordKey(0), w0.wordKey);
    expect(page.lineSpacing, greaterThan(0));
    expect(page.surahs().map((s) => s.number), contains(2));
    expect(page.ayahKeys(), contains((2, 255)));
    expect(page.wordLabel(0), isNotEmpty);
    expect(page.ayahLabel(page.words[0].ayahIndex), isNotEmpty);
    expect(page.ayahMarks(), isNotEmpty);
    expect(page.lineBands().length, 15);
    expect(page.hitAreas().length, 147);
    final pageTxt = page.text('page');
    expect(pageTxt, contains(w0.text));
    // two separators go through one scratch buffer; a shared offset would make both of them
    // the line separator and leave no spaces at all
    expect(pageTxt, contains(' '));
    expect(page.text('page', 'rasmUthmani', '_', '|'), contains('_'));
    expect(page.text('page', 'rasmUthmani', '_', '|'), contains('|'));
    // path(i) builds one path and keeps it; buildPaths still returns the whole list
    expect(identical(page.path(0), page.path(0)), isTrue);
    expect(page.buildPaths().length, page.nPaths);
  });

  test('search الله → 7 matches', () {
    final m = page.search('الله');
    expect(m.length, 7);
    expect(m.first.text, isNotEmpty);
    expect(m.first.wordKey, page.wordKey(m.first.word));
  });

  test("targetWords('2:255') → 50 words; citation; ayahWordCount", () {
    final ws = page.targetWords('2:255');
    expect(ws.length, 50);
    expect(page.targetWords(T.ayah(2, 255)), ws);
    expect(page.targetWords(ws), ws);
    expect(page.targetWords('2:255:1'), [ws.first]);
    expect(page.citation(ws), '2:255');
    expect(page.ayahWordCount(2, 255).count, 50);
    expect(page.text('2:255'), isNotEmpty);
  });

  test('attachWords sidecar adds forms', () {
    if (!File(wordsPath).existsSync()) return;
    final n = page.attachWords(File(wordsPath).readAsStringSync());
    expect(n, greaterThanOrEqualTo(0));
    expect(page.attachWords('not json'), -1);
    expect(page.hasForm('rasmImlai'), isTrue);
    expect(page.wordForm(0, 'rasmImlai'), isNotEmpty);
    expect(page.wordForm(0, 'search'), isNotEmpty);
  });

  test("hitTest at word 0's bbox centre → word 0; exact inside its ink", () {
    final w = page.words[0];
    final cx = (w.x0 + w.x1) / 2, cy = (w.y0 + w.y1) / 2;
    final h = page.hitTest(cx, cy);
    expect(h, isNotNull);
    expect(h!.word, 0);
    expect(h.distance, 0);
    expect(h.wordKey, w.wordKey);
    expect(h.line, w.lineIndex);
    expect(page.hitTestExact(cx, cy)?.word, 0);
    // the bbox centre can fall between glyphs: probe the bbox for a point inside the outline
    QvpHit? inside;
    for (var y = w.y0; y <= w.y1 && inside == null; y += 0.5) {
      for (var x = w.x0; x <= w.x1; x += 0.5) {
        final e = page.hitTest(x, y);
        if (e != null && e.word == 0 && e.isExact) {
          inside = e;
          break;
        }
      }
    }
    expect(inside, isNotNull, reason: 'no point of word 0 is inside its ink');
    expect(inside!.path, greaterThanOrEqualTo(0));
    expect(page.pathWord(inside.path), 0);
    // far outside the page, with a distance cap → nothing
    expect(
      page.hitTest(-500, -500, const QvpHitOptions(maxDistance: 6)),
      isNull,
    );
  });

  test('layout fillHeight 690×1100 pads 50 → scale 2.0, 15 lineDy; hitTestView hits word 0', () {
    final l = page.layout(
      const QvpLayoutSpec(
        viewportW: 690,
        viewportH: 1100,
        padTop: 50,
        padBottom: 50,
        fillHeight: true,
      ),
    );
    expect(l.scale, closeTo(2.0, 1e-5));
    expect(l.lineDy.length, 15);
    expect(l.slots.length, 15);
    expect(page.currentLayout, same(l));
    final w = page.words[0];
    final viewX = l.offsetX + (w.x0 + w.x1) / 2 * l.scale;
    final viewY =
        l.offsetY + ((w.y0 + w.y1) / 2 + l.lineDy[w.lineIndex]) * l.scale;
    final h = page.hitTestView(
      viewX,
      viewY,
      const QvpHitOptions(maxDistance: 6),
    );
    expect(h, isNotNull);
    expect(h!.word, 0);
    final box = page.wordBoundsView(0);
    expect(box.x0, lessThan(viewX));
    expect(box.x1, greaterThan(viewX));
    expect(
      page.layoutLineSpacingToFill(
        const QvpLayoutSpec(viewportW: 600, viewportH: 1000),
      ),
      greaterThanOrEqualTo(1),
    );
    expect(page.grid.lines, 15);
    expect(
      page.layoutWastedFraction(
        const QvpLayoutSpec(viewportW: 600, viewportH: 1000),
      ),
      inInclusiveRange(0, 1),
    );
    expect(
      page.layoutPrintedHeight(
        const QvpLayoutSpec(viewportW: 600, viewportH: 1000),
      ),
      greaterThan(0),
    );
  });

  test(
    'style(Sel.wordMark(w,1), colour) → styledPaths() has exactly 1 path',
    () {
      page.clearStyles();
      expect(page.styledPaths(), isEmpty);
      final w = 0;
      final h = page.style(Sel.wordMark(w, 1), '#ef6c00');
      expect(h, greaterThan(0));
      final s = page.styledPaths();
      expect(s.length, 1);
      expect(s.first.color, 0xef6c00ff);
      expect(page.pathWord(s.first.path), w);
      expect(page.pathNthMark(s.first.path), 1);
      expect(page.colorOf(s.first.path), 0xef6c00ff);
      expect(page.colors()[s.first.path], 0xef6c00ff);
      expect(page.styleHandles(), contains(h));
      expect(page.removeStyle(h), greaterThan(0));
      expect(page.styledPaths(), isEmpty);
      // theme + hide + target styling under handles
      final th = page.theme(
        const QvpTheme(
          diacritics: '#1a73e8',
          dots: '#c62828',
          marks: {'shaddah': '#0a7d32'},
        ),
      );
      expect(page.styledPaths(), isNotEmpty);
      final hh = page.hide(Sel.kind(QvpKind.mark));
      expect(
        page.styledPaths().where((p) => (p.color & 0xff) == 0),
        isNotEmpty,
      );
      page.removeStyle(hh);
      page.removeStyle(th);
      final ht = page.styleTarget('2:255', '#0a7d32', layer: QvpLayer.top);
      expect(page.styledPaths().length, greaterThan(50));
      page.removeStyle(ht);
      expect(page.styledPaths(), isEmpty);
      page.setDefaultColor('#3b2a14');
      expect(page.defaultInk, 0x3b2a14ff);
      expect(page.colors()[0], 0x3b2a14ff);
      page.setDefaultColor('#231f20');
    },
  );

  test("highlight('2:255', both, 200 ms): tick(100) true, 6 boxes, tick(1000) false", () {
    page.clearHighlights();
    final h = page.highlight(
      '2:255',
      const QvpHighlightStyle(mode: 'both', ms: 200),
    );
    expect(h, greaterThan(0));
    expect(page.tick(100), isTrue);
    final boxes = page.highlightBoxesView();
    expect(boxes.length, 6);
    expect(boxes.every((b) => b.id == h), isTrue);
    expect(page.highlightHandles(), [h]);
    expect(page.highlightWords(h).length, 50);
    expect(page.tick(1000), isFalse);
    expect(page.moveHighlight(h, T.word(0)), isTrue);
    expect(
      page.restyleHighlight(h, const QvpHighlightStyle(mode: 'band')),
      isTrue,
    );
    expect(page.removeHighlight(h), isTrue);
    page.tick(5000);
    page.clearHighlights();
    expect(page.highlightHandles(), isEmpty);
    expect(page.wordBands(page.targetWords('2:255')).length, 6);
  });

  test("mask('2:255'): 50 hidden, unmaskNext → 49; reveal steps", () {
    page.mask('2:255');
    expect(page.maskHidden().length, 50);
    expect(page.maskWords().length, 50);
    expect(page.unmaskNext(1), 1);
    expect(page.maskHidden().length, 49);
    expect(page.maskBack(1), 1);
    expect(page.maskHidden().length, 50);
    page.unmask();
    expect(page.maskHidden(), isEmpty);
    page.mask('2:255', 'block');
    expect(page.maskBoxesView(), isNotEmpty);
    page.unmask();
    final steps = page.revealStart(lit: 2, ms: 0);
    expect(steps, greaterThan(0));
    expect(page.revealStepCount(), steps);
    expect(page.revealPosition(), -1);
    expect(page.revealGoto(0), isTrue);
    expect(page.revealPosition(), 0);
    expect(page.revealStepOf(0), greaterThanOrEqualTo(0));
    page.revealStop();
    expect(page.revealPosition(), isNull);
  });

  test('selection', () {
    page.select(0, 3);
    expect(page.selection(), [0, 1, 2, 3]);
    expect(page.selectionText('rasmUthmani', true), contains(':'));
    page.clearSelection();
    expect(page.selection(), isEmpty);
  });

  test("cropSvg('2:255:1') starts with <svg", () {
    final svg = page.cropSvg('2:255:1', background: '#fffdf7');
    expect(svg, isNotNull);
    expect(svg!.startsWith('<svg'), isTrue);
    final box = page.cropBounds('2:255');
    expect(box, isNotNull);
    expect(box!.nWords, 50);
  });

  test('atlas: pageOf(2,255) == 42, pagesOfJuz(30) == [582, 604], searchSurahs(cow)[0].n == 2', () {
    if (!File(atlasPath).existsSync()) return;
    final atlas = engine.loadAtlas(File(atlasPath).readAsBytesSync());
    expect(atlas.pages, 604);
    expect(atlas.pageOf(2, 255), 42);
    expect(atlas.pagesOfJuz(30), [582, 604]);
    final cow = atlas.searchSurahs('cow');
    expect(cow, isNotEmpty);
    expect(cow.first.number, 2);
    expect(atlas.surah(36)!.latin, isNotEmpty);
    expect(atlas.surahs().length, 114);
    expect(atlas.pageOfSurah(36), atlas.surah(36)!.page);
    expect(atlas.juz(30)!.page, 582);
    expect(atlas.juzOf(2, 255), 3);
    expect(atlas.pageRange(42)!.first.$1, 2);
    atlas.dispose();
  });

  test('layout scenarios match the engine (conformance/scenarios/layout.json)', () {
    final json = jsonDecode(
      File('$repo/conformance/scenarios/layout.json').readAsStringSync(),
    ) as Map<String, dynamic>;
    final tolerance = (json['tolerance'] as num).toDouble();
    final cases = json['cases'] as List<dynamic>;
    void close(double a, num b, String what) => expect(
      (a - b).abs() <= tolerance * (b.abs() < 1 ? 1 : b.abs()),
      isTrue,
      reason: '$what: got $a, engine says $b',
    );
    for (final c in cases) {
      final s = c['spec'] as Map<String, dynamic>,
          want = c['layout'] as Map<String, dynamic>;
      double f(String k) => (s[k] as num).toDouble();
      final spec = QvpLayoutSpec(
        viewportW: f('viewportW'),
        viewportH: f('viewportH'),
        padTop: f('padTop'),
        padBottom: f('padBottom'),
        padLeft: f('padLeft'),
        padRight: f('padRight'),
        lineSpacing: f('lineSpacing'),
        fillHeight: s['fillHeight'] as bool,
        gridLines: s['gridLines'] as int,
        cropLeft: f('cropLeft'),
        cropRight: f('cropRight'),
        maxAspectSlack: f('maxAspectSlack'),
      );
      final tag =
          '${spec.viewportW}x${spec.viewportH} fill=${spec.fillHeight} slack=${spec.maxAspectSlack} crop=${spec.cropLeft}';
      final l = page.layout(spec);
      close(l.scale, want['scale'] as num, '$tag scale');
      close(l.offsetX, want['offsetX'] as num, '$tag offsetX');
      close(l.offsetY, want['offsetY'] as num, '$tag offsetY');
      close(l.contentW, want['contentW'] as num, '$tag contentW');
      close(l.contentH, want['contentH'] as num, '$tag contentH');
      close(l.lineSpacing, want['lineSpacing'] as num, '$tag lineSpacing');
      close(l.fitScale, want['fitScale'] as num, '$tag fitScale');
      close(l.fitX, want['fitX'] as num, '$tag fitX');
      close(l.fitY, want['fitY'] as num, '$tag fitY');
      close(l.lineDy.first, want['lineDy0'] as num, '$tag lineDy[0]');
      close(l.lineDy.last, want['lineDyLast'] as num, '$tag lineDy[last]');
      close(
        page.layoutLineSpacingToFill(spec),
        c['lineSpacingToFill'] as num,
        '$tag lineSpacingToFill',
      );
      close(
        page.layoutWastedFraction(spec),
        c['wastedFraction'] as num,
        '$tag wastedFraction',
      );
    }
    expect(cases.length, 40);
  });

  test('zoom scenarios match the engine (conformance/scenarios/zoom.json)', () {
    final json = jsonDecode(
      File('$repo/conformance/scenarios/zoom.json').readAsStringSync(),
    ) as Map<String, dynamic>;
    final tolerance = (json['tolerance'] as num).toDouble();
    void close(double a, num b, String what) => expect(
      (a - b).abs() <= tolerance * (b.abs() < 1 ? 1 : b.abs()),
      isTrue,
      reason: '$what: got $a, engine says $b',
    );
    double f(Map<String, dynamic> o, String k) => (o[k] as num).toDouble();
    QvpView view(Map<String, dynamic> o) => QvpView(
      scale: f(o, 'scale'),
      offsetX: f(o, 'offsetX'),
      offsetY: f(o, 'offsetY'),
    );
    void same(QvpView v, Map<String, dynamic> want, String what) {
      close(v.scale, want['scale'] as num, '$what scale');
      close(v.offsetX, want['offsetX'] as num, '$what offsetX');
      close(v.offsetY, want['offsetY'] as num, '$what offsetY');
    }

    QvpZoom zoom(Map<String, dynamic> o) => QvpZoom(
      mode: QvpZoomMode.values.byName(o['mode'] as String),
      step: o['step'] as int,
      zoom: f(o, 'zoom'),
    );
    final s = json['spec'] as Map<String, dynamic>;
    final r = s['reflow'] as Map<String, dynamic>;
    final spec = QvpLayoutSpec(
      viewportW: f(s, 'viewportW'),
      viewportH: f(s, 'viewportH'),
      padTop: f(s, 'padTop'),
      padBottom: f(s, 'padBottom'),
      padLeft: f(s, 'padLeft'),
      padRight: f(s, 'padRight'),
      lineSpacing: f(s, 'lineSpacing'),
      fillHeight: s['fillHeight'] as bool,
      gridLines: s['gridLines'] as int,
      cropLeft: f(s, 'cropLeft'),
      cropRight: f(s, 'cropRight'),
      maxAspectSlack: f(s, 'maxAspectSlack'),
      surahFrames: s['surahFrames'] as bool,
      reflow: QvpReflowSpec(
        zoom: f(r, 'zoom'),
        fill: r['fill'] as String,
        wordGap: f(r, 'wordGap'),
        maxStretch: f(r, 'maxStretch'),
      ),
    );
    final l = page.layout(spec);
    final fit = json['fit'] as Map<String, dynamic>;
    close(l.fitScale, fit['fitScale'] as num, 'fit fitScale');
    close(l.fitX, fit['fitX'] as num, 'fit fitX');
    close(l.fitY, fit['fitY'] as num, 'fit fitY');
    close(l.contentW, fit['contentW'] as num, 'fit contentW');
    close(l.contentH, fit['contentH'] as num, 'fit contentH');

    for (final c in (json['zoomAbout'] as List).cast<Map<String, dynamic>>()) {
      final got = engine.viewZoomAbout(
        view(c['view'] as Map<String, dynamic>),
        f(c, 'focalX'),
        f(c, 'focalY'),
        f(c, 'factor'),
        min: f(c, 'min'),
        max: f(c, 'max'),
      );
      same(
        got,
        c['result'] as Map<String, dynamic>,
        'zoomAbout x${c['factor']} max ${c['max']}',
      );
    }
    final pan = json['pan'] as Map<String, dynamic>;
    same(
      engine.viewPan(
        view(pan['view'] as Map<String, dynamic>),
        f(pan, 'dx'),
        f(pan, 'dy'),
      ),
      pan['result'] as Map<String, dynamic>,
      'pan',
    );
    for (final c in (json['clamp'] as List).cast<Map<String, dynamic>>()) {
      final v = view(c['view'] as Map<String, dynamic>);
      same(
        engine.viewClamp(
          v,
          l.contentW,
          l.contentH,
          spec.viewportW,
          spec.viewportH,
        ),
        c['result'] as Map<String, dynamic>,
        'clamp of ${v.offsetX}, ${v.offsetY}',
      );
    }
    final toLayout = json['viewToLayout'] as Map<String, dynamic>;
    final held = view(toLayout['view'] as Map<String, dynamic>);
    for (final p in (toLayout['points'] as List).cast<List>()) {
      final got = page.viewToLayout(
        held,
        (p[0] as num).toDouble(),
        (p[1] as num).toDouble(),
      );
      close(got.x, p[2] as num, 'viewToLayout(${p[0]}, ${p[1]}) x');
      close(got.y, p[3] as num, 'viewToLayout(${p[0]}, ${p[1]}) y');
    }
    for (final c in (json['isZoomed'] as List).cast<Map<String, dynamic>>()) {
      final v = view(c['view'] as Map<String, dynamic>);
      expect(
        page.isZoomed(
          zoom(c['zoom'] as Map<String, dynamic>),
          v,
          f(c, 'fitScale'),
        ),
        c['result'] as bool,
        reason: 'isZoomed at scale ${v.scale}',
      );
    }
    final fitted = QvpView(scale: l.fitScale, offsetX: l.fitX, offsetY: l.fitY);
    for (final c in (json['pinches'] as List).cast<Map<String, dynamic>>()) {
      final tag = '${c['mode']} pinch';
      // each pinch starts from the printed layout, as the generator did
      page.layout(spec);
      final got = page.zoomPinch(
        spec,
        zoom(c['start'] as Map<String, dynamic>),
        fitted,
        f(c, 'factor'),
        f(c, 'focalX'),
        f(c, 'focalY'),
      );
      final want = zoom(c['zoom'] as Map<String, dynamic>);
      expect(got.zoom.mode, want.mode, reason: tag);
      expect(got.zoom.step, want.step, reason: tag);
      close(got.zoom.zoom, want.zoom, '$tag zoom');
      same(got.view, c['view'] as Map<String, dynamic>, tag);
      expect(got.relaid, c['relaid'] as bool, reason: tag);
      expect(
        page.isZoomed(got.zoom, got.view, l.fitScale),
        c['zoomedAfter'] as bool,
        reason: '$tag zoomed after',
      );
      // the spec the control asks for comes back whole: the zoom in force and the host's knobs
      final w = c['spec'] as Map<String, dynamic>;
      final asks = page.zoomSpec(spec, got.zoom);
      final reflow = asks.reflow;
      expect(reflow, isNotNull, reason: '$tag reflows');
      close(asks.viewportW, w['viewportW'] as num, '$tag spec viewportW');
      close(asks.padTop, w['padTop'] as num, '$tag spec padTop');
      expect(
        asks.surahFrames,
        w['surahFrames'] as bool,
        reason: '$tag spec surahFrames',
      );
      close(reflow!.zoom, w['reflowZoom'] as num, '$tag spec reflow zoom');
      expect(reflow.fill, w['fill'] as String, reason: '$tag spec fill');
      close(reflow.wordGap, w['wordGap'] as num, '$tag spec wordGap');
      close(reflow.maxStretch, w['maxStretch'] as num, '$tag spec maxStretch');
    }
    // On the printed page the control reflows nothing, and the host's knobs come back as it
    // set them.
    final printed = json['printedSpec'] as Map<String, dynamic>;
    final pw = printed['spec'] as Map<String, dynamic>;
    final asks = page.zoomSpec(
      spec,
      zoom(printed['zoom'] as Map<String, dynamic>),
    );
    final kept = asks.reflow;
    expect(kept, isNotNull, reason: 'printed spec keeps the reflow knobs');
    close(asks.viewportW, pw['viewportW'] as num, 'printed spec viewportW');
    close(asks.padTop, pw['padTop'] as num, 'printed spec padTop');
    expect(asks.surahFrames, pw['surahFrames'] as bool);
    close(kept!.zoom, pw['reflowZoom'] as num, 'printed spec reflow zoom');
    expect(kept.fill, pw['fill'] as String, reason: 'printed spec fill');
    close(kept.wordGap, pw['wordGap'] as num, 'printed spec wordGap');
    close(kept.maxStretch, pw['maxStretch'] as num, 'printed spec maxStretch');
    // A pinch whose fingers drift, replayed as a page view runs it: the engine works from the
    // control and the focal point the gesture began on, with the factor of the whole gesture;
    // the magnifying glass zooms the view the pinch began on, and the reflowing controls take
    // the view the last frame returned. The host pans the answer by the drift and holds it.
    for (final g in (json['drifts'] as List).cast<Map<String, dynamic>>()) {
      page.layout(spec);
      final start = zoom(g['start'] as Map<String, dynamic>);
      final began = view(g['view'] as Map<String, dynamic>);
      final fx = f(g, 'focalX'), fy = f(g, 'focalY');
      var base = began;
      final frames = (g['frames'] as List).cast<Map<String, dynamic>>();
      for (var i = 0; i < frames.length; i++) {
        final fr = frames[i];
        final tag = '${g['mode']} drifting pinch, frame $i';
        final c = page.zoomPinch(
          spec,
          start,
          start.mode == QvpZoomMode.magnify ? began : base,
          f(fr, 'factor'),
          fx,
          fy,
        );
        base = c.view;
        final drawn = page.dressClamp(
          engine.viewPan(c.view, f(fr, 'focalX') - fx, f(fr, 'focalY') - fy),
          spec.viewportW,
          spec.viewportH,
        );
        final want = zoom(fr['zoom'] as Map<String, dynamic>);
        expect(c.zoom.mode, want.mode, reason: tag);
        expect(c.zoom.step, want.step, reason: tag);
        close(c.zoom.zoom, want.zoom, '$tag zoom');
        expect(c.relaid, fr['relaid'] as bool, reason: '$tag relaid');
        same(drawn, fr['view'] as Map<String, dynamic>, tag);
      }
    }
    page.layout(spec);
  });

  // The exact and the gap-aware hit tests on a word's outline, a word's box
  // alone, a decoration's outline and a decoration's box alone, in page units
  // and in viewport px.
  test('hit scenarios match the engine (conformance/scenarios/hit.json)', () {
    final json = jsonDecode(
      File('$repo/conformance/scenarios/hit.json').readAsStringSync(),
    ) as Map<String, dynamic>;
    final tolerance = (json['tolerance'] as num).toDouble();
    final cases = json['cases'] as List<dynamic>;
    final s = json['spec'] as Map<String, dynamic>;
    double f(String k) => (s[k] as num).toDouble();
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    p.layout(
      QvpLayoutSpec(
        viewportW: f('viewportW'),
        viewportH: f('viewportH'),
        padTop: f('padTop'),
        padBottom: f('padBottom'),
        padLeft: f('padLeft'),
        padRight: f('padRight'),
      ),
    );
    void same(QvpHit? got, Object? want, String what) {
      if (want == null) {
        expect(got, isNull, reason: '$what: got a hit, engine says none');
        return;
      }
      expect(got, isNotNull, reason: '$what: got no hit');
      final w = want as Map<String, dynamic>;
      expect(
        [got!.word, got.path, got.decoration, got.line, got.isExact],
        [w['word'], w['path'], w['decoration'], w['line'], w['isExact']],
        reason: what,
      );
      expect(
        (got.distance - (w['distance'] as num)).abs() <= tolerance,
        isTrue,
        reason:
            '$what distance: got ${got.distance}, engine says ${w['distance']}',
      );
    }

    for (final c in cases) {
      final x = (c['x'] as num).toDouble(), y = (c['y'] as num).toDouble();
      final view = c['units'] == 'view';
      final tag = 'hit ${c['name']} (${c['units']} $x, $y)';
      same(
        view ? p.hitTestExactView(x, y) : p.hitTestExact(x, y),
        c['exact'],
        '$tag exact',
      );
      same(
        view ? p.hitTestView(x, y) : p.hitTest(x, y),
        c['hit'],
        '$tag gap-aware',
      );
    }
    expect(cases.length, 8);
    p.dispose();
  });

  // The band of the page a host keeps drawn: a page that fits, a magnified page
  // drawn a band at a time, pans inside one step and across it, the host's own
  // limits, and no screen at all.
  test(
    'ink band scenarios match the engine (conformance/scenarios/ink-band.json)',
    () {
      final json = jsonDecode(
        File('$repo/conformance/scenarios/ink-band.json').readAsStringSync(),
      ) as Map<String, dynamic>;
      final tolerance = (json['tolerance'] as num).toDouble();
      final cases = json['cases'] as List<dynamic>;
      final s = json['spec'] as Map<String, dynamic>;
      double f(String k) => (s[k] as num).toDouble();
      void close(double got, num want, String what) => expect(
        (got - want).abs() <= tolerance * math.max(1, want.abs()),
        isTrue,
        reason: '$what: got $got, engine says $want',
      );
      final p = engine.loadPage(File(pagePath).readAsBytesSync());
      final l = p.layout(
        QvpLayoutSpec(
          viewportW: f('viewportW'),
          viewportH: f('viewportH'),
          padTop: f('padTop'),
          padBottom: f('padBottom'),
          padLeft: f('padLeft'),
          padRight: f('padRight'),
        ),
      );
      close(l.contentH, json['contentH'] as num, 'contentH');
      for (final c in cases) {
        final v = c['view'] as Map<String, dynamic>;
        final got = p.layoutInkBand(
          QvpView(
            scale: (v['scale'] as num).toDouble(),
            offsetX: (v['offsetX'] as num).toDouble(),
            offsetY: (v['offsetY'] as num).toDouble(),
          ),
          (c['viewportW'] as num).toDouble(),
          (c['viewportH'] as num).toDouble(),
          maxHeight: (c['maxHeight'] as num).toDouble(),
          maxPixels: (c['maxPixels'] as num).toDouble(),
        );
        final want = c['band'] as Map<String, dynamic>;
        close(got.top, want['top'] as num, 'ink band ${c['name']} top');
        close(
          got.height,
          want['height'] as num,
          'ink band ${c['name']} height',
        );
      }
      expect(cases.length, 14);
      p.dispose();
    },
  );

  test('the reader\'s zoom control reflows the page and one loop draws it', () {
    const spec = QvpLayoutSpec(
      viewportW: 390,
      viewportH: 844,
      padTop: 12,
      padBottom: 12,
      padLeft: 8,
      padRight: 8,
      surahFrames: false,
    );

    // a zeroed control is stepped, on the printed page, and asks for no reflow when the host
    // gave it no knobs
    const printed = QvpZoom();
    expect(printed.mode, QvpZoomMode.stepped);
    expect(page.zoomSpec(spec, printed).reflow, isNull);
    expect(page.zoomSpec(spec, printed).surahFrames, isFalse);

    // the host's reflow knobs survive the printed page, with zoom 0, so the first pinch that
    // leaves it reflows with them and not with the engine's defaults
    final knobbed = spec.copyWith(
      reflow: const QvpReflowSpec(
        fill: 'justified',
        maxStretch: 3,
        wordGap: 1.5,
      ),
    );
    final kept = page.zoomSpec(knobbed, printed).reflow;
    expect(kept, isNotNull);
    expect(kept!.zoom, 0);
    expect(kept.fill, 'justified');
    expect(kept.maxStretch, 3);
    expect(kept.wordGap, 1.5);
    expect(page.layout(page.zoomSpec(knobbed, printed)).reflowed, isFalse);

    // the page ships its own steps, each larger than the last
    final steps = page.zoomSteps(spec);
    expect(steps, isNotEmpty);
    for (var i = 1; i < steps.length; i++) {
      expect(steps[i], greaterThan(steps[i - 1]));
    }
    expect(page.zoomAtStep(spec, 0), 1);
    expect(page.zoomAtStep(spec, 1), closeTo(steps[0], 1e-4));
    expect(page.reflowMaxZoom(spec), greaterThan(1));

    // a size button lands on a step exactly, and the spec it asks for reflows to it
    final c = page.zoomToStep(spec, printed, 1, const QvpView());
    expect(c.zoom.step, 1);
    expect(c.relaid, isTrue);
    final reflowed = page.zoomSpec(spec, c.zoom);
    expect(reflowed.reflow, isNotNull);
    expect(reflowed.reflow!.zoom, closeTo(steps[0], 1e-4));
    expect(reflowed.surahFrames, isFalse);

    // laid out under it, the page is rows rather than the print's lines
    final l = page.layout(reflowed);
    expect(l.reflowed, isTrue);
    expect(l.rows, greaterThan(0));
    expect(page.readLayout()!.reflowed, isTrue);

    // and one loop draws it: every draw names a path and a placement that exists
    final places = page.layoutPlacements();
    final draws = page.layoutDrawList();
    expect(draws, isNotEmpty);
    expect(places, isNotEmpty);
    for (final d in draws) {
      expect(d.path, lessThan(page.nPaths));
      expect(d.placement, lessThan(places.length));
    }

    // a pinch that has not reached the next step commits nothing. The page is on the printed
    // layout first: the engine lays out a page that is at another step than its control.
    page.layout(spec);
    final held = page.zoomPinch(spec, printed, const QvpView(), 1.0, 195, 422);
    expect(held.relaid, isFalse);
    expect(held.zoom.step, 0);

    // the printed page is the floor: a magnifying pinch never shrinks it inside the screen
    final fit = page.layout(spec).fitScale;
    const mag = QvpZoom(mode: QvpZoomMode.magnify);
    final out = page.zoomPinch(spec, mag, QvpView(scale: fit), 0.2, 195, 422);
    expect(out.view.scale, greaterThanOrEqualTo(fit - 1e-4));

    // a page turn keeps the step, because every page's steps are its own
    expect(page.zoomCarried(spec, c.zoom).step, 1);

    // and the page laid out again is the printed one when the control goes back to it
    page.layout(spec);
  });

  test('passage scenarios match the engine (conformance/scenarios/passage.json)', () {
    final json = jsonDecode(
      File('$repo/conformance/scenarios/passage.json').readAsStringSync(),
    ) as Map<String, dynamic>;
    final tolerance = (json['tolerance'] as num).toDouble();
    final cases = json['cases'] as List<dynamic>;
    void close(double a, num b, String what) => expect(
      (a - b).abs() <= tolerance,
      isTrue,
      reason: '$what: got $a, engine says $b',
    );
    QvpPage load(int n) => engine.loadPage(
      File('$repo/dist/pages/${n.toString().padLeft(3, '0')}.qvp')
          .readAsBytesSync(),
    );
    for (final c in cases) {
      final numbers = (c['pages'] as List<dynamic>).cast<int>();
      final surah = c['surah'] as int,
          from = c['from'] as int,
          to = c['to'] as int;
      final basmalah = c['basmalah'] as bool? ?? false;
      final pages = [for (final n in numbers) load(n)];
      final passage = basmalah
          ? QvpPassage.basmalah(pages, surah)
          : QvpPassage.load(pages, surah, from, to: to);
      // The passage keeps what it needs.
      for (final p in pages) {
        p.dispose();
      }
      final s = c['spec'] as Map<String, dynamic>,
          want = c['layout'] as Map<String, dynamic>;
      double f(String k) => (s[k] as num).toDouble();
      final spec = QvpPassageSpec(
        width: f('width'),
        scale: f('scale'),
        lineSpacing: f('lineSpacing'),
        padding: f('padding'),
        align: QvpPassageAlign.values.byName(s['align'] as String),
        maxRows: s['maxRows'] as int,
        keepAyahMark: s['keepAyahMark'] as bool,
        ellipsisWidth: f('ellipsisWidth'),
        maxStretch: f('maxStretch'),
      );
      final tag =
          '${basmalah ? 'basmalah of ' : ''}$surah:$from-$to at ${spec.width} ${s['align']} rows ${spec.maxRows}';
      final l = passage.layout(spec);
      expect(l, isNotNull, reason: tag);
      expect(passage.currentLayout, same(l));
      close(l!.width, want['width'] as num, '$tag width');
      close(l.height, want['height'] as num, '$tag height');
      close(l.scale, want['scale'] as num, '$tag scale');
      close(l.lineSpacing, want['lineSpacing'] as num, '$tag lineSpacing');
      expect(l.isTruncated, want['isTruncated'] as bool, reason: tag);
      expect(l.draws.length, want['draws'] as int, reason: tag);
      final rows = want['rows'] as List<dynamic>,
          words = want['words'] as List<dynamic>;
      expect(l.rows.length, rows.length, reason: tag);
      expect(l.words.length, words.length, reason: tag);
      for (var i = 0; i < l.rows.length && i < rows.length; i++) {
        final r = l.rows[i], g = (rows[i] as List<dynamic>).cast<num>();
        close(r.x0, g[0], '$tag row $i x0');
        close(r.y0, g[1], '$tag row $i y0');
        close(r.x1, g[2], '$tag row $i x1');
        close(r.y1, g[3], '$tag row $i y1');
        close(r.baseline, g[4], '$tag row $i baseline');
      }
      for (var i = 0; i < l.words.length && i < words.length; i++) {
        final w = l.words[i], g = (words[i] as List<dynamic>).cast<num>();
        expect(
          [w.page, w.ayah, w.word, w.row],
          g.sublist(0, 4).map((v) => v.toInt()).toList(),
          reason: '$tag word $i',
        );
        close(w.x0, g[4], '$tag word $i x0');
        close(w.y0, g[5], '$tag word $i y0');
        close(w.x1, g[6], '$tag word $i x1');
        close(w.y1, g[7], '$tag word $i y1');
        close(w.wordX0, g[8], '$tag word $i wordX0');
        close(w.wordY0, g[9], '$tag word $i wordY0');
        close(w.wordX1, g[10], '$tag word $i wordX1');
        close(w.wordY1, g[11], '$tag word $i wordY1');
      }
      final e = (want['ellipsis'] as List<dynamic>?)?.cast<num>();
      if (e != null) {
        expect(l.ellipsis, isNotNull, reason: tag);
        close(l.ellipsis!.left, e[0], '$tag ellipsis x0');
        close(l.ellipsis!.top, e[1], '$tag ellipsis y0');
        close(l.ellipsis!.right, e[2], '$tag ellipsis x1');
        close(l.ellipsis!.bottom, e[3], '$tag ellipsis y1');
      } else {
        expect(l.ellipsis, isNull, reason: tag);
      }
      // every draw names a page of the passage, a path on it and a placement that exists
      for (final d in l.draws) {
        expect(numbers, contains(d.page), reason: tag);
        expect(d.placement, lessThan(l.placements.length), reason: tag);
      }
      if (basmalah) {
        expect(passage.ayahs, isEmpty, reason: tag);
      } else {
        expect(passage.ayahs.length, to - from + 1, reason: tag);
        expect(passage.ayahs.first.text, isNotEmpty, reason: tag);
      }
      expect(passage.lineSpacing, greaterThan(0), reason: tag);
      passage.close();
      expect(passage.isDisposed, isTrue);
    }
    expect(cases.length, 58);
  });

  test('a basmalah passage has no ayahs; surah 1, surah 9 and a page without it fail to load', () {
    QvpPage load(int n) => engine.loadPage(
      File('$repo/dist/pages/${n.toString().padLeft(3, '0')}.qvp')
          .readAsBytesSync(),
    );
    final two = load(2), one = load(1), tawbah = load(187);
    final basmalah = QvpPassage.basmalah([two], 2);
    expect(basmalah.ayahs, isEmpty);
    final first = QvpPassage.load([two], 2, 1);
    expect(basmalah.lineSpacing, first.lineSpacing);
    expect(first.ayahs.single.ayahKey, '2:1');
    first.close();
    final l = basmalah.layout(
      QvpPassageSpec(
        width: 366,
        scale: 22 / basmalah.lineSpacing,
        align: QvpPassageAlign.center,
      ),
    );
    expect(l, isNotNull);
    expect(l!.rows.length, 1);
    expect(l.words, isEmpty);
    expect(l.draws, isNotEmpty);
    expect(() => QvpPassage.basmalah([one], 1), throwsFormatException);
    expect(() => QvpPassage.basmalah([tawbah], 9), throwsFormatException);
    expect(() => QvpPassage.basmalah([two], 3), throwsFormatException);
    expect(() => QvpPassage.load([two], 2, 1, to: 300), throwsFormatException);
    basmalah.close();
    basmalah.close();
    for (final p in [two, one, tawbah]) {
      p.dispose();
    }
    expect(() => QvpPassage.load([two], 2, 1), throwsStateError);
  });

  // A three-ornament set, one of each kind in the style "test": `ornament_bytes()` in
  // crates/qvp-ffi/tests/abi.rs, encoded by the engine's own encoder. Each asset is a filled
  // box (part c1) and a stroked box (part line).
  Uint8List testOrnaments() => base64Decode(
    'UVZPMegDAwABBHRlc3QOSGFmcyAnYW4gJ0FzaW0PQ0MtQlktTkMtU0EtNC4wC3Byb3Zpc2lvbmFsAAZhIHRlc3QCAmMx/5tkAAAE'
    'bGluZf9BNjABAAAAAAAAAAAAAKBCAADIQgEAAKBBAACgQQAAIEIAAHBCAAIAAQAEdGVzdA5IYWZzICdhbiAnQXNpbQ9DQy1CWS1O'
    'Qy1TQS00LjALcHJvdmlzaW9uYWwABmEgdGVzdAICYzH/m2QAAARsaW5l/0E2MAEAAAAAAAAAAAAASEQAAMhCAQAAyEIAACBBAAAW'
    'RAAAoEICAgACAAR0ZXN0DkhhZnMgJ2FuICdBc2ltD0NDLUJZLU5DLVNBLTQuMAtwcm92aXNpb25hbAAGYSB0ZXN0AgJjMf+bZAAA'
    'BGxpbmX/QTYwAQAAAAAAAAAAAACMQgAAyEIBAACgQAAAoEAAAHBCAAC0QgQCAAYAAAAAEwECyAETEwAAACYTAQLIATkTAAAATBMB'
    'AsgBXxNyAAAAAYDiCQABAMCaDAH/4QkABAAAAAGA4gkAAQDAmgwB/+EJAAQAAAABgNRhAAEAwJoMAf/TYQAEAAAAAYDUYQABAMCa'
    'DAH/02EABAAAAAHgxQgAAQDAmgwB38UIAAQAAAAB4MUIAAEAwJoMAd/FCAAE',
  );

  test('dress: ornaments load, find, dress page 604, geometry, undress, a refused spec', () {
    final set = engine.loadOrnaments(testOrnaments());
    expect(set.count, 3);
    expect(set.list.length, 3);
    final mark = set.list[0];
    expect(mark.kind, QvpOrnamentKind.ayahMark);
    expect(mark.lineage, QvpLineage.font);
    expect(mark.style, 'test');
    expect(mark.riwayah, "Hafs 'an 'Asim");
    expect(mark.hasSlices, isFalse);
    expect(mark.licence.id, 'CC-BY-NC-SA-4.0');
    expect(mark.licence.status, 'provisional');
    expect(mark.licence.redistributable, isFalse);
    expect(mark.licence.attribution, 'a test');
    expect(mark.parts.length, 2);
    expect(mark.parts[1].name, 'line');
    expect(mark.parts[1].color, 0x303641ff);
    expect(mark.parts[1].stroke, isTrue);
    expect(set.list[1].lineage, QvpLineage.scan);
    expect(set.find(QvpOrnamentKind.pageFrame, 'test')?.index, 2);
    expect(set.find(QvpOrnamentKind.ayahMark, 'nope'), isNull);
    expect(set.ofKind(QvpOrnamentKind.surahHeader).single.index, 1);
    final styles = set.styles();
    expect(styles.length, 1);
    expect(styles.single.isComplete, isTrue);
    expect(styles.single.pageFrame?.index, 2);

    final p = engine.loadPage(
      File('$repo/dist/pages/604.qvp').readAsBytesSync(),
    );
    p.layout(
      const QvpLayoutSpec(
        viewportW: 690,
        viewportH: 1100,
        padTop: 20,
        padBottom: 20,
      ),
    );
    expect(p.dressInfo(), isNull);
    expect(p.dressRevision(), 0);
    expect(p.dressGeometry(), isEmpty);

    // a style takes all three kinds, and the border's fill is repainted red
    final info = p.dress(
      set,
      QvpDressSpec(
        style: 'test',
        ayahMarkScale: 1,
        colors: [(set.list[2], 0, 0xff0000ff)],
      ),
    )!;
    expect(p.dressSpec, isNotNull);
    expect((info.ayahMark, info.surahHeader, info.pageFrame), (0, 1, 2));
    expect(info.ayahMarks, greaterThan(0));
    expect(info.surahHeaders, greaterThan(0));
    expect(info.frameStretched, isTrue, reason: 'no slices in this set');
    expect(info.frameRepeats, 0);
    expect(info.viewBox[0], lessThan(0));
    expect(info.viewBox[2], greaterThan(p.width));
    expect(
      info.overflow.every((v) => v > 0),
      isTrue,
      reason: '${info.overflow}',
    );
    expect(info.revision, greaterThan(0));
    expect(p.dressRevision(), info.revision);

    final draws = p.dressGeometry();
    expect(draws.length, info.draws);
    expect(
      draws.length,
      (info.ayahMarks + info.surahHeaders + 1) * 2,
      reason: 'one fill and one stroke per placed asset',
    );
    expect(
      identical(p.dressGeometry(), draws),
      isTrue,
      reason: 'kept until the revision changes',
    );
    final places = p.layoutPlacements();
    var red = 0;
    for (final d in draws) {
      expect(d.path.getBounds().isEmpty, isFalse);
      if (d.stroke) {
        expect(d.strokeWidth, greaterThan(0));
        expect(d.color, 0x303641ff);
      } else if (d.kind == QvpOrnamentKind.pageFrame) {
        expect(d.color, 0xff0000ff);
        red++;
      }
      // the border belongs to no placement, and the rest are drawn under one that exists
      expect(d.placement == null, d.kind == QvpOrnamentKind.pageFrame);
      if (d.placement != null) expect(d.placement, lessThan(places.length));
    }
    expect(red, 1);

    // a layout rebuilds the layer
    p.layout(
      const QvpLayoutSpec(
        viewportW: 690,
        viewportH: 2000,
        padTop: 20,
        padBottom: 20,
        fillHeight: true,
      ),
    );
    expect(p.dressRevision(), isNot(info.revision));
    expect(identical(p.dressGeometry(), draws), isFalse);

    // a kind key overrides the style, an index and a style name both work, and line art is a set
    final mixed = p.dress(
      set,
      const QvpDressSpec(
        style: 'test',
        ayahMark: 0,
        surahHeader: 'test',
        lineArt: {QvpOrnamentKind.ayahMark},
        pageFrameScale: 1.5,
      ),
    );
    expect(mixed, isNotNull);
    expect(
      p
          .dressGeometry()
          .where((d) => d.kind == QvpOrnamentKind.ayahMark)
          .every((d) => d.stroke),
      isTrue,
    );

    // refused: an ornament of another kind, or nothing to dress
    expect(p.dress(set, const QvpDressSpec(ayahMark: 1)), isNull);
    expect(p.dressInfo(), isNull);
    expect(p.dress(set, const QvpDressSpec()), isNull);
    expect(p.dress(set, const QvpDressSpec(style: 'nope')), isNull);

    p.dress(set, const QvpDressSpec(pageFrame: 2));
    expect(p.dressInfo()?.pageFrame, 2);
    p.undress();
    expect(p.dressInfo(), isNull);
    expect(p.dressSpec, isNull);
    expect(p.dressRevision(), 0);
    p.dispose();
    set.dispose();
    expect(set.isDisposed, isTrue);
  });

  test('a removed highlight leaves no band once the clock passes its fade', () {
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    final h = p.highlight(
      '2:255',
      const QvpHighlightStyle(mode: 'band', ms: 200),
    );
    // part way through the fade, where the test above reads six boxes
    p.tick(100);
    expect(p.highlightBoxesView(), isNotEmpty);
    p.removeHighlight(h);
    // The engine drops a removed highlight inside its own moving window, so whoever reads the
    // boxes has to read them again after the clock has moved. QvpPageView does that by
    // keying its band cache on the page's clock as well. A cache keyed on page.revision alone
    // would hold these six boxes for ever, because tick() changes no revision.
    p.tick(400);
    p.tick(1000);
    expect(p.highlightBoxesView(), isEmpty);
    p.dispose();
  });

  test(
    'a colour-only re-dress moves the revision the ornament layer is keyed on',
    () {
      final set = engine.loadOrnaments(testOrnaments());
      final p = engine.loadPage(
        File('$repo/dist/pages/604.qvp').readAsBytesSync(),
      );
      expect(p.dress(set, const QvpDressSpec(style: 'test')), isNotNull);
      final before = p.dressInfo()!.revision;
      final colours = p.dressGeometry().map((d) => d.color).toList();
      expect(colours, isNotEmpty);
      // A repaint keeps the dressed box, so the page number, the style and the layout all stay
      // put and only dressInfo().revision moves. That is why the ornament picture is keyed on
      // the revision: a key built from the style and the ornament indices would replay the old
      // colours for ever.
      expect(
        p.dress(
          set,
          QvpDressSpec(style: 'test', colors: [(set.list[2], 0, 0xff0000ff)]),
        ),
        isNotNull,
      );
      expect(p.dressInfo()!.revision, greaterThan(before));
      expect(p.dressGeometry().map((d) => d.color).toList(), isNot(colours));
      p.dispose();
    },
  );

  test('batch notifies once', () {
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    var n = 0;
    p.addListener(() => n++);
    p.batch(() {
      p.highlight('2:255');
      p.highlight('2:256');
      p.setDefaultColor(const Color(0xFF112233));
    });
    expect(n, 1);
    // and the memo behind ayahMarks() is dropped by a mutation, not held past it
    final marks = p.ayahMarks();
    expect(identical(p.ayahMarks(), marks), isTrue);
    p.highlight('2:255');
    expect(identical(p.ayahMarks(), marks), isFalse);
    p.dispose();
  });

  testWidgets('a dressed page paints its ornament layer behind the ink', (
    tester,
  ) async {
    final set = engine.loadOrnaments(testOrnaments());
    final p = engine.loadPage(
      File('$repo/dist/pages/604.qvp').readAsBytesSync(),
    );
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: SizedBox(width: 400, height: 640, child: QvpPageView(page: p)),
      ),
    );
    final state = tester.state<QvpPageViewState>(find.byType(QvpPageView));
    final printed = state.controller.scale;
    expect(p.dress(set, const QvpDressSpec(style: 'test')), isNotNull);
    await tester.pump();
    // a dressed page is bigger than its text, so it is fitted smaller
    expect(state.controller.scale, lessThan(printed));
    // the engine owns the fit: the view sits where its layout says
    final l = p.readLayout()!;
    expect(state.controller.scale, closeTo(l.fitScale, 1e-6));
    expect(state.controller.offsetX, closeTo(l.fitX, 1e-3));
    expect(state.controller.offsetY, closeTo(l.fitY, 1e-3));
    // a drag far to the right is held by the engine, and an undressed page gets the plain clamp
    final far = QvpView(scale: l.fitScale, offsetX: 1e5, offsetY: 1e5);
    final held = p.dressClamp(far, 400, 640);
    expect(held.offsetX, lessThan(1e5));
    // a host that scrolls the page: the size is content plus overflow, the border's top at 0
    final o = p.dressInfo()!.overflow;
    final shown = p.dressView(1, 400);
    expect(shown.width, closeTo(l.contentW + o[0] + o[2], 1e-3));
    expect(shown.height, closeTo(l.contentH + o[1] + o[3], 1e-3));
    expect(shown.view.offsetY - shown.view.scale * o[1], closeTo(0, 1e-3));
    expect(state.stats.basePaths, greaterThan(0));
    // A new colour keeps the dressed box, so the reader's zoom stays.
    state.controller.set(scale: l.fitScale * 2, offsetX: -40, offsetY: -60);
    expect(
      p.dress(
        set,
        QvpDressSpec(style: 'test', colors: [(set.list[2], 0, 0xff0000ff)]),
      ),
      isNotNull,
    );
    await tester.pump();
    expect(state.controller.scale, closeTo(l.fitScale * 2, 1e-6));
    expect((state.controller.offsetX, state.controller.offsetY), (-40, -60));
    // A bigger border grows the box, so the page is fitted again.
    expect(
      p.dress(
        set,
        const QvpDressSpec(style: 'test', pageFrameScale: 2, gap: 20),
      ),
      isNotNull,
    );
    await tester.pump();
    expect(state.controller.scale, closeTo(p.readLayout()!.fitScale, 1e-6));
    p.undress();
    await tester.pump();
    expect(state.controller.scale, closeTo(printed, 1e-6));
    expect(p.dressClamp(far, 400, 640).offsetX, lessThan(1e5));
    await tester.pumpWidget(const SizedBox());
    p.dispose();
    set.dispose();
  });
  testWidgets('a tap hands over the word, the decoration and the hit', (
    tester,
  ) async {
    // Page 1 carries a surah heading as well as words, so one page reaches
    // both branches of the tap.
    final p = engine.loadPage(
      File('$repo/dist/pages/001.qvp').readAsBytesSync(),
    );
    QvpWordInfo? word;
    QvpHit? wordHit;
    QvpDecorationInfo? deco;
    QvpHit? decoHit;
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: QvpPageView(
          page: p,
          onWordTap: (w, h) {
            word = w;
            wordHit = h;
          },
          onDecorationTap: (d, h) {
            deco = d;
            decoHit = h;
          },
        ),
      ),
    );
    final c = tester
        .state<QvpPageViewState>(find.byType(QvpPageView))
        .controller;
    // The widget's local space: view units through the layout it set.
    Offset at(double x0, double y0, double x1, double y1) => Offset(
      (x0 + x1) / 2 * c.scale + c.offsetX,
      (y0 + y1) / 2 * c.scale + c.offsetY,
    );

    // The host is handed the word itself, not its index.
    final w = p.wordBoundsView(0);
    await tester.tapAt(at(w.x0, w.y0, w.x1, w.y1));
    // onDoubleTap puts a double-tap recognizer in the arena, so a single tap
    // resolves once its timeout has passed.
    await tester.pump(const Duration(milliseconds: 400));
    expect(word?.index, 0);
    expect(word?.text, p.words[0].text);
    expect(wordHit?.word, 0);

    // A decoration tap carries its hit, so a host reads the line, the distance
    // and isExact without hit-testing the point again.
    final h = p.surahHeadersView().first;
    await tester.tapAt(at(h.titleX0, h.titleY0, h.titleX1, h.titleY1));
    await tester.pump(const Duration(milliseconds: 400));
    expect(deco?.index, h.decoration);
    expect(decoHit?.decoration, h.decoration);
    expect(decoHit!.word, lessThan(0));

    await tester.pumpWidget(const SizedBox());
    p.dispose();
  });

  // The same three rules as every other wrapper (docs/API.md, conventions). `flutter test`
  // runs in debug mode, where the old handle was only an assert; the error is now the same
  // StateError in every mode.
  test(
    'a disposed page, atlas or passage throws a StateError that names it',
    () {
      final p = engine.loadPage(File(pagePath).readAsBytesSync());
      final passage = QvpPassage.load([p], 2, 255);
      final h = p.handle;
      p.dispose();
      expect(
        () => p.hitAreas(),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            'QvpPage is disposed',
          ),
        ),
      );
      // the backend refuses the freed handle itself, so the engine never sees a null
      expect(
        () => engine.backend.lineBands(h),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            'QvpPage is disposed',
          ),
        ),
      );
      passage.close();
      expect(
        () => passage.layout(const QvpPassageSpec(width: 360)),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            'QvpPassage is disposed',
          ),
        ),
      );
      if (!File(atlasPath).existsSync()) return;
      final atlas = engine.loadAtlas(File(atlasPath).readAsBytesSync());
      atlas.free();
      expect(
        () => atlas.pageOf(2, 255),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            'QvpAtlas is disposed',
          ),
        ),
      );
    },
  );

  test(
    'a call after engine.dispose throws a StateError, and a free still works',
    () {
      final own = QvpEngine.open(path: libPath);
      final p = own.loadPage(File(pagePath).readAsBytesSync());
      own.dispose();
      own.dispose();
      final disposed = throwsA(
        isA<StateError>().having(
          (e) => e.message,
          'message',
          'QvpEngine was disposed',
        ),
      );
      expect(() => p.hitAreas(), disposed);
      // calls that fill a struct before they reach the engine are refused first
      expect(
        () => p.hitTest(10, 10, const QvpHitOptions(maxDistance: 6)),
        disposed,
      );
      expect(
        () => p.layout(const QvpLayoutSpec(viewportW: 690, viewportH: 1100)),
        disposed,
      );
      expect(() => p.style(Sel.mark('shaddah'), 0xff0000ff), disposed);
      expect(() => p.text('2:255'), disposed);
      expect(() => p.theme(const QvpTheme()), disposed);
      expect(() => own.version, disposed);
      expect(() => own.name(QvpNames.mark, 1), disposed);
      expect(() => own.loadPage(File(pagePath).readAsBytesSync()), disposed);
      p.dispose();
      expect(p.isDisposed, isTrue);
    },
  );

  test('a second free, close or dispose does nothing', () {
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    final passage = QvpPassage.load([p], 2, 255);
    final h = p.handle;
    p.free();
    p.free();
    p.dispose();
    p.dispose();
    engine.backend.pageFree(h);
    expect(p.isDisposed, isTrue);
    passage.close();
    passage.dispose();
    expect(passage.isDisposed, isTrue);
    if (!File(atlasPath).existsSync()) return;
    final atlas = engine.loadAtlas(File(atlasPath).readAsBytesSync());
    atlas.free();
    atlas.dispose();
    expect(atlas.isDisposed, isTrue);
  });

  // The engine's swipe rule: mostly sideways, and far enough or fast enough. A mushaf reads
  // right to left, so a flick to the right turns to the next page.
  test('viewSwipe and swipePages read a released drag', () {
    expect(engine.viewSwipe(100, 0), 1);
    expect(engine.viewSwipe(-100, 0), -1);
    expect(engine.viewSwipe(10, 0), 0, reason: 'too short and too slow');
    expect(engine.viewSwipe(10, 0, 900, 0), 1, reason: 'short but fast');
    expect(engine.viewSwipe(100, 100), 0, reason: 'not sideways');
    expect(engine.swipePages(100, 0), 1);
    expect(engine.swipePages(-100, 0, -900, 0), -1);
    expect(engine.swipePages(0, 200), 0);
  });

  test('viewAnchor holds a point of a word where it is on the screen', () {
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    const spec = QvpLayoutSpec(viewportW: 690, viewportH: 1100);
    final l = p.layout(spec);
    final held = engine.viewClamp(
      const QvpView(scale: 2, offsetX: -300, offsetY: -400),
      l.contentW,
      l.contentH,
      spec.viewportW,
      spec.viewportH,
    );
    const word = 40;
    final b = p.wordBoundsView(word);
    // a point a quarter across the word and three quarters down, so a swapped pair of
    // arguments lands somewhere else
    final toX = held.offsetX + held.scale * (b.x0 + 0.25 * (b.x1 - b.x0));
    final toY = held.offsetY + held.scale * (b.y0 + 0.75 * (b.y1 - b.y0));
    final v = p.viewAnchor(
      held,
      word,
      0.25,
      0.75,
      toX,
      toY,
      spec.viewportW,
      spec.viewportH,
    );
    expect(v.scale, closeTo(held.scale, 1e-4));
    expect(v.offsetX, closeTo(held.offsetX, 1e-2));
    expect(v.offsetY, closeTo(held.offsetY, 1e-2));
    p.dispose();
  });

  // The viewport-px twins of ayahMarks, hitAreas and wordBands: the same records through the
  // current layout. On a printed layout each is the page-unit answer scaled and moved.
  test('ayahMarksView, hitAreasView and wordBandsView answer in viewport px', () {
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    const spec = QvpLayoutSpec(viewportW: 690, viewportH: 1100);
    final l = p.layout(spec);
    final marks = p.ayahMarks(), marksView = p.ayahMarksView();
    expect(marksView.map((m) => m.ayah), marks.map((m) => m.ayah));
    expect(marksView, isNotEmpty);
    for (var i = 0; i < marks.length; i++) {
      expect(marksView[i].r, closeTo(marks[i].r * l.scale, 1e-2));
      expect(marksView[i].cy, inInclusiveRange(0, l.contentH));
    }
    final areasView = p.hitAreasView();
    expect(areasView.map((a) => a.word), p.hitAreas().map((a) => a.word));
    expect(areasView.length, 147);
    final ws = p.targetWords('2:255');
    final bands = p.wordBands(ws), bandsView = p.wordBandsView(ws);
    expect(bandsView.length, bands.length);
    for (var i = 0; i < bands.length; i++) {
      final (b, v) = (bands[i], bandsView[i]);
      expect(v.x1 - v.x0, closeTo((b.x1 - b.x0) * l.scale, 1e-2));
      expect(v.y0, inInclusiveRange(0, l.contentH));
    }
    // a reflowed page puts its words on other rows; the twins follow them there
    p.layout(spec.copyWith(reflow: const QvpReflowSpec(zoom: 1.6)));
    expect(p.hitAreasView().length, 147);
    expect(p.wordBandsView(ws), isNotEmpty);
    expect(p.ayahMarksView().length, marks.length);
    p.dispose();
  });

  test('an array larger than the scratch is read in full', () {
    // A 4 KB scratch holds 102 hit areas and 1,024 handles: page 42 has 147 words, and
    // 1,100 highlights have 1,100 handles. Both must come back whole.
    final small = QvpEngine.fromBackend(
      QvpNativeBackend(path: libPath, scratchBytes: 4096),
    );
    final p = small.loadPage(File(pagePath).readAsBytesSync());
    expect(
      p.hitAreas().map((a) => a.word).toList(),
      page.hitAreas().map((a) => a.word).toList(),
    );
    expect(p.hitAreas().length, 147);
    final added = [
      for (var i = 0; i < 1100; i++) p.highlight(T.word(i % p.nWords)),
    ];
    expect(p.highlightHandles(), added);
    p.dispose();
    small.dispose();
  });

  testWidgets('a host controller handed to a new view keeps working', (
    tester,
  ) async {
    final p = engine.loadPage(
      File('$repo/dist/pages/042.qvp').readAsBytesSync(),
    );
    final ctl = QvpViewController();
    Widget view(int key) => Directionality(
      textDirection: TextDirection.ltr,
      child: QvpPageView(key: ValueKey(key), page: p, controller: ctl),
    );
    await tester.pumpWidget(view(1));
    // The new view takes the controller in its initState, before the old view's dispose: the
    // old view must not let go of a controller that is no longer its own.
    await tester.pumpWidget(view(2));
    expect(() => ctl.toView(const Offset(10, 10)), returnsNormally);

    // zoomBy floors at the scale the page is fitted at, as a magnifying pinch does
    final fitted = ctl.scale;
    ctl.zoomBy(0.1);
    expect(ctl.scale, closeTo(fitted, 1e-6));
    ctl.zoomBy(2);
    expect(ctl.scale, closeTo(fitted * 2, 1e-4));

    // With no view, zoomBy and panBy keep the engine of the last one; toView needs a view.
    await tester.pumpWidget(const SizedBox());
    final x = ctl.offsetX;
    ctl.panBy(const Offset(5, 0));
    expect(ctl.offsetX, closeTo(x + 5, 1e-4));
    expect(() => ctl.toView(Offset.zero), throwsStateError);
    ctl.dispose();
    p.dispose();
  });

  /// The painter of the one view on screen, and the size it paints at.
  (CustomPainter, Size) painterOf(WidgetTester tester) {
    final box = tester.renderObject<RenderCustomPaint>(
      find
          .descendant(
            of: find.byType(QvpPageView),
            matching: find.byType(CustomPaint),
          )
          .first,
    );
    return (box.painter!, box.size);
  }

  testWidgets('a frame reads the engine again after a change, and only then', (
    tester,
  ) async {
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: SizedBox(width: 400, height: 640, child: QvpPageView(page: p)),
      ),
    );
    final state = tester.state<QvpPageViewState>(find.byType(QvpPageView));
    expect(state.stats.bands, 0);
    expect(state.stats.overlayPaths, 0);

    // a highlight is a change: the next frame draws its bands and its ink
    final h = p.highlight('2:255', const QvpHighlightStyle(mode: 'both'));
    await tester.pump();
    expect(state.stats.bands, p.highlightBoxesView().length);
    expect(state.stats.bands, greaterThan(0));
    expect(state.stats.overlayPaths, p.styledPaths().length);
    expect(state.stats.overlayPaths, greaterThan(0));

    // a new layout moves the bands, though nothing on the page changed
    state.zoomToStep(1);
    await tester.pump();
    expect(state.stats.bands, p.highlightBoxesView().length);

    // and taking the highlight away takes its bands and its ink with it
    p.removeHighlight(h);
    await tester.pump(const Duration(milliseconds: 50));
    expect(state.stats.bands, 0);
    expect(state.stats.overlayPaths, 0);

    await tester.pumpWidget(const SizedBox());
    p.dispose();
  });

  testWidgets('benchmark: 200 frames of a page where nothing moves', (
    tester,
  ) async {
    final p = engine.loadPage(File(pagePath).readAsBytesSync());
    await tester.pumpWidget(
      Directionality(
        textDirection: TextDirection.ltr,
        child: SizedBox(width: 400, height: 640, child: QvpPageView(page: p)),
      ),
    );
    double medianUs(int frames) {
      final (painter, size) = painterOf(tester);
      final times = <int>[];
      final sw = Stopwatch();
      for (var i = 0; i < frames; i++) {
        final rec = ui.PictureRecorder();
        final canvas = ui.Canvas(rec);
        sw
          ..reset()
          ..start();
        painter.paint(canvas, size);
        sw.stop();
        times.add(sw.elapsedMicroseconds);
        rec.endRecording().dispose();
      }
      times.sort();
      return times[times.length ~/ 2].toDouble();
    }

    medianUs(20); // warm up
    final plain = medianUs(200);
    // what a reader following a recitation sees between two steps
    p.highlight('2:255', const QvpHighlightStyle(mode: 'both'));
    p.mask(T.words([0, 1, 2]), 'block');
    await tester.pump();
    medianUs(20);
    final styled = medianUs(200);
    debugPrint(
      'QvpBenchmark {"page":42,"staticFrameMedianUs":$plain,'
      '"staticStyledFrameMedianUs":$styled}',
    );
    expect(plain.isFinite && styled.isFinite, isTrue);

    await tester.pumpWidget(const SizedBox());
    p.dispose();
  });
}
