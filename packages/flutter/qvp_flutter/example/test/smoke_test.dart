// The example's UI smoke test (docs/standards/TESTING.md): open a bundled page, tap a word,
// and check that the app highlights it. It runs on the host with the engine built for it.
//
// Library: $QVP_LIB, else <repo>/target/release/libqvp_ffi.so (cargo build -p qvp-ffi --release).
// Data: assets/pages, from scripts/sync-example-data.sh flutter.
// Run in this directory: QVP_LIB=<repo>/target/release/libqvp_ffi.so flutter test
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qvp_flutter/qvp_flutter.dart';
import 'package:qvp_flutter_example/main.dart';

void main() {
  testWidgets('open page 1, tap a word, and the app highlights it', (
    tester,
  ) async {
    // Wide enough for the side panel, which lists every section, the selection among them.
    tester.view.physicalSize = const Size(1280, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    // flutter_test draws every glyph as a square of the font size, which is wider than a
    // real font, so some rows of the side panel overflow in this test. The test ignores
    // those overflows and fails on every other error.
    final onError = FlutterError.onError;
    FlutterError.onError = (details) {
      if (!details.exceptionAsString().contains('A RenderFlex overflowed')) {
        onError?.call(details);
      }
    };
    addTearDown(() => FlutterError.onError = onError);

    await tester.pumpWidget(const MushafApp());
    // The engine and the first bundled page load from the asset bundle, outside the fake clock.
    QvpPage? page;
    for (var i = 0; i < 50 && page == null; i++) {
      await tester.runAsync(
        () => Future.delayed(const Duration(milliseconds: 20)),
      );
      await tester.pump();
      final views = find.byType(QvpPageView);
      if (views.evaluate().isNotEmpty) {
        page = tester.widget<QvpPageView>(views).page;
      }
    }
    expect(page, isNotNull, reason: 'the example did not open a page');
    final p = page!;
    expect(p.highlightHandles(), isEmpty);

    // Tap the middle of a word in the middle of the page, in the view's local space.
    final w = p.words.length ~/ 2;
    final c = tester
        .state<QvpPageViewState>(find.byType(QvpPageView))
        .controller;
    final b = p.wordBoundsView(w);
    final at =
        tester.getTopLeft(find.byType(QvpPageView)) +
        Offset(
          (b.x0 + b.x1) / 2 * c.scale + c.offsetX,
          (b.y0 + b.y1) / 2 * c.scale + c.offsetY,
        );
    await tester.tapAt(at);
    // A single tap resolves once the double-tap timeout has passed; then the fade runs.
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump(const Duration(milliseconds: 400));

    // One highlight, on the tapped word, with boxes to draw.
    final handles = p.highlightHandles();
    expect(handles, hasLength(1), reason: 'the tap made no highlight');
    expect(p.highlightWords(handles.single), [w]);
    expect(p.highlightBoxesView(), isNotEmpty);
    // The selection panel shows the word's key.
    expect(find.text(p.wordKey(w)), findsOneWidget);

    // A second tap on the same word clears the highlight, as on every platform.
    await tester.tapAt(at);
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text(p.wordKey(w)), findsNothing);

    // Dispose the app, and with it the engine, inside the test.
    await tester.pumpWidget(const SizedBox());
  });
}
