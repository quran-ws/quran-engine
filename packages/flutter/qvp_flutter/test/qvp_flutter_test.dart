// dart:ffi tests against the host build of libqvp_ffi.
//
// Library: $QVP_LIB, else <repo>/target/release/libqvp_ffi.so
// (build with `cargo build -p qvp-ffi --release`).
// Data: <repo>/dist/pages/042.qvp, 042.words.json, atlas.qva
// (`cargo run -p qvp-convert --release -- batch pages dist/pages`).
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qvp_flutter/qvp_flutter.dart';

const _repo = '/home/abdullah/Dev/github.com/quranpedia/quran-engine';

String _findRepo() {
  var d = Directory.current;
  for (var i = 0; i < 6; i++) {
    if (File('${d.path}/crates/qvp-ffi/include/qvp.h').existsSync()) return d.path;
    d = d.parent;
  }
  return _repo;
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
    expect(engine.version, greaterThan(0));
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
    expect(page.decos.length, page.nDecos);
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
    expect(page.naturalPitch, greaterThan(0));
    expect(page.surahs().map((s) => s.number), contains(2));
    expect(page.ayahKeys(), contains((2, 255)));
    expect(page.wordLabel(0), isNotEmpty);
    expect(page.ayahLabel(page.words[0].ayahIdx), isNotEmpty);
    expect(page.ayahMarks(), isNotEmpty);
    expect(page.lineBands().length, 15);
    expect(page.hitBoxes().length, 147);
    expect(page.text('page'), contains(w0.text));
  });

  test('search الله → 7 matches', () {
    final m = page.search('الله');
    expect(m.length, 7);
    expect(m.first.text, isNotEmpty);
    expect(m.first.wordKey, page.wordKey(m.first.word));
  });

  test("resolve('2:255') → 50 words; citation; ayahWordCount", () {
    final ws = page.resolve('2:255');
    expect(ws.length, 50);
    expect(page.resolve(T.ayah(2, 255)), ws);
    expect(page.resolve(ws), ws);
    expect(page.resolve('2:255:1'), [ws.first]);
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

  test("hitTestEx at word 0's bbox centre → word 0; exact inside its ink", () {
    final w = page.words[0];
    final cx = (w.x0 + w.x1) / 2, cy = (w.y0 + w.y1) / 2;
    final h = page.hitTestEx(cx, cy);
    expect(h, isNotNull);
    expect(h!.word, 0);
    expect(h.distance, 0);
    expect(h.wordKey, w.wordKey);
    expect(h.line, w.lineIdx);
    expect(page.hitTest(cx, cy)?.word, 0);
    // the bbox centre can fall between glyphs: probe the bbox for a point inside the outline
    QvpHitEx? inside;
    for (var y = w.y0; y <= w.y1 && inside == null; y += 0.5) {
      for (var x = w.x0; x <= w.x1; x += 0.5) {
        final e = page.hitTestEx(x, y);
        if (e != null && e.word == 0 && e.exact) {
          inside = e;
          break;
        }
      }
    }
    expect(inside, isNotNull, reason: 'no point of word 0 is inside its ink');
    expect(inside!.path, greaterThanOrEqualTo(0));
    expect(page.pathWord(inside.path), 0);
    // far outside the page, with a distance cap → nothing
    expect(page.hitTestEx(-500, -500, const QvpHitOptions(maxDistance: 6)), isNull);
  });

  test('layout fillHeight 690×1100 pads 50 → scale 2.0, 15 lineDy; hitTestViewEx hits word 0', () {
    final l = page.layout(const QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true));
    expect(l.scale, closeTo(2.0, 1e-5));
    expect(l.lineDy.length, 15);
    expect(l.slots.length, 15);
    expect(page.currentLayout, same(l));
    final w = page.words[0];
    final vx = l.ox + (w.x0 + w.x1) / 2 * l.scale;
    final vy = l.oy + ((w.y0 + w.y1) / 2 + l.lineDy[w.lineIdx]) * l.scale;
    final h = page.hitTestViewEx(vx, vy, const QvpHitOptions(maxDistance: 6));
    expect(h, isNotNull);
    expect(h!.word, 0);
    final box = page.wordBoxView(0);
    expect(box.x0, lessThan(vx));
    expect(box.x1, greaterThan(vx));
    expect(engine.gapToFill(page.width, page.height, page.nLines, 600, 1000), isA<double>());
    expect(engine.wastedFraction(page.width, page.height, 600, 1000), inInclusiveRange(0, 1));
  });

  test('style(Sel.wordMark(w,1), colour) → styled() has exactly 1 path', () {
    page.clearStyles();
    expect(page.styled(), isEmpty);
    final w = 0;
    final h = page.style(Sel.wordMark(w, 1), '#ef6c00');
    expect(h, greaterThan(0));
    final s = page.styled();
    expect(s.length, 1);
    expect(s.first.color, 0xef6c00ff);
    expect(page.pathWord(s.first.path), w);
    expect(page.pathNthMark(s.first.path), 1);
    expect(page.colorOf(s.first.path), 0xef6c00ff);
    expect(page.paint()[s.first.path], 0xef6c00ff);
    expect(page.styleHandles(), contains(h));
    expect(page.unstyle(h), greaterThan(0));
    expect(page.styled(), isEmpty);
    // theme + hide + target styling under handles
    final th = page.theme(const QvpTheme(diacritics: '#1a73e8', dots: '#c62828', marks: {'shaddah': '#0a7d32'}));
    expect(page.styled(), isNotEmpty);
    final hh = page.hide(Sel.kind(QvpKind.mark));
    expect(page.styled().where((p) => (p.color & 0xff) == 0), isNotEmpty);
    page.unstyle(hh);
    page.unstyle(th);
    final ht = page.styleTarget('2:255', '#0a7d32', layer: QvpLayer.top);
    expect(page.styled().length, greaterThan(50));
    page.unstyle(ht);
    expect(page.styled(), isEmpty);
    page.setDefaultInk('#3b2a14');
    expect(page.defaultInk, 0x3b2a14ff);
    expect(page.paint()[0], 0x3b2a14ff);
    page.setDefaultInk('#231f20');
  });

  test("highlight('2:255', both, 200 ms): tick(100) true, 6 boxes, tick(1000) false", () {
    page.clearHighlights();
    final h = page.highlight('2:255', const QvpHighlightStyle(mode: 'both', ms: 200));
    expect(h, greaterThan(0));
    expect(page.tick(100), isTrue);
    final boxes = page.highlightBoxes();
    expect(boxes.length, 6);
    expect(boxes.every((b) => b.id == h), isTrue);
    expect(page.highlightHandles(), [h]);
    expect(page.highlightWords(h).length, 50);
    expect(page.tick(1000), isFalse);
    expect(page.rehighlight(h, T.word(0)), isTrue);
    expect(page.restyleHighlight(h, const QvpHighlightStyle(mode: 'band')), isTrue);
    expect(page.unhighlight(h), isTrue);
    page.tick(5000);
    page.clearHighlights();
    expect(page.highlightHandles(), isEmpty);
    expect(page.bandBoxes(page.resolve('2:255')).length, 6);
  });

  test("mask('2:255'): 50 hidden, revealNext → 49; reveal steps", () {
    page.mask('2:255');
    expect(page.maskHidden().length, 50);
    expect(page.maskWords().length, 50);
    expect(page.revealNext(1), 1);
    expect(page.maskHidden().length, 49);
    expect(page.hideBack(1), 1);
    expect(page.maskHidden().length, 50);
    page.unmask();
    expect(page.maskHidden(), isEmpty);
    page.mask('2:255', 'block');
    expect(page.maskBoxes(), isNotEmpty);
    page.unmask();
    final steps = page.revealStart(lit: 2, ms: 0);
    expect(steps, greaterThan(0));
    expect(page.revealSteps(), steps);
    expect(page.revealAt(), -1);
    expect(page.revealGoto(0), isTrue);
    expect(page.revealAt(), 0);
    expect(page.revealStepOf(0), greaterThanOrEqualTo(0));
    page.revealStop();
    expect(page.revealAt(), isNull);
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
    final box = page.cropBox('2:255');
    expect(box, isNotNull);
    expect(box!.nWords, 50);
  });

  test('atlas: pageOf(2,255) == 42, pagesOfJuz(30) == [582, 604], findSurah(cow)[0].n == 2', () {
    if (!File(atlasPath).existsSync()) return;
    final atlas = engine.loadAtlas(File(atlasPath).readAsBytesSync());
    expect(atlas.pages, 604);
    expect(atlas.pageOf(2, 255), 42);
    expect(atlas.pagesOfJuz(30), [582, 604]);
    final cow = atlas.findSurah('cow');
    expect(cow, isNotEmpty);
    expect(cow.first.n, 2);
    expect(atlas.surah(36)!.latin, isNotEmpty);
    expect(atlas.surahs().length, 114);
    expect(atlas.pageOfSurah(36), atlas.surah(36)!.page);
    expect(atlas.juz(30)!.page, 582);
    expect(atlas.juzAt(2, 255), 3);
    expect(atlas.pageRange(42)!.first.$1, 2);
    atlas.dispose();
  });
}
