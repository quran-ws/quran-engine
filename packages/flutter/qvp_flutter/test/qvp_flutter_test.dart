// dart:ffi tests against the host build of libqvp_ffi.
//
// Library: $QVP_LIB, else <repo>/target/release/libqvp_ffi.so
// (build with `cargo build -p qvp-ffi --release`).
// Data: <repo>/dist/pages/042.qvp, 042.words.json, atlas.qva
// (`cargo run -p qvp-convert --release -- batch pages dist/pages`).
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

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
    if (File('${d.path}/crates/qvp-ffi/include/qvp.h').existsSync()) return d.path;
    d = d.parent;
  }
  throw StateError('repository root not found; set QVP_REPO');
}

void main() {
  final repo = _findRepo();
  final libPath = Platform.environment['QVP_LIB'] ?? '$repo/target/release/libqvp_ffi.so';
  final pagePath = '$repo/dist/pages/042.qvp';
  final wordsPath = '$repo/dist/pages/042.words.json';
  final atlasPath = '$repo/dist/pages/atlas.qva';

  late QvpEngine engine;
  late QvpPage page;

  setUpAll(() {
    expect(File(libPath).existsSync(), isTrue, reason: 'engine library not found at $libPath');
    expect(File(pagePath).existsSync(), isTrue, reason: 'page not found at $pagePath');
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
      expect(page.pathOpStart(i) + page.pathOpCount(i), lessThanOrEqualTo(page.ops.length));
      expect(page.pathPtStart(i) + page.pathPtCount(i), lessThanOrEqualTo(page.pts.length));
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
    expect(page.text('page'), contains(w0.text));
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
    expect(page.hitTest(-500, -500, const QvpHitOptions(maxDistance: 6)), isNull);
  });

  test('layout fillHeight 690×1100 pads 50 → scale 2.0, 15 lineDy; hitTestView hits word 0', () {
    final l = page.layout(const QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true));
    expect(l.scale, closeTo(2.0, 1e-5));
    expect(l.lineDy.length, 15);
    expect(l.slots.length, 15);
    expect(page.currentLayout, same(l));
    final w = page.words[0];
    final viewX = l.offsetX + (w.x0 + w.x1) / 2 * l.scale;
    final viewY = l.offsetY + ((w.y0 + w.y1) / 2 + l.lineDy[w.lineIndex]) * l.scale;
    final h = page.hitTestView(viewX, viewY, const QvpHitOptions(maxDistance: 6));
    expect(h, isNotNull);
    expect(h!.word, 0);
    final box = page.wordBoundsView(0);
    expect(box.x0, lessThan(viewX));
    expect(box.x1, greaterThan(viewX));
    expect(page.layoutLineSpacingToFill(const QvpLayoutSpec(viewportW: 600, viewportH: 1000)), greaterThanOrEqualTo(1));
    expect(page.grid.lines, 15);
    expect(page.layoutWastedFraction(const QvpLayoutSpec(viewportW: 600, viewportH: 1000)), inInclusiveRange(0, 1));
    expect(page.layoutPrintedHeight(const QvpLayoutSpec(viewportW: 600, viewportH: 1000)), greaterThan(0));
  });

  test('style(Sel.wordMark(w,1), colour) → styledPaths() has exactly 1 path', () {
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
    final th = page.theme(const QvpTheme(diacritics: '#1a73e8', dots: '#c62828', marks: {'shaddah': '#0a7d32'}));
    expect(page.styledPaths(), isNotEmpty);
    final hh = page.hide(Sel.kind(QvpKind.mark));
    expect(page.styledPaths().where((p) => (p.color & 0xff) == 0), isNotEmpty);
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
  });

  test("highlight('2:255', both, 200 ms): tick(100) true, 6 boxes, tick(1000) false", () {
    page.clearHighlights();
    final h = page.highlight('2:255', const QvpHighlightStyle(mode: 'both', ms: 200));
    expect(h, greaterThan(0));
    expect(page.tick(100), isTrue);
    final boxes = page.highlightBoxesView();
    expect(boxes.length, 6);
    expect(boxes.every((b) => b.id == h), isTrue);
    expect(page.highlightHandles(), [h]);
    expect(page.highlightWords(h).length, 50);
    expect(page.tick(1000), isFalse);
    expect(page.moveHighlight(h, T.word(0)), isTrue);
    expect(page.restyleHighlight(h, const QvpHighlightStyle(mode: 'band')), isTrue);
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
    final json = jsonDecode(File('$repo/conformance/scenarios/layout.json').readAsStringSync()) as Map<String, dynamic>;
    final tolerance = (json['tolerance'] as num).toDouble();
    final cases = json['cases'] as List<dynamic>;
    void close(double a, num b, String what) => expect((a - b).abs() <= tolerance * (b.abs() < 1 ? 1 : b.abs()), isTrue, reason: '$what: got $a, engine says $b');
    for (final c in cases) {
      final s = c['spec'] as Map<String, dynamic>, want = c['layout'] as Map<String, dynamic>;
      double f(String k) => (s[k] as num).toDouble();
      final spec = QvpLayoutSpec(
        viewportW: f('viewportW'), viewportH: f('viewportH'), padTop: f('padTop'), padBottom: f('padBottom'), padLeft: f('padLeft'), padRight: f('padRight'),
        lineSpacing: f('lineSpacing'), fillHeight: s['fillHeight'] as bool, gridLines: s['gridLines'] as int,
        cropLeft: f('cropLeft'), cropRight: f('cropRight'), maxAspectSlack: f('maxAspectSlack'));
      final tag = '${spec.viewportW}x${spec.viewportH} fill=${spec.fillHeight} slack=${spec.maxAspectSlack} crop=${spec.cropLeft}';
      final l = page.layout(spec);
      close(l.scale, want['scale'] as num, '$tag scale'); close(l.offsetX, want['offsetX'] as num, '$tag offsetX'); close(l.offsetY, want['offsetY'] as num, '$tag offsetY');
      close(l.contentW, want['contentW'] as num, '$tag contentW'); close(l.contentH, want['contentH'] as num, '$tag contentH'); close(l.lineSpacing, want['lineSpacing'] as num, '$tag lineSpacing');
      close(l.fitScale, want['fitScale'] as num, '$tag fitScale'); close(l.fitX, want['fitX'] as num, '$tag fitX'); close(l.fitY, want['fitY'] as num, '$tag fitY');
      close(l.lineDy.first, want['lineDy0'] as num, '$tag lineDy[0]'); close(l.lineDy.last, want['lineDyLast'] as num, '$tag lineDy[last]');
      close(page.layoutLineSpacingToFill(spec), c['lineSpacingToFill'] as num, '$tag lineSpacingToFill');
      close(page.layoutWastedFraction(spec), c['wastedFraction'] as num, '$tag wastedFraction');
    }
    expect(cases.length, 40);
  });

  test('the reader\'s zoom control reflows the page and one loop draws it', () {
    const spec = QvpLayoutSpec(viewportW: 390, viewportH: 844, padTop: 12, padBottom: 12, padLeft: 8, padRight: 8, surahFrames: false);

    // a zeroed control is stepped, on the printed page, and asks for no reflow when the host
    // gave it no knobs
    const printed = QvpZoom();
    expect(printed.mode, QvpZoomMode.stepped);
    expect(page.zoomSpec(spec, printed).reflow, isNull);
    expect(page.zoomSpec(spec, printed).surahFrames, isFalse);

    // the host's reflow knobs survive the printed page, with zoom 0, so the first pinch that
    // leaves it reflows with them and not with the engine's defaults
    final knobbed = spec.copyWith(reflow: const QvpReflowSpec(fill: 'justified', maxStretch: 3, wordGap: 1.5));
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

    // a pinch that has not reached the next step commits nothing
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
    final json = jsonDecode(File('$repo/conformance/scenarios/passage.json').readAsStringSync()) as Map<String, dynamic>;
    final tolerance = (json['tolerance'] as num).toDouble();
    final cases = json['cases'] as List<dynamic>;
    void close(double a, num b, String what) => expect((a - b).abs() <= tolerance, isTrue, reason: '$what: got $a, engine says $b');
    QvpPage load(int n) => engine.loadPage(File('$repo/dist/pages/${n.toString().padLeft(3, '0')}.qvp').readAsBytesSync());
    for (final c in cases) {
      final numbers = (c['pages'] as List<dynamic>).cast<int>();
      final surah = c['surah'] as int, from = c['from'] as int, to = c['to'] as int;
      final basmalah = c['basmalah'] as bool? ?? false;
      final pages = [for (final n in numbers) load(n)];
      final passage = basmalah ? QvpPassage.basmalah(pages, surah) : QvpPassage.load(pages, surah, from, to: to);
      // The passage keeps what it needs.
      for (final p in pages) {
        p.dispose();
      }
      final s = c['spec'] as Map<String, dynamic>, want = c['layout'] as Map<String, dynamic>;
      double f(String k) => (s[k] as num).toDouble();
      final spec = QvpPassageSpec(
        width: f('width'), scale: f('scale'), lineSpacing: f('lineSpacing'), padding: f('padding'),
        align: QvpPassageAlign.values.byName(s['align'] as String), maxRows: s['maxRows'] as int,
        keepAyahMark: s['keepAyahMark'] as bool, ellipsisWidth: f('ellipsisWidth'), maxStretch: f('maxStretch'));
      final tag = '${basmalah ? 'basmalah of ' : ''}$surah:$from-$to at ${spec.width} ${s['align']} rows ${spec.maxRows}';
      final l = passage.layout(spec);
      expect(l, isNotNull, reason: tag);
      expect(passage.currentLayout, same(l));
      close(l!.width, want['width'] as num, '$tag width'); close(l.height, want['height'] as num, '$tag height');
      close(l.scale, want['scale'] as num, '$tag scale'); close(l.lineSpacing, want['lineSpacing'] as num, '$tag lineSpacing');
      expect(l.isTruncated, want['isTruncated'] as bool, reason: tag);
      expect(l.draws.length, want['draws'] as int, reason: tag);
      final rows = want['rows'] as List<dynamic>, words = want['words'] as List<dynamic>;
      expect(l.rows.length, rows.length, reason: tag);
      expect(l.words.length, words.length, reason: tag);
      for (var i = 0; i < l.rows.length && i < rows.length; i++) {
        final r = l.rows[i], g = (rows[i] as List<dynamic>).cast<num>();
        close(r.x0, g[0], '$tag row $i x0'); close(r.y0, g[1], '$tag row $i y0');
        close(r.x1, g[2], '$tag row $i x1'); close(r.y1, g[3], '$tag row $i y1');
        close(r.baseline, g[4], '$tag row $i baseline');
      }
      for (var i = 0; i < l.words.length && i < words.length; i++) {
        final w = l.words[i], g = (words[i] as List<dynamic>).cast<num>();
        expect([w.page, w.ayah, w.word, w.row], g.sublist(0, 4).map((v) => v.toInt()).toList(), reason: '$tag word $i');
        close(w.x0, g[4], '$tag word $i x0'); close(w.y0, g[5], '$tag word $i y0');
        close(w.x1, g[6], '$tag word $i x1'); close(w.y1, g[7], '$tag word $i y1');
        close(w.wordX0, g[8], '$tag word $i wordX0'); close(w.wordY0, g[9], '$tag word $i wordY0');
        close(w.wordX1, g[10], '$tag word $i wordX1'); close(w.wordY1, g[11], '$tag word $i wordY1');
      }
      final e = (want['ellipsis'] as List<dynamic>?)?.cast<num>();
      if (e != null) {
        expect(l.ellipsis, isNotNull, reason: tag);
        close(l.ellipsis!.left, e[0], '$tag ellipsis x0'); close(l.ellipsis!.top, e[1], '$tag ellipsis y0');
        close(l.ellipsis!.right, e[2], '$tag ellipsis x1'); close(l.ellipsis!.bottom, e[3], '$tag ellipsis y1');
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
    QvpPage load(int n) => engine.loadPage(File('$repo/dist/pages/${n.toString().padLeft(3, '0')}.qvp').readAsBytesSync());
    final two = load(2), one = load(1), tawbah = load(187);
    final basmalah = QvpPassage.basmalah([two], 2);
    expect(basmalah.ayahs, isEmpty);
    final first = QvpPassage.load([two], 2, 1);
    expect(basmalah.lineSpacing, first.lineSpacing);
    expect(first.ayahs.single.ayahKey, '2:1');
    first.close();
    final l = basmalah.layout(QvpPassageSpec(width: 366, scale: 22 / basmalah.lineSpacing, align: QvpPassageAlign.center));
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

    final p = engine.loadPage(File('$repo/dist/pages/604.qvp').readAsBytesSync());
    p.layout(const QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 20, padBottom: 20));
    expect(p.dressInfo(), isNull);
    expect(p.dressRevision(), 0);
    expect(p.dressGeometry(), isEmpty);

    // a style takes all three kinds, and the border's fill is repainted red
    final info = p.dress(set, QvpDressSpec(style: 'test', ayahMarkScale: 1, colors: [(set.list[2], 0, 0xff0000ff)]))!;
    expect(p.dressSpec, isNotNull);
    expect((info.ayahMark, info.surahHeader, info.pageFrame), (0, 1, 2));
    expect(info.ayahMarks, greaterThan(0));
    expect(info.surahHeaders, greaterThan(0));
    expect(info.frameStretched, isTrue, reason: 'no slices in this set');
    expect(info.frameRepeats, 0);
    expect(info.viewBox[0], lessThan(0));
    expect(info.viewBox[2], greaterThan(p.width));
    expect(info.overflow.every((v) => v > 0), isTrue, reason: '${info.overflow}');
    expect(info.revision, greaterThan(0));
    expect(p.dressRevision(), info.revision);

    final draws = p.dressGeometry();
    expect(draws.length, info.draws);
    expect(draws.length, (info.ayahMarks + info.surahHeaders + 1) * 2, reason: 'one fill and one stroke per placed asset');
    expect(identical(p.dressGeometry(), draws), isTrue, reason: 'kept until the revision changes');
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
    p.layout(const QvpLayoutSpec(viewportW: 690, viewportH: 2000, padTop: 20, padBottom: 20, fillHeight: true));
    expect(p.dressRevision(), isNot(info.revision));
    expect(identical(p.dressGeometry(), draws), isFalse);

    // a kind key overrides the style, an index and a style name both work, and line art is a set
    final mixed = p.dress(set, const QvpDressSpec(style: 'test', ayahMark: 0, surahHeader: 'test', lineArt: {QvpOrnamentKind.ayahMark}, pageFrameScale: 1.5));
    expect(mixed, isNotNull);
    expect(p.dressGeometry().where((d) => d.kind == QvpOrnamentKind.ayahMark).every((d) => d.stroke), isTrue);

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

  testWidgets('a dressed page paints its ornament layer behind the ink', (tester) async {
    final set = engine.loadOrnaments(testOrnaments());
    final p = engine.loadPage(File('$repo/dist/pages/604.qvp').readAsBytesSync());
    await tester.pumpWidget(Directionality(textDirection: TextDirection.ltr, child: SizedBox(width: 400, height: 640, child: QvpPageView(page: p))));
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
    expect(p.dress(set, QvpDressSpec(style: 'test', colors: [(set.list[2], 0, 0xff0000ff)])), isNotNull);
    await tester.pump();
    expect(state.controller.scale, closeTo(l.fitScale * 2, 1e-6));
    expect((state.controller.offsetX, state.controller.offsetY), (-40, -60));
    // A bigger border grows the box, so the page is fitted again.
    expect(p.dress(set, const QvpDressSpec(style: 'test', pageFrameScale: 2, gap: 20)), isNotNull);
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
}
