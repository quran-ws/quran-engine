// QvpPageView — the Flutter host renderer for a QvpPage.
//
// Draw order per frame (same as web/qvp.js CanvasRenderer):
//   highlight bands → cached base ink → styled paths → mask boxes
// The engine decides everything (layout, hit-testing, colours, boxes); this
// widget only converts geometry into ui.Path objects once, caches the
// unstyled ink as an image at the current transform, and forwards gestures
// to the engine's hit test / selection.
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'engine.dart';

/// Layout parameters of [QvpPageView] (viewport size is taken from the widget's constraints).
/// Spacing only opens up: `lineSpacing` < 1 is clamped by the engine.
/// Pinch limits as multiples of the fitted scale; the same pair on every platform.
const double kMinZoom = 0.5;
const double kMaxZoom = 12;

@immutable
class QvpViewLayout {
  const QvpViewLayout({this.padTop = 24, this.padBottom = 24, this.padSide = 16, this.lineSpacing = 1, this.fillHeight = false, this.gridLines = 0, this.surahFrames = true});
  final double padTop, padBottom, padSide, lineSpacing;
  final bool fillHeight, surahFrames;
  final int gridLines;

  QvpViewLayout copyWith({double? padTop, double? padBottom, double? padSide, double? lineSpacing, bool? fillHeight, int? gridLines, bool? surahFrames}) => QvpViewLayout(
        padTop: padTop ?? this.padTop,
        padBottom: padBottom ?? this.padBottom,
        padSide: padSide ?? this.padSide,
        lineSpacing: lineSpacing ?? this.lineSpacing,
        fillHeight: fillHeight ?? this.fillHeight,
        gridLines: gridLines ?? this.gridLines,
        surahFrames: surahFrames ?? this.surahFrames,
      );

  QvpLayoutSpec toSpec(double viewportW, double viewportH) => QvpLayoutSpec(
        viewportW: viewportW,
        viewportH: viewportH,
        padTop: padTop,
        padBottom: padBottom,
        padLeft: padSide,
        padRight: padSide,
        lineSpacing: lineSpacing,
        fillHeight: fillHeight,
        gridLines: gridLines,
        maxAspectSlack: QvpDefaults.aspectSlack,
        surahFrames: surahFrames,
      );

  @override
  bool operator ==(Object other) =>
      other is QvpViewLayout &&
      other.padTop == padTop &&
      other.padBottom == padBottom &&
      other.padSide == padSide &&
      other.lineSpacing == lineSpacing &&
      other.fillHeight == fillHeight &&
      other.gridLines == gridLines &&
      other.surahFrames == surahFrames;

  @override
  int get hashCode => Object.hash(padTop, padBottom, padSide, lineSpacing, fillHeight, gridLines, surahFrames);
}

/// Pan / zoom on top of the engine layout. `view px = offsetX + layout px * scale`.
class QvpViewController extends ChangeNotifier {
  double scale = 1, offsetX = 0, offsetY = 0;
  bool _fitRequested = true;

  /// Size of the last laid-out viewport (logical px).
  Size viewport = Size.zero;

  void set({double? scale, double? offsetX, double? offsetY}) {
    this.scale = scale ?? this.scale;
    this.offsetX = offsetX ?? this.offsetX;
    this.offsetY = offsetY ?? this.offsetY;
    notifyListeners();
  }

  /// Repaints views using this controller.
  void refresh() => notifyListeners();

  /// Fits the page into the viewport on the next frame.
  void fit() {
    _fitRequested = true;
    notifyListeners();
  }

  /// Zooms by [factor] around a point in widget coordinates (default: centre).
  void zoomBy(double factor, {Offset? around, double minScale = kMinZoom, double maxScale = kMaxZoom}) {
    final c = around ?? Offset(viewport.width / 2, viewport.height / 2);
    final ns = (scale * factor).clamp(minScale, maxScale);
    final k = ns / scale;
    set(scale: ns, offsetX: c.dx - (c.dx - offsetX) * k, offsetY: c.dy - (c.dy - offsetY) * k);
  }

  /// Widget coordinates → layout viewport px (what the engine's `*View` calls take).
  Offset toView(Offset local) => Offset((local.dx - offsetX) / scale, (local.dy - offsetY) / scale);
}

/// Renders a [QvpPage] with the engine's layout, animated highlights, masks,
/// styled paths, tap / long-press selection and pinch / pan.
class QvpPageView extends StatefulWidget {
  const QvpPageView({
    super.key,
    required this.page,
    this.layout = const QvpViewLayout(),
    this.paper = const Color(0xfffffdf7),
    this.defaultInk,
    this.controller,
    this.onWordTap,
    this.onDecorationTap,
    this.onEmptyTap,
    this.onSelectionChanged,
    this.hitMaxDistance = QvpDefaults.tapDistance,
    this.selectionEnabled = true,
    this.panZoomEnabled = true,
    this.selectionBand = QvpDefaults.selectionBand,
    this.paperShadow = true,
    this.minScale = kMinZoom,
    this.maxScale = kMaxZoom,
    this.onZoomChanged,
  });

  final QvpPage page;
  final QvpViewLayout layout;

  /// The size the reader is at changed: a pinch committed, or a step was asked for. The page
  /// reflows as they zoom, so this is where a host redraws its own furniture.
  final void Function(QvpZoom)? onZoomChanged;

  /// Paper colour behind the ink (null = transparent).
  final Color? paper;

  /// Default ink; applied to the page with `setDefaultColor` when it changes. Anything [rgba] accepts.
  final Object? defaultInk;
  final QvpViewController? controller;

  /// Tap on a word (gap-aware). [hit] carries path / decoration / line / distance.
  final void Function(int word, QvpHit hit)? onWordTap;

  /// Tap on a decoration (ayah mark, surah banner, …) that is not a word.
  final void Function(QvpDecorationInfo decoration)? onDecorationTap;
  final VoidCallback? onEmptyTap;

  /// Whole-word selection changed through long-press-drag (empty list = cleared).
  final void Function(List<int> words)? onSelectionChanged;

  /// Max distance (page units) for the gap-aware tap.
  final double hitMaxDistance;
  final bool selectionEnabled, panZoomEnabled, paperShadow;

  /// Band colour of the drag selection (0xRRGGBBAA), drawn in [QvpLayer.selection].
  final int selectionBand;
  final double minScale, maxScale;

  @override
  State<QvpPageView> createState() => QvpPageViewState();
}

class QvpPageViewState extends State<QvpPageView> with SingleTickerProviderStateMixin {
  late QvpViewController _ctl;
  late final Ticker _ticker;
  final Stopwatch _clock = Stopwatch()..start();
  final ValueNotifier<int> _frame = ValueNotifier(0);
  final _InkCache _cache = _InkCache();
  Size _size = Size.zero;
  QvpViewLayout? _laidOut;
  int _selHandle = 0, _selAnchor = -1;
  bool _selecting = false;

  // scale gesture
  double _gScale = 1, _gOx = 0, _gOy = 0;
  /// Where the reader's zoom control stands, and where it stood when a pinch began.
  QvpZoom _zoom = const QvpZoom();
  QvpZoom _gZoom = const QvpZoom();

  QvpViewController get controller => _ctl;
  QvpPage get page => widget.page;

  /// True while the engine is animating.
  bool get animating => _ticker.isActive;

  @override
  void initState() {
    super.initState();
    _ctl = widget.controller ?? QvpViewController();
    _ticker = createTicker(_onTick);
    _attachPage();
    _applyInk();
  }

  void _attachPage() {
    widget.page.addListener(_onPageChanged);
    _cache.attach(widget.page);
    _selHandle = 0;
    _selAnchor = -1;
    _laidOut = null;
    _ctl._fitRequested = true;
  }

  void _applyInk() {
    final ink = widget.defaultInk;
    if (ink != null && rgba(ink) != widget.page.defaultInk) widget.page.setDefaultColor(ink);
  }

  @override
  void didUpdateWidget(QvpPageView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) _ctl = widget.controller ?? (oldWidget.controller == null ? _ctl : QvpViewController());
    if (oldWidget.page != widget.page) {
      oldWidget.page.removeListener(_onPageChanged);
      _attachPage();
    }
    if (oldWidget.defaultInk != widget.defaultInk) _applyInk();
    if (oldWidget.layout != widget.layout) _ctl._fitRequested = true;
  }

  @override
  void dispose() {
    widget.page.removeListener(_onPageChanged);
    _ticker.dispose();
    _cache.dispose();
    _frame.dispose();
    if (widget.controller == null) _ctl.dispose();
    super.dispose();
  }

  void _onPageChanged() {
    // the app cleared the selection behind our back → drop our band
    if (_selHandle != 0 && !_selecting && !page.isDisposed && page.selection().isEmpty) {
      final h = _selHandle;
      _selHandle = 0;
      page.removeHighlight(h);
      return; // removeHighlight notifies again
    }
    _ensureTicking();
  }

  void _ensureTicking() {
    if (!_ticker.isActive && mounted) _ticker.start();
  }

  double get nowMs => _clock.elapsedMicroseconds / 1000;

  void _onTick(Duration _) {
    if (page.isDisposed) {
      _ticker.stop();
      return;
    }
    final more = page.tick(nowMs);
    _frame.value++;
    if (!more) _ticker.stop();
  }

  // ── layout & fit (engine layout, then pan/zoom on top) ──
  /// The spec the page is laid out under: the reader's layout, with the size their zoom control
  /// asks for written onto it. The engine decides what that is.
  QvpLayoutSpec _spec(Size size) {
    final base = widget.layout.toSpec(size.width, size.height);
    return page.zoomSpec(base, _zoom);
  }

  void _relayout(Size size) {
    page.layout(_spec(size));
    _laidOut = widget.layout;
    _size = size;
    _ctl.viewport = size;
  }

  void _fit(Size size) {
    final l = page.currentLayout!;
    // A reflowed page is taller than the screen because the reader asked for it: show the top
    // and let them scroll, rather than shrinking it back to fit.
    _ctl.scale = l.reflowed ? 1 : l.fitScale;
    _ctl.offsetX = l.fitX;
    _ctl.offsetY = l.reflowed ? 0 : l.fitY;
    _ctl._fitRequested = false;
  }

  /// Re-runs the engine layout and fits the page.
  void fit() {
    if (_size == Size.zero) return;
    _relayout(_size);
    _fit(_size);
    _ctl.refresh();
  }

  // ── gestures ──
  QvpHit? _hitAt(Offset local, {double? maxDistance}) {
    final v = _ctl.toView(local);
    return page.hitTestView(v.dx, v.dy, QvpHitOptions(maxDistance: maxDistance ?? 0));
  }

  void _onTapUp(TapUpDetails d) {
    final h = _hitAt(d.localPosition, maxDistance: widget.hitMaxDistance);
    if (h != null && h.word >= 0) {
      widget.onWordTap?.call(h.word, h);
    } else if (h != null && h.decoration >= 0) {
      widget.onDecorationTap?.call(page.decorations[h.decoration]);
    } else {
      widget.onEmptyTap?.call();
    }
  }

  void _onScaleStart(ScaleStartDetails d) {
    _gScale = _ctl.scale;
    _gOx = _ctl.offsetX;
    _gOy = _ctl.offsetY;
    // where the control stood when the fingers went down: the engine measures them against this,
    // so a slow pinch walks the steps one at a time instead of running away through them
    _gZoom = _zoom;
  }

  void _onScaleUpdate(ScaleUpdateDetails d) {
    if (!widget.panZoomEnabled || _size == Size.zero) return;
    final f = d.localFocalPoint;
    // The engine owns the whole pinch: which step it lands on, when it commits, and what holds
    // the reader's place when the page reflows under the fingers.
    final c = page.zoomPinch(
      widget.layout.toSpec(_size.width, _size.height),
      _gZoom,
      QvpView(scale: _gScale, offsetX: _gOx, offsetY: _gOy),
      d.scale,
      f.dx,
      f.dy,
    );
    if (c.relaid) {
      _zoom = c.zoom;
      page.readLayout();
      widget.onZoomChanged?.call(c.zoom);
    }
    _ctl.set(scale: c.view.scale, offsetX: c.view.offsetX, offsetY: c.view.offsetY);
  }

  /// Move the control to one of the page's steps: 0 is the printed page.
  void zoomToStep(int step) {
    if (_size == Size.zero) return;
    final c = page.zoomToStep(
      widget.layout.toSpec(_size.width, _size.height),
      _zoom,
      step,
      QvpView(scale: _ctl.scale, offsetX: _ctl.offsetX, offsetY: _ctl.offsetY),
    );
    setState(() => _zoom = c.zoom);
    page.readLayout();
    _ctl.set(scale: c.view.scale, offsetX: c.view.offsetX, offsetY: c.view.offsetY);
    widget.onZoomChanged?.call(c.zoom);
  }

  /// Where the reader's zoom control stands.
  QvpZoom get zoom => _zoom;

  void _onLongPressStart(LongPressStartDetails d) {
    if (!widget.selectionEnabled) return;
    final h = _hitAt(d.localPosition, maxDistance: widget.hitMaxDistance);
    if (h == null || h.word < 0) return;
    _selecting = true;
    _selAnchor = h.word;
    HapticFeedback.selectionClick();
    _updateSelection(h.word);
  }

  void _onLongPressMove(LongPressMoveUpdateDetails d) {
    if (!_selecting) return;
    final h = _hitAt(d.localPosition);
    if (h != null && h.word >= 0) _updateSelection(h.word);
  }

  void _updateSelection(int focus) {
    page.select(_selAnchor, focus);
    final ws = page.selection();
    if (_selHandle != 0) {
      page.moveHighlight(_selHandle, T.words(ws));
    } else {
      _selHandle = page.highlight(T.words(ws), QvpHighlightStyle(mode: 'band', band: widget.selectionBand, padX: 0.6, ms: 0, layer: QvpLayer.selection));
    }
  }

  void _onLongPressEnd(LongPressEndDetails d) {
    if (!_selecting) return;
    _selecting = false;
    widget.onSelectionChanged?.call(page.selection());
  }

  /// Clears the drag selection and its band.
  void clearSelection() {
    if (_selHandle != 0) {
      page.removeHighlight(_selHandle);
      _selHandle = 0;
    }
    page.clearSelection();
    widget.onSelectionChanged?.call(const []);
  }

  @override
  Widget build(BuildContext context) {
    final dpr = MediaQuery.devicePixelRatioOf(context);
    return LayoutBuilder(builder: (context, c) {
      final size = Size(c.maxWidth.isFinite ? c.maxWidth : 400, c.maxHeight.isFinite ? c.maxHeight : 600);
      if (size != _size || _laidOut != widget.layout || page.currentLayout == null) {
        _relayout(size);
        _ctl._fitRequested = true;
      }
      if (_ctl._fitRequested) _fit(size);
      return ClipRect(
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapUp: _onTapUp,
          onDoubleTap: fit,
          onScaleStart: _onScaleStart,
          onScaleUpdate: _onScaleUpdate,
          onLongPressStart: _onLongPressStart,
          onLongPressMoveUpdate: _onLongPressMove,
          onLongPressEnd: _onLongPressEnd,
          child: RepaintBoundary(
            child: CustomPaint(
              size: size,
              isComplex: true,
              painter: _QvpPainter(
                page: page,
                view: _ctl,
                frame: _frame,
                cache: _cache,
                dpr: dpr,
                paper: widget.paper,
                paperShadow: widget.paperShadow,
              ),
            ),
          ),
        ),
      );
    });
  }
}

/// Per-page render cache: ui.Path per engine path (page units) and the base ink image.
class _InkCache {
  QvpPage? page;
  List<ui.Path>? paths;
  ui.Image? image;
  ui.Picture? picture;
  String key = '';

  /// Stats of the last frame (for HUDs).
  int basePaths = 0, overlayPaths = 0, bands = 0;
  double baseMs = 0, overlayMs = 0;

  void attach(QvpPage p) {
    if (page == p) return;
    page = p;
    paths = null;
    key = '';
    _dropImage();
  }

  void _dropImage() {
    image?.dispose();
    image = null;
    picture?.dispose();
    picture = null;
  }

  void dispose() {
    _dropImage();
    paths = null;
    page = null;
  }

  List<ui.Path> buildPaths(QvpPage p) {
    final cached = paths;
    if (cached != null) return cached;
    final t = p.table, ops = p.ops, pts = p.pts;
    final out = List<ui.Path>.generate(p.nPaths, (i) {
      final path = ui.Path();
      path.fillType = p.pathEvenOdd(i) ? PathFillType.evenOdd : PathFillType.nonZero;
      var o = t[i * 8];
      final oe = o + t[i * 8 + 1];
      var k = t[i * 8 + 2];
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
      return path;
    }, growable: false);
    return paths = out;
  }
}

class _QvpPainter extends CustomPainter {
  _QvpPainter({required this.page, required this.view, required ValueListenable<int> frame, required this.cache, required this.dpr, required this.paper, required this.paperShadow})
      : super(repaint: Listenable.merge([page, view, frame]));

  final QvpPage page;
  final QvpViewController view;
  final _InkCache cache;
  final double dpr;
  final Color? paper;
  final bool paperShadow;

  /// Put the canvas under one placement of the draw list: page units in, logical px out. A
  /// reflowed page moves words between rows, so a path is placed by the group the engine put it
  /// in and not by the line it was printed on.
  void _placeAt(ui.Canvas c, QvpLayout? l, QvpPlacement q) {
    final ls = l?.scale ?? 1;
    final s = view.scale * ls;
    c.translate(view.offsetX + view.scale * ((l?.offsetX ?? 0) + q.dx * ls), view.offsetY + view.scale * ((l?.offsetY ?? 0) + q.dy * ls));
    c.scale(s * q.kx, s * q.ky);
  }

  void _drawBoxes(ui.Canvas c, List<QvpBox> boxes) {
    if (boxes.isEmpty) return;
    c.save();
    c.translate(view.offsetX, view.offsetY);
    c.scale(view.scale);
    ui.Path? cur;
    var id = -1, col = 0;
    final paint = ui.Paint()..isAntiAlias = true;
    void flush() {
      if (cur != null) {
        paint.color = QvpColor.toColor(col);
        c.drawPath(cur!, paint);
        cur = null;
      }
    }

    for (final b in boxes) {
      if (cur == null || b.id != id || b.color != col) {
        flush();
        cur = ui.Path()..fillType = PathFillType.nonZero;
        id = b.id;
        col = b.color;
      }
      final r = Rect.fromLTRB(b.x0, b.y0, b.x1, b.y1);
      if (b.radius > 0) {
        cur!.addRRect(RRect.fromRectAndRadius(r, Radius.circular(b.radius)));
      } else {
        cur!.addRect(r);
      }
    }
    flush();
    c.restore();
  }

  @override
  void paint(ui.Canvas canvas, Size size) {
    if (page.isDisposed) return;
    final paths = cache.buildPaths(page);
    final l = page.currentLayout;
    final styled = page.styledPaths();
    final styledSet = <int>{for (final s in styled) s.path};
    final ink = page.defaultInk;

    // paper
    final paperColor = paper;
    if (paperColor != null) {
      final rect = Rect.fromLTWH(view.offsetX, view.offsetY, (l?.contentW ?? page.width) * view.scale, (l?.contentH ?? page.height) * view.scale);
      if (paperShadow) canvas.drawShadow(ui.Path()..addRect(rect), const Color(0x66000000), 6, false);
      canvas.drawRect(rect, ui.Paint()..color = paperColor);
    }

    // base ink cache: every non-styled path at the current transform
    final ids = styledSet.toList()..sort();
    final key = '${view.scale.toStringAsFixed(5)}|${view.offsetX.toStringAsFixed(2)}|${view.offsetY.toStringAsFixed(2)}|$dpr|$ink|${identityHashCode(l)}|${size.width}x${size.height}|${ids.join(',')}';
    if (key != cache.key || (cache.image == null && cache.picture == null)) {
      final sw = Stopwatch()..start();
      final rec = ui.PictureRecorder();
      final c = ui.Canvas(rec);
      c.scale(dpr);
      final paint = ui.Paint()
        ..color = QvpColor.toColor(ink)
        ..isAntiAlias = true;
      // The engine says what this layout draws and where: `{path, placement}` pairs in drawing
      // order, with omitted paths already gone and a repeated one appearing twice. One loop
      // draws any page, printed or reflowed.
      final draws = page.layoutDrawList();
      final places = page.layoutPlacements();
      var cur = -1, n = 0, open = false;
      for (final d in draws) {
        if (styledSet.contains(d.path)) continue;
        if (d.placement != cur) {
          if (open) c.restore();
          c.save();
          _placeAt(c, l, d.placement < places.length ? places[d.placement] : QvpPlacement.identity);
          open = true;
          cur = d.placement;
        }
        c.drawPath(paths[d.path], paint);
        n++;
      }
      if (open) c.restore();
      final pic = rec.endRecording();
      cache._dropImage();
      try {
        cache.image = pic.toImageSync((size.width * dpr).ceil().clamp(1, 1 << 14), (size.height * dpr).ceil().clamp(1, 1 << 14));
        pic.dispose();
      } catch (_) {
        cache.picture = pic; // no GPU image available: replay the picture
      }
      cache.key = key;
      cache.basePaths = n;
      cache.baseMs = sw.elapsedMicroseconds / 1000;
    }

    final sw = Stopwatch()..start();
    // 1. highlight bands (behind the ink)
    final bands = page.highlightBoxesView();
    _drawBoxes(canvas, bands);

    // 2. base ink
    final img = cache.image;
    if (img != null) {
      canvas.drawImageRect(img, Rect.fromLTWH(0, 0, img.width.toDouble(), img.height.toDouble()), Rect.fromLTWH(0, 0, size.width, size.height), ui.Paint()..filterQuality = FilterQuality.none);
    } else if (cache.picture != null) {
      canvas.save();
      canvas.scale(1 / dpr);
      canvas.drawPicture(cache.picture!);
      canvas.restore();
    }

    // 3. styled paths, placed the same way the ink under them was: by the group the engine put
    // the path in, which is the only answer that holds once a page has reflowed.
    if (styled.isNotEmpty) {
      final places = page.layoutPlacements();
      final ofPath = <int, int>{};
      for (final d in page.layoutDrawList()) {
        ofPath.putIfAbsent(d.path, () => d.placement);
      }
      final paint = ui.Paint()..isAntiAlias = true;
      var cur = -1, open = false;
      for (final s in styled) {
        if ((s.color & 0xff) == 0) continue;
        final pi = ofPath[s.path];
        if (pi == null) continue; // this layout does not draw it
        if (pi != cur) {
          if (open) canvas.restore();
          canvas.save();
          _placeAt(canvas, l, pi < places.length ? places[pi] : QvpPlacement.identity);
          open = true;
          cur = pi;
        }
        paint.color = QvpColor.toColor(s.color);
        canvas.drawPath(paths[s.path], paint);
      }
      if (open) canvas.restore();
    }

    // 4. mask boxes (block / blur) on top
    _drawBoxes(canvas, page.maskBoxesView());
    cache.overlayMs = sw.elapsedMicroseconds / 1000;
    cache.overlayPaths = styled.length;
    cache.bands = bands.length;
  }

  @override
  bool shouldRepaint(_QvpPainter old) => old.page != page || old.view != view || old.dpr != dpr || old.paper != paper || old.paperShadow != paperShadow || old.cache != cache;
}

/// Render statistics of a [QvpPageView] (base / overlay timings of the last frame).
extension QvpPageViewStats on QvpPageViewState {
  ({int basePaths, double baseMs, int overlayPaths, double overlayMs, int bands}) get stats =>
      (basePaths: _cache.basePaths, baseMs: _cache.baseMs, overlayPaths: _cache.overlayPaths, overlayMs: _cache.overlayMs, bands: _cache.bands);
}
