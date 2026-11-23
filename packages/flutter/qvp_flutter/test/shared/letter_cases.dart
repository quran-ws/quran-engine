// The letter layer replay: conformance/scenarios/letters.json through the public API, on
// both backends (qvp_flutter_test.dart and qvp_web_test.dart). The scenario and its hex
// fixtures are synthetic squares, no page data, so these cases never skip. No dart:io here:
// each runner reads the files its own way and hands them over in a [LetterFixture].
import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qvp_flutter/qvp_flutter.dart';

/// The engine and the files the letter cases read.
class LetterFixture {
  LetterFixture({
    required this.engine,
    required this.scenario,
    required this.hex,
    required this.maskTransition,
  });

  final QvpEngine engine;

  /// conformance/scenarios/letters.json
  final String scenario;

  /// Every `conformance/*.hex` file the scenario names, by file name, as text.
  final Map<String, String> hex;

  /// `qvp_mask_transition`, a declared gap of this wrapper (docs/API-PARITY.md): each
  /// runner calls the engine under its backend directly.
  final void Function(QvpPage page, int ms) maskTransition;
}

/// The file names of the hex fixtures letters.json names, for a runner to read.
List<String> letterHexFiles(String scenario) {
  final json = jsonDecode(scenario) as Map<String, dynamic>;
  return {
    for (final f in (json['fixtures'] as Map<String, dynamic>).values)
      for (final k in const ['page', 'letters']) (f as Map)[k] as String,
  }.toList();
}

/// The bytes of a hex text, whitespace ignored.
Uint8List fromHex(String text) {
  final h = text.replaceAll(RegExp(r'\s'), '');
  return Uint8List.fromList([
    for (var i = 0; i + 1 < h.length; i += 2)
      int.parse(h.substring(i, i + 2), radix: 16),
  ]);
}

/// Registers the letter cases. [load] runs once, before the first of them.
void letterCases(FutureOr<LetterFixture> Function() load) {
  late LetterFixture fx;
  late QvpEngine engine;
  late Map<String, dynamic> s;
  late double tolerance;

  setUpAll(() async {
    fx = await load();
    engine = fx.engine;
    s = jsonDecode(fx.scenario) as Map<String, dynamic>;
    tolerance = (s['tolerance'] as num).toDouble();
  });

  // ── inputs: -1 is QVP_NONE ──
  int none(Object? v) => (v as int) < 0 ? qvpNone : v;
  QvpSelector selector(Map<String, dynamic> j) => QvpSelector(
    j['selector'] as int,
    none(j['a']),
    none(j['b']),
    none(j['c']),
  );
  QvpTarget target(Map<String, dynamic> j) => QvpTarget(
    j['target'] as int,
    a: none(j['a']),
    b: none(j['b']),
    c: none(j['c']),
    words: [for (final w in j['words'] as List) w as int],
  );
  QvpLetterQuery query(Map<String, dynamic> j) => QvpLetterQuery(
    scope: target(j['scope'] as Map<String, dynamic>),
    text: j['text'] as int,
    mark: j['mark'] as int,
    part: j['part'] as int,
  );
  QvpHitOptions? options(Object? o) {
    if (o == null) return null;
    final m = o as Map<String, dynamic>;
    return QvpHitOptions(
      maxDistance: (m['maxDistance'] as num).toDouble(),
      gapBias: (m['gapBias'] as num).toDouble(),
      preferExact: m['preferExact'] as bool,
    );
  }

  QvpLayoutSpec spec(String name) {
    final j = (s['layouts'] as Map<String, dynamic>)[name] as Map;
    double f(String k) => (j[k] as num).toDouble();
    final r = j['reflow'] as Map?;
    return QvpLayoutSpec(
      viewportW: f('viewportW'),
      viewportH: f('viewportH'),
      padTop: f('padTop'),
      padBottom: f('padBottom'),
      padLeft: f('padLeft'),
      padRight: f('padRight'),
      reflow: r == null
          ? null
          : QvpReflowSpec(zoom: (r['zoom'] as num).toDouble()),
    );
  }

  // ── loading ──
  Map<String, dynamic> fixture(String name) =>
      (s['fixtures'] as Map<String, dynamic>)[name] as Map<String, dynamic>;
  Uint8List bytesOf(String file) =>
      fromHex(fx.hex[file] ?? (throw StateError('$file not read')));
  Uint8List layerOf(String name) => bytesOf(fixture(name)['letters'] as String);

  /// A fresh page of [name], with its layer and, when [words] is true, the sidecar.
  QvpPage open(String name, {bool layer = true, bool words = false}) {
    final p = engine.loadPage(bytesOf(fixture(name)['page'] as String));
    if (layer) expect(p.attachLetters(layerOf(name)), isTrue, reason: name);
    if (words) expect(p.attachWords(s['words'] as String), greaterThan(0));
    return p;
  }

  // ── answers ──
  void close(num got, num want, String what) => expect(
    (got - want).abs() <= tolerance * (want.abs() < 1 ? 1 : want.abs()),
    isTrue,
    reason: '$what: got $got, the engine says $want',
  );
  List<List<int>> drawList(QvpPage p) => [
    for (final d in p.layoutDrawList()) [d.path, d.placement],
  ];
  List<int> redOf(QvpPage p) {
    final red = s['red'] as int, c = p.colors();
    return [
      for (var i = 0; i < c.length; i++)
        if (c[i] == red) i,
    ];
  }

  /// nPaths as the page info, the table rows and the colours each give it.
  void samePathCount(QvpPage p, int want, String what) {
    expect(p.nPaths, want, reason: '$what nPaths');
    expect(p.table.length, want * 8, reason: '$what table rows');
    expect(p.colors().length, want, reason: '$what colours');
    expect(p.buildPaths().length, want, reason: '$what built paths');
  }

  // The scenario names a mark by its engine name; the wrapper gives the id and the name, and
  // the name is the one the engine's table gives that id.
  Map<String, Object?> member(QvpLetterMember m) {
    expect(
      m.markName,
      engine.markName(m.mark),
      reason: 'markName of ${m.mark}',
    );
    return {
      'path': m.path,
      'at': m.at,
      'mark': m.markName,
      'isBody': m.isBody,
      'isCut': m.isCut,
    };
  }

  void sameLetter(QvpLetter? got, Object? want, String what) {
    if (want == null) {
      expect(got, isNull, reason: what);
      return;
    }
    final w = want as Map<String, dynamic>;
    expect(got, isNotNull, reason: what);
    final l = got!;
    expect(
      {
        'word': l.word,
        'index': l.index,
        'at': l.at,
        'text': l.codePoint,
        'char': l.text,
        'nMembers': l.nMembers,
        'nMarks': l.nMarks,
        'moreAt': l.moreAt,
        'nMoreAt': l.nMoreAt,
        'hasInk': l.hasInk,
      },
      {
        for (final k in const [
          'word',
          'index',
          'at',
          'text',
          'char',
          'nMembers',
          'nMarks',
          'moreAt',
          'nMoreAt',
          'hasInk',
        ])
          k: w[k],
      },
      reason: what,
    );
    // a letter's text is its character, one Unicode scalar, and codePoint the same as a number
    expect(l.text.runes.length, 1, reason: '$what text is one character');
    expect(l.text.runes.first, l.codePoint, reason: '$what codePoint');
    close(l.x0, w['x0'] as num, '$what x0');
    close(l.y0, w['y0'] as num, '$what y0');
    close(l.x1, w['x1'] as num, '$what x1');
    close(l.y1, w['y1'] as num, '$what y1');
    // a sequence's letterInfo step gives the info alone, with no members
    if (!w.containsKey('members')) return;
    expect(l.members.map(member).toList(), w['members'], reason: what);
    // marks[n] is the nth mark Sel.letterMark takes: the members after the bodies
    expect(l.marks.map(member).toList(), [
      for (final m in w['members'] as List)
        if (!((m as Map)['isBody'] as bool)) m,
    ], reason: '$what marks');
  }

  void sameHit(QvpLetterHit? got, Object? want, String what) {
    if (want == null) {
      expect(got, isNull, reason: what);
      return;
    }
    final w = want as Map<String, dynamic>;
    expect(got, isNotNull, reason: what);
    final h = got!;
    expect(
      {
        'word': h.word,
        'letter': h.letter,
        'path': h.path,
        'at': h.at,
        'line': h.line,
        'mark': h.markName,
        'isExact': h.isExact,
        'isBody': h.isBody,
      },
      {
        for (final k in const [
          'word',
          'letter',
          'path',
          'at',
          'line',
          'mark',
          'isExact',
          'isBody',
        ])
          k: w[k],
      },
      reason: what,
    );
    expect(h.markName, engine.markName(h.mark), reason: '$what markName');
    close(h.distance, w['distance'] as num, '$what distance');
  }

  Object? pathLetter(({int letter, int at})? v) =>
      v == null ? null : {'letter': v.letter, 'at': v.at};
  List<List<int>> found(List<({int word, int letter})> v) => [
    for (final f in v) [f.word, f.letter],
  ];

  test('letters.json: the layers read as the engine says', () {
    final cases = s['letters'] as List;
    for (final c in cases.cast<Map<String, dynamic>>()) {
      final what = c['name'] as String;
      final p = open(c['fixture'] as String, words: c['words'] as bool);
      expect(p.hasLetters(), c['hasLetters'], reason: what);
      samePathCount(p, c['nPaths'] as int, what);
      final counts = c['counts'] as List;
      for (var w = 0; w < counts.length; w++) {
        expect(p.letterCount(w), counts[w], reason: '$what count $w');
      }
      final letters = (c['letters'] as List).cast<Map<String, dynamic>>();
      for (final l in letters) {
        final w = l['word'] as int, i = l['index'] as int;
        sameLetter(p.letter(w, i), l, '$what letter $w.$i');
        expect(
          p.letterMembers(w, i).map(member).toList(),
          l['members'],
          reason: '$what members $w.$i',
        );
      }
      // letters(w) is letter(w, i) for every letter of the word
      for (var w = 0; w < counts.length; w++) {
        expect(p.letters(w).map((l) => [l.word, l.index]).toList(), [
          for (final l in letters)
            if (l['word'] == w) [w, l['index']],
        ], reason: '$what letters($w)');
      }
      for (final m in (c['missing'] as List).cast<Map<String, dynamic>>()) {
        final w = m['word'] as int, i = m['letter'] as int;
        sameLetter(p.letter(w, i), m['info'], '$what missing $w.$i');
        expect(p.letterMembers(w, i), isEmpty, reason: '$what missing $w.$i');
      }
      for (final pl
          in (c['pathLetters'] as List).cast<Map<String, dynamic>>()) {
        expect(
          pathLetter(p.pathLetter(pl['path'] as int)),
          pl['value'],
          reason: '$what pathLetter ${pl['path']}',
        );
      }
      for (final o in (c['offsets'] as List).cast<List>()) {
        expect(
          p.letterAtOffset(o[0] as int, o[1] as int),
          o[2],
          reason: '$what letterAtOffset $o',
        );
      }
      p.dispose();
    }
    expect(cases.length, 4);
  });

  test(
    'letters.json: the letter selectors colour and draw as the engine says',
    () {
      final cases = s['selections'] as List;
      for (final c in cases.cast<Map<String, dynamic>>()) {
        final what = c['name'] as String;
        final p = open(c['fixture'] as String, words: c['words'] as bool);
        p.layout(spec('printed'));
        p.tick(0);
        final h = p.style(
          selector(c['selector'] as Map<String, dynamic>),
          s['red'] as int,
        );
        expect(h != 0, c['handle'], reason: '$what handle');
        p.tick(0);
        expect(redOf(p), c['red'], reason: '$what red');
        expect(
          p.layoutDrawRevision(),
          c['drawRevision'],
          reason: '$what revision',
        );
        expect(drawList(p), c['drawList'], reason: '$what draw list');
        p.dispose();
      }
      expect(cases.length, 42);
    },
  );

  test('the letter selectors are the four header codes', () {
    expect(Sel.letter(1, 2).code, 16);
    expect(Sel.letter(1, 2, QvpLetterPart.marks).argC, 2);
    expect(Sel.letterMark(1, 2, 3).code, 17);
    expect(Sel.letterMarkNamed(1, 2, 'omitted_alif').argC, 12);
    expect(Sel.letterMarkNamed(1, 2, 16).argC, 16);
    expect(Sel.wordChars(1, 3).argC, 3);
    expect(Sel.wordChars(1, 3, 5).code, 19);
    expect(const QvpLetterQuery(text: 'ل').codePoint, 0x644);
    expect(const QvpLetterQuery(mark: 'shaddah').markId, 7);
    expect(
      () => const QvpLetterQuery(text: 'لا').codePoint,
      throwsArgumentError,
    );
    expect(
      () => const QvpLetterQuery(text: '\uD800').codePoint,
      throwsArgumentError,
    );
    // '' is any letter, as 0 and a text left out are
    expect(const QvpLetterQuery(text: '').codePoint, 0);
    expect(const QvpLetterQuery().codePoint, 0);
    expect(const QvpLetterQuery(text: 0x644).codePoint, 0x644);
  });

  test(
    'letters.json: the letter queries find and colour as the engine says',
    () {
      final cases = s['queries'] as List;
      for (final c in cases.cast<Map<String, dynamic>>()) {
        final what = c['name'] as String;
        final p = open(c['fixture'] as String);
        p.layout(spec('printed'));
        p.tick(0);
        final q = query(c['query'] as Map<String, dynamic>);
        expect(found(p.findLetters(q)), c['found'], reason: '$what found');
        final h = p.styleLetters(q, s['red'] as int);
        expect(h != 0, c['handle'], reason: '$what handle');
        p.tick(0);
        expect(redOf(p), c['red'], reason: '$what red');
        expect(drawList(p), c['drawList'], reason: '$what draw list');
        p.dispose();
      }
      expect(cases.length, 15);
      // the friendly forms reach the same query: a character, a mark name, a string scope
      final p = open('letters');
      expect(found(p.findLetters(const QvpLetterQuery(text: 'ل'))), [
        [0, 1],
        [1, 0],
      ]);
      expect(
        found(p.findLetters(const QvpLetterQuery(mark: 'dot', scope: [0, 1]))),
        [
          [0, 0],
        ],
      );
      expect(found(p.findLetters(const QvpLetterQuery(scope: '1:2'))), [
        [2, 0],
      ]);
      p.dispose();
    },
  );

  test('letters.json: the letter hits match the engine', () {
    final cases = s['hits'] as List;
    for (final c in cases.cast<Map<String, dynamic>>()) {
      final what = c['name'] as String;
      final p = open(c['fixture'] as String, words: c['words'] as bool);
      final layout = c['layout'] as String?;
      if (layout != null) p.layout(spec(layout));
      p.tick(0);
      final x = (c['x'] as num).toDouble(), y = (c['y'] as num).toDouble();
      final o = options(c['options']);
      final got = c['units'] == 'page'
          ? p.hitTestLetter(x, y, o)
          : p.hitTestLetterView(x, y, o);
      sameHit(got, c['hit'], what);
      if (got != null) expect(got.wordKey, p.wordKey(got.word), reason: what);
      p.dispose();
    }
    expect(cases.length, 34);
  });

  test('letters.json: the call sequences replay as the engine says', () {
    final seqs = s['sequences'] as List;
    for (final seq in seqs.cast<Map<String, dynamic>>()) {
      final name = seq['name'] as String;
      final p = open(seq['fixture'] as String, layer: false);
      final handles = <String, int>{};
      var n = 0;
      for (final st in (seq['steps'] as List).cast<Map<String, dynamic>>()) {
        final call = st['call'] as String;
        final what = '$name, step ${n++} ($call)';
        Object? value() => st['value'];
        switch (call) {
          case 'tick':
            expect(
              p.tick((st['ms'] as num).toDouble()),
              st['animating'],
              reason: what,
            );
          case 'attach':
            expect(
              p.attachLetters(layerOf(st['letters'] as String)) ? 0 : -1,
              st['result'],
              reason: what,
            );
          case 'attachBytes':
            expect(
              p.attachLetters(fromHex(st['hex'] as String)) ? 0 : -1,
              st['result'],
              reason: what,
            );
          case 'attachWords':
            expect(
              p.attachWords(s['words'] as String),
              st['result'],
              reason: what,
            );
          case 'layout':
            p.layout(spec(st['spec'] as String));
          case 'style':
            final h = p.style(
              selector(st['selector'] as Map<String, dynamic>),
              st['rgba'] as int,
              ms: st['ms'] as int,
            );
            handles[st['as'] as String] = h;
            expect(h != 0, st['handle'], reason: what);
          case 'styleLetters':
            final h = p.styleLetters(
              query(st['query'] as Map<String, dynamic>),
              st['rgba'] as int,
              ms: st['ms'] as int,
            );
            handles[st['as'] as String] = h;
            expect(h != 0, st['handle'], reason: what);
          case 'remove':
            expect(
              p.removeStyle(handles[st['handle'] as String]!),
              st['result'],
              reason: what,
            );
          case 'theme':
            p.theme(
              QvpTheme(
                ink: st['ink'] as int,
                dots: st['dots'] as int,
                ms: st['ms'] as int,
              ),
            );
          case 'highlight':
            handles[st['as'] as String] = p.highlight(
              T.word(st['word'] as int),
              QvpHighlightStyle(
                mode: 'ink',
                ink: st['ink'] as int,
                ms: st['ms'] as int,
              ),
            );
          case 'removeHighlight':
            expect(
              p.removeHighlight(handles[st['handle'] as String]!),
              st['result'],
              reason: what,
            );
          case 'maskTransition':
            fx.maskTransition(p, st['ms'] as int);
          case 'mask':
            p.mask(T.word(st['word'] as int), 'hide');
          case 'unmask':
            p.unmask();
          case 'hasLetters':
            expect(p.hasLetters(), value(), reason: what);
          case 'nPaths':
            samePathCount(p, value() as int, what);
          case 'rows':
            final first = st['first'] as int, count = st['count'] as int;
            expect(
              [
                for (var i = first; i < first + count; i++)
                  p.table.sublist(i * 8, i * 8 + 8).toList(),
              ],
              value(),
              reason: what,
            );
          case 'colors':
            expect(p.colors().toList(), value(), reason: what);
          case 'styledPaths':
            expect(
              [
                for (final sp in p.styledPaths()) [sp.path, sp.color],
              ],
              value(),
              reason: what,
            );
          case 'drawRevision':
            expect(p.layoutDrawRevision(), value(), reason: what);
          case 'drawList':
            expect(drawList(p), value(), reason: what);
          case 'letterCount':
            expect(p.letterCount(st['word'] as int), value(), reason: what);
          case 'letterInfo':
            sameLetter(
              p.letter(st['word'] as int, st['letter'] as int),
              value(),
              what,
            );
          case 'letterMembers':
            expect(
              p
                  .letterMembers(st['word'] as int, st['letter'] as int)
                  .map(member)
                  .toList(),
              value(),
              reason: what,
            );
          case 'pathLetter':
            expect(
              pathLetter(p.pathLetter(st['path'] as int)),
              value(),
              reason: what,
            );
          case 'letterAtOffset':
            expect(
              p.letterAtOffset(st['word'] as int, st['at'] as int),
              value(),
              reason: what,
            );
          case 'findLetters':
            expect(
              found(p.findLetters(query(st['query'] as Map<String, dynamic>))),
              value(),
              reason: what,
            );
          case 'hitTestLetter' || 'hitTestLetterView':
            final x = (st['x'] as num).toDouble(),
                y = (st['y'] as num).toDouble();
            final o = options(st['options']);
            sameHit(
              call == 'hitTestLetter'
                  ? p.hitTestLetter(x, y, o)
                  : p.hitTestLetterView(x, y, o),
              value(),
              what,
            );
          default:
            fail('$what: a step this replay does not know');
        }
      }
      p.dispose();
    }
    expect(seqs.length, 11);
  });

  test('an attach refreshes the geometry; a refused one changes nothing', () {
    final p = open('letters', layer: false);
    final rows = p.table.sublist(0, 8 * 8).toList();
    final first = p.path(0);
    expect(p.nPaths, 8);
    expect(p.attachLetters(Uint8List.fromList([1, 2, 3])), isFalse);
    expect(p.nPaths, 8);
    expect(
      identical(p.path(0), first),
      isTrue,
      reason: 'a refusal keeps the paths',
    );
    final rev = p.revision;
    expect(p.attachLetters(layerOf('letters')), isTrue);
    expect(
      p.revision,
      greaterThan(rev),
      reason: 'an attach notifies the views',
    );
    samePathCount(p, 10, 'after the attach');
    expect(p.table.sublist(0, 8 * 8).toList(), rows);
    // the piece's outline is read from the new geometry: the ba, the right half of path 0
    final b = p.path(8).getBounds();
    close(b.left, 50, 'piece 8 left');
    close(b.right, 60, 'piece 8 right');
    p.dispose();
  });
}
