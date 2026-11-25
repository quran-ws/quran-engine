// The web backend: the shared cases of shared/api_cases.dart through qvp.js and the wasm
// engine, in a browser.
//
// flutter test --platform chrome test/qvp_web_test.dart
//
// Engine: assets/js/qvp.js and assets/wasm/qvp_ffi.wasm, the two files the package bundles
// (build them with scripts/sync-flutter-web-engine.sh).
// Data: <repo>/dist/pages/042.qvp, 042.words.json, atlas.qva,
// <repo>/conformance/scenarios/layout.json, dress.json and letters.json, the ornament set
// dress.json names and the hex fixtures that letters.json names, in <repo>/conformance/.
//
// The browser test server serves no package assets, and a browser cannot read the disk. So
// a hybrid isolate on the Dart VM reads the files and sends them over a channel. The test
// then puts qvp.js on the page and hands the wasm bytes to QvpEngine.initWeb, the two steps
// the plugin takes with its bundled assets in an app.
@TestOn('browser')
library;

import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qvp_flutter/qvp_flutter.dart';
// WebPageHandle: the replay reaches one wasm export qvp.js leaves out
import 'package:qvp_flutter/src/backend/qvp_backend_web.dart';

import 'shared/api_cases.dart';
import 'shared/letter_cases.dart';

/// Runs on the Dart VM. Sends {name: base64 or null} for each file the test reads. The
/// working directory is the package; the repository is QVP_REPO or three levels up.
const _readFiles = r'''
import 'dart:convert';
import 'dart:io';

import 'package:stream_channel/stream_channel.dart';

void hybridMain(StreamChannel<Object?> channel) {
  final repo = Platform.environment['QVP_REPO'] ?? '../../..';
  final files = {
    'qvp.js': 'assets/js/qvp.js',
    'qvp_ffi.wasm': 'assets/wasm/qvp_ffi.wasm',
    '042.qvp': '$repo/dist/pages/042.qvp',
    '042.words.json': '$repo/dist/pages/042.words.json',
    'atlas.qva': '$repo/dist/pages/atlas.qva',
    'layout.json': '$repo/conformance/scenarios/layout.json',
    'dress.json': '$repo/conformance/scenarios/dress.json',
    'qvo1-dress.hex': '$repo/conformance/qvo1-dress.hex',
    'letters.json': '$repo/conformance/scenarios/letters.json',
  };
  // the hex fixtures letters.json names, as text
  final letters = File(files['letters.json']!);
  if (letters.existsSync()) {
    final fixtures =
        (jsonDecode(letters.readAsStringSync()) as Map)['fixtures'] as Map;
    for (final f in fixtures.values) {
      for (final k in ['page', 'letters']) {
        final name = (f as Map)[k] as String;
        files[name] = '$repo/conformance/$name';
      }
    }
  }
  channel.sink.add({
    for (final e in files.entries)
      e.key: File(e.value).existsSync()
          ? base64Encode(File(e.value).readAsBytesSync())
          : null,
  });
  channel.sink.close();
}
''';

void main() {
  late Map<String, Uint8List?> files;
  late QvpEngine engine;
  late QvpPage page;

  Uint8List need(String name) {
    final bytes = files[name];
    expect(
      bytes,
      isNotNull,
      reason: '$name not found; see the comment at the top',
    );
    return bytes!;
  }

  setUpAll(() async {
    final sent = await spawnHybridCode(_readFiles).stream.first as Map;
    files = {
      for (final e in sent.entries)
        e.key as String: e.value == null
            ? null
            : base64Decode(e.value as String),
    };
    // The plugin injects its bundled qvp.js the same way.
    final document = globalContext['document'] as JSObject;
    final script = document.callMethod<JSObject>(
      'createElement'.toJS,
      'script'.toJS,
    );
    script['text'] = utf8.decode(need('qvp.js')).toJS;
    (document['head'] as JSObject).callMethod('appendChild'.toJS, script);
    expect(globalContext.has('QVP'), isTrue, reason: 'qvp.js did not run');

    engine = await QvpEngine.initWeb(wasmBytes: need('qvp_ffi.wasm'));
    page = engine.loadPage(need('042.qvp'));
  });

  tearDownAll(() {
    page.dispose();
    engine.dispose();
  });

  apiCases(
    () => ApiFixture(
      engine: engine,
      page: page,
      words042: files['042.words.json'] == null
          ? null
          : utf8.decode(files['042.words.json']!),
      atlas: files['atlas.qva'],
      layoutScenarios: utf8.decode(need('layout.json')),
      dressScenarios: utf8.decode(need('dress.json')),
      dressOrnaments: utf8.decode(need('qvo1-dress.hex')),
    ),
  );

  letterCases(() {
    final scenario = utf8.decode(need('letters.json'));
    return LetterFixture(
      engine: engine,
      scenario: scenario,
      hex: {for (final f in letterHexFiles(scenario)) f: utf8.decode(need(f))},
      // qvp.js leaves qvp_mask_transition out (a declared gap): call the wasm export
      maskTransition: (p, ms) {
        final js = (p.handle as WebPageHandle).js;
        ((js['e'] as JSObject)['ex'] as JSObject).callMethod(
          'qvp_mask_transition'.toJS,
          js['h'],
          ms.toJS,
        );
      },
    );
  });
}
