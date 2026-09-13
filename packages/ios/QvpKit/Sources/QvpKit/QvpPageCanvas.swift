// SwiftUI renderer for a `QvpPage` — the same frame as QvpPageView (UIKit), drawn with
// `Canvas`: highlight bands → cached base ink (a CGImage of every non-styled path at the
// current per-line transform, rebuilt only when the styled set / layout / transform changes)
// → styled ink from `styled()` → mask boxes. A `TimelineView(.animation)` runs frames only
// while `page.tick(now)` reports a transition in flight.
//
// SwiftUI has no `setNeedsDisplay()`, so the mutable surface lives on `QvpCanvasController`
// (`@Observable`): set the page and the layout knobs there, call `invalidate()` after engine
// calls the controller cannot see (`highlight`, `style`, `mask`, …), and pass the controller
// to `QvpPageCanvas`. Gestures mirror QvpPageView and every one is a callback: tap →
// gap-aware hit-test → `onWordTap` / `onDecoTap` / `onEmptyTap`; double-tap → `onDoubleTap`
// (nil resets the view); long-press + drag → whole-word selection; pinch / pan on top of the
// engine layout, with a horizontal swipe at the fitted size reported through `onSwipe`.
// When the page is not zoomed and `onSwipe` is nil the drag gesture is detached entirely,
// so an enclosing pager (`TabView`, `ScrollView`) keeps its own swipe.
#if canImport(SwiftUI)
import SwiftUI
import CoreGraphics
#if canImport(UIKit)
import UIKit
#endif

/// Engine state + view state for one `QvpPageCanvas`. Create one per displayed page slot,
/// assign `page`, and call `invalidate()` after engine calls that change what is drawn.
@available(iOS 17.0, macOS 14.0, *)
@MainActor @Observable
public final class QvpCanvasController {
    public var page: QvpPage? {
        didSet { cache.image = nil; cache.key = ""; selectionHandle = 0; relayout(); resetView() }
    }
    // layout knobs (viewport size comes from the canvas)
    public var padTop: CGFloat = 0 { didSet { relayout() } }
    public var padBottom: CGFloat = 0 { didSet { relayout() } }
    public var padSide: CGFloat = 0 { didSet { relayout() } }
    /// Crop the printed page's own horizontal margins (page units, ≥ 0): the
    /// INK spans the padded viewport instead of the full viewBox — for hosts
    /// replacing ink-cropped raster pages. Feed from `page.cropBounds("page")`:
    /// left = box.x0, right = page.width − box.x1.
    public var cropLeft: Float = 0 { didSet { relayout() } }
    public var cropRight: Float = 0 { didSet { relayout() } }
    /// Line spacing only opens up; the engine clamps values below 1 to the printed pitch.
    public var lineSpacing: Float = 1 { didSet { relayout() } }
    public var lineGap: Float = 0 { didSet { relayout() } }
    public var fillHeight = false { didSet { relayout() } }
    /// Paper behind the page content, 0xRRGGBBAA (nil = transparent).
    public var paperColor: UInt32?
    /// 0xRRGGBBAA band colour of the drag selection.
    public var selectionBand: UInt32 = QvpDefaults.SELECTION_BAND
    public var onWordTap: ((QvpWord, QvpHit) -> Void)?
    public var onDecoTap: ((QvpDecoration, QvpHit) -> Void)?
    public var onEmptyTap: (() -> Void)?
    public var onSelectionChanged: (([Int]) -> Void)?
    /// Horizontal swipe while the page is not zoomed in: +1 = finger moved right, -1 = left. The host flips pages.
    public var onSwipe: ((Int) -> Void)?
    /// Double-tap, with the gap-aware hit under it (nil off the page). nil (the default)
    /// resets the view; a host that repurposes the gesture can still call `resetView()`
    /// itself — `isZoomed` says when.
    public var onDoubleTap: ((QvpHit?) -> Void)?
    /// Long-press, with the gap-aware hit under the finger. Fires once, while the finger
    /// is still down, and only while `selectionEnabled` is false — selection owns the
    /// long-press otherwise. On iOS it is UIKit's recognizer, which fails on movement,
    /// so it never takes a swipe from an enclosing pager.
    public var onLongPress: ((QvpHit?) -> Void)?
    public var longPressDuration: Double = 0.35
    public var zoomEnabled = true
    /// Zoom lasts only while the fingers are down: on release the page eases back to its fitted
    /// size — a peek, not a reading zoom — so a pinch never leaves the page holding a pager's swipe.
    public var zoomSpringsBack = false
    public var selectionEnabled = true
    public var hitOptions = QvpHitOptions(maxDistance: QvpDefaults.TAP_DISTANCE)

    public private(set) var viewScale: CGFloat = 1
    public private(set) var viewOx: CGFloat = 0
    public private(set) var viewOy: CGFloat = 0
    /// True while an engine transition is fading — the canvas keeps drawing frames.
    public private(set) var animating = false
    // stats for a HUD
    @ObservationIgnored public private(set) var lastBaseMs = 0.0
    @ObservationIgnored public private(set) var lastOverlayMs = 0.0
    @ObservationIgnored public private(set) var lastHitUs = 0.0
    @ObservationIgnored public private(set) var lastBasePaths = 0
    @ObservationIgnored public private(set) var lastOverlayPaths = 0
    @ObservationIgnored public private(set) var lastBands = 0

    /// Redraw epoch: bumped by `invalidate()`; the canvas body reads it.
    private(set) var revision = 0
    /// Cached base-ink layer. A plain class so draw-time rebuilds don't re-enter observation.
    @ObservationIgnored private let cache = BaseCache()
    @ObservationIgnored private var bounds = CGSize.zero
    @ObservationIgnored private var fitScale: CGFloat = 1
    @ObservationIgnored private var selectionHandle = 0
    @ObservationIgnored private var selAnchor = -1
    @ObservationIgnored var selecting = false
    @ObservationIgnored var pinching = false
    @ObservationIgnored var pinchStart: CGFloat = 1
    @ObservationIgnored var lastDrag = CGSize.zero
    @ObservationIgnored private var springTask: Task<Void, Never>?
    /// True once the reader pinched in beyond the fitted size (panning then moves the page, not the book).
    public var isZoomed: Bool { viewScale > fitScale * QvpViewPolicy.zoomedThreshold }

    final class BaseCache { var image: CGImage?; var key = "" }

    public init() {}

    /// Redraw after engine calls the controller cannot see (`highlight`, `style`, `mask`, `theme`, …).
    public func invalidate() { revision &+= 1 }

    /// Recompute the engine layout for the current size / knobs.
    public func relayout() {
        guard let p = page, p.isOpen, bounds.width > 0, bounds.height > 0 else { return }
        _ = p.layout(QvpLayoutSpec(viewportW: Float(bounds.width), viewportH: Float(bounds.height),
                                   padTop: Float(padTop), padBottom: Float(padBottom),
                                   padLeft: Float(padSide), padRight: Float(padSide),
                                   lineSpacing: lineSpacing, lineGap: lineGap, fillHeight: fillHeight,
                                   cropLeft: cropLeft, cropRight: cropRight))
        cache.key = ""; invalidate()
    }
    /// Fit the content height and centre it.
    public func resetView() {
        springTask?.cancel(); springTask = nil
        let f = fittedView()
        viewScale = f.scale; fitScale = f.scale; viewOx = f.ox; viewOy = f.oy
        invalidate()
    }
    /// The transform `resetView()` applies: content height fitted, centred.
    private func fittedView() -> QvpZoomSpring.ViewTransform {
        let l = page?.currentLayout
        return (CGFloat(l?.fitScale ?? 1), CGFloat(l?.fitX ?? 0), CGFloat(l?.fitY ?? 0))
    }
    /// Clear the selection band and the engine selection.
    public func clearSelection() {
        guard let p = page, p.isOpen else { return }
        p.clearSelection(); if selectionHandle != 0 { p.unhighlight(selectionHandle); selectionHandle = 0 }
        onSelectionChanged?([]); invalidate()
    }
    /// Page units of `line` → view points (engine layout + pan/zoom).
    public func lineTransform(_ line: Int) -> CGAffineTransform {
        let l = page?.currentLayout
        let ls = CGFloat(l?.scale ?? 1), lox = CGFloat(l?.ox ?? 0)
        let dy = (l.flatMap { line < $0.lineDy.count ? $0.lineDy[line] : nil }) ?? 0
        let loy = CGFloat(l?.oy ?? 0) + CGFloat(dy) * ls
        let s = viewScale * ls
        return CGAffineTransform(a: s, b: 0, c: 0, d: s, tx: viewOx + viewScale * lox, ty: viewOy + viewScale * loy)
    }

    // ── input (called by QvpPageCanvas) ──
    func setBounds(_ size: CGSize) {
        guard size != bounds else { return }
        bounds = size; relayout(); resetView()
    }
    func hitAt(_ pt: CGPoint, _ o: QvpHitOptions? = nil) -> QvpHit? {
        guard let p = page, p.isOpen else { return nil }
        let t0 = now()
        let h = p.hitTestView(Float((pt.x - viewOx) / viewScale), Float((pt.y - viewOy) / viewScale), o ?? hitOptions)
        lastHitUs = (now() - t0) * 1e6
        return h
    }
    func tap(_ pt: CGPoint) {
        guard let p = page, p.isOpen else { return }
        let hit = hitAt(pt)
        if let h = hit, h.word >= 0 { onWordTap?(p.words[h.word], h) }
        else if let h = hit, h.deco >= 0 { onDecoTap?(p.decos[h.deco], h) }
        else { onEmptyTap?() }
    }
    func doubleTap(_ pt: CGPoint) { if let cb = onDoubleTap { cb(hitAt(pt)) } else { resetView() } }
    func longPress(_ pt: CGPoint) { onLongPress?(hitAt(pt)) }
    func pinch(_ magnification: CGFloat, at focus: CGPoint) {
        guard zoomEnabled, !selecting else { return }
        if !pinching { springTask?.cancel(); springTask = nil; pinching = true; pinchStart = viewScale }
        let ns = QvpViewPolicy.clampZoom(pinchStart * magnification); let k = ns / viewScale
        viewOx = focus.x - (focus.x - viewOx) * k; viewOy = focus.y - (focus.y - viewOy) * k; viewScale = ns
    }
    func pinchEnded() {
        pinching = false
        if zoomSpringsBack { springBack() }
    }
    /// Ease from the released transform to the fitted one, one step per display frame or so.
    private func springBack() {
        let to = fittedView()
        let spring = QvpZoomSpring(from: (viewScale, viewOx, viewOy), to: to, start: now())
        fitScale = to.scale
        springTask?.cancel()
        springTask = Task { @MainActor [weak self] in
            while let self, !Task.isCancelled {
                let (v, done) = spring.value(at: self.now())
                self.viewScale = v.scale; self.viewOx = v.ox; self.viewOy = v.oy
                self.invalidate()
                if done { self.springTask = nil; return }
                try? await Task.sleep(nanoseconds: 8_000_000)
            }
        }
    }
    func pan(_ translation: CGSize) {
        guard zoomEnabled, !selecting, isZoomed, springTask == nil else { return }
        viewOx += translation.width - lastDrag.width; viewOy += translation.height - lastDrag.height
        lastDrag = translation; invalidate()
    }
    func panEnded(_ t: CGSize, velocity v: CGSize) {
        lastDrag = .zero
        guard !isZoomed, !selecting, onSwipe != nil else { return }
        if let dir = QvpViewPolicy.swipeDirection(translation: t, velocity: v) { onSwipe?(dir) }
    }
    func selectTo(_ pt: CGPoint) {
        guard selectionEnabled, let p = page else { return }
        if !selecting {
            guard let h = hitAt(pt), h.word >= 0 else { return }
            selecting = true; selAnchor = h.word
            p.select(h.word, h.word); paintSelection()
        } else if let h = hitAt(pt, QvpHitOptions()), h.word >= 0 {
            p.select(selAnchor, h.word); paintSelection()
        }
    }
    func selectEnded() { selecting = false }
    private func paintSelection() {
        guard let p = page, p.isOpen else { return }
        let ws = p.selection()
        let t = Target.words(ws)
        if selectionHandle != 0 { p.rehighlight(selectionHandle, t) }
        else { selectionHandle = p.highlight(t, QvpHighlightStyle(mode: .band, band: selectionBand, padX: 0.6, layer: QvpLayer.SELECTION)) }
        onSelectionChanged?(ws); invalidate()
    }

    // ── frame (called from the Canvas renderer) ──
    func draw(in ctx: GraphicsContext, size: CGSize, displayScale: CGFloat) {
        guard let p = page, p.isOpen else { return }
        if p.currentLayout == nil { relayout() }
        guard let l = p.currentLayout else { return }
        let moving = p.tick(now() * 1000)
        let paths = p.buildPaths()
        let styled = p.styled()
        let styledSet = Set(styled.map { $0.path })
        let ink = p.defaultInk
        let W = Int(size.width * displayScale), H = Int(size.height * displayScale)
        var hasher = Hasher(); hasher.combine(styledSet.sorted()); hasher.combine(l.lineDy)
        let key = "\(viewScale)|\(viewOx)|\(viewOy)|\(ink)|\(l.pitch)|\(l.scale)|\(hasher.finalize())|\(W)x\(H)"

        if let paper = paperColor {
            ctx.fill(Path(CGRect(x: viewOx, y: viewOy, width: CGFloat(l.contentW) * viewScale, height: CGFloat(l.contentH) * viewScale)),
                     with: .color(color(paper)))
        }
        let bands = p.highlightBoxesView()
        drawBoxes(ctx, bands)

        if cache.image == nil || key != cache.key || cache.image!.width != W || cache.image!.height != H, W > 0, H > 0 {
            let t0 = now()
            let cs = CGColorSpaceCreateDeviceRGB()
            if let bc = CGContext(data: nil, width: W, height: H, bitsPerComponent: 8, bytesPerRow: 0, space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue) {
                // flip to y-down page space so `makeImage()` comes out upright
                bc.translateBy(x: 0, y: CGFloat(H)); bc.scaleBy(x: displayScale, y: -displayScale)
                bc.setFillColor(QvpColor.cgColor(ink))
                bc.setAllowsAntialiasing(true); bc.setShouldAntialias(true)
                var cur = -1, n = 0
                for pi in 0..<p.nPaths where !styledSet.contains(pi) {
                    let ln = p.pathLine(pi)
                    if ln != cur { if cur >= 0 { bc.restoreGState() }; bc.saveGState(); bc.concatenate(lineTransform(ln)); cur = ln }
                    bc.addPath(paths[pi]); bc.fillPath(using: p.pathEvenOdd(pi) ? .evenOdd : .winding); n += 1
                }
                if cur >= 0 { bc.restoreGState() }
                cache.image = bc.makeImage(); cache.key = key
                lastBaseMs = (now() - t0) * 1000; lastBasePaths = n
            }
        }
        if let img = cache.image {
            ctx.draw(Image(decorative: img, scale: displayScale), in: CGRect(origin: .zero, size: size))
        }
        let t1 = now()
        for (pi, col) in styled where col & 0xff != 0 {
            var c = ctx
            c.concatenate(lineTransform(p.pathLine(pi)))
            c.fill(Path(paths[pi]), with: .color(color(col)), style: FillStyle(eoFill: p.pathEvenOdd(pi)))
        }
        drawBoxes(ctx, p.maskBoxesView())
        lastOverlayMs = (now() - t1) * 1000; lastOverlayPaths = styled.count; lastBands = bands.count
        if moving != animating {
            Task { @MainActor [weak self] in if let self, moving != self.animating { self.animating = moving; self.invalidate() } }
        }
    }
    private func drawBoxes(_ ctx: GraphicsContext, _ boxes: [QvpBox]) {
        if boxes.isEmpty { return }
        var c = ctx
        c.concatenate(CGAffineTransform(a: viewScale, b: 0, c: 0, d: viewScale, tx: viewOx, ty: viewOy))
        var path = Path(), curColor: UInt32 = 0, curId = Int.min
        func flush() { if !path.isEmpty { c.fill(path, with: .color(color(curColor))) }; path = Path() }
        for b in boxes {
            if b.id != curId || b.color != curColor { flush(); curId = b.id; curColor = b.color }
            let r = CGRect(x: CGFloat(b.x0), y: CGFloat(b.y0), width: CGFloat(b.x1 - b.x0), height: CGFloat(b.y1 - b.y0))
            if b.radius > 0 { path.addRoundedRect(in: r, cornerSize: CGSize(width: CGFloat(b.radius), height: CGFloat(b.radius))) }
            else { path.addRect(r) }
        }
        flush()
    }
    private func color(_ rgba: UInt32) -> Color {
        let c = QvpColor.components(rgba)
        return Color(red: c.r, green: c.g, blue: c.b, opacity: c.a)
    }
    private func now() -> Double { Double(DispatchTime.now().uptimeNanoseconds) / 1e9 }
}

/// SwiftUI canvas over a `QvpCanvasController`. Give it a size (it fills what it is given),
/// set `controller.page`, and wire the controller's callbacks.
@available(iOS 17.0, macOS 14.0, *)
public struct QvpPageCanvas: View {
    @Bindable private var controller: QvpCanvasController
    @Environment(\.displayScale) private var displayScale

    public init(controller: QvpCanvasController) { self.controller = controller }

    public var body: some View {
        // read the observable state the renderer depends on, so the canvas redraws on it
        let _ = controller.revision
        let _ = controller.viewScale; let _ = controller.viewOx; let _ = controller.viewOy
        GeometryReader { geo in
            TimelineView(.animation(minimumInterval: nil, paused: !controller.animating)) { _ in
                Canvas(opaque: false, rendersAsynchronously: false) { ctx, size in
                    controller.draw(in: ctx, size: size, displayScale: displayScale)
                }
            }
            // `invalidate()` is this view's setNeedsDisplay(). Engine calls the controller cannot
            // see (mask, reveal, style) leave SwiftUI nothing to diff, and on iOS a bumped revision
            // alone did not repaint a canvas whose timeline was paused. A new identity per revision
            // always repaints; `.identity` keeps the swap from fading inside an animated transaction.
            .id(controller.revision)
            .transition(.identity)
            .onAppear { controller.setBounds(geo.size) }
            .onChange(of: geo.size) { _, s in controller.setBounds(s) }
        }
        .contentShape(Rectangle())
        .gesture(SpatialTapGesture(count: 2).onEnded { v in controller.doubleTap(v.location) }
            .exclusively(before: SpatialTapGesture().onEnded { v in controller.tap(v.location) }))
        .gesture(MagnifyGesture()
            .onChanged { v in controller.pinch(v.magnification, at: v.startLocation) }
            .onEnded { _ in controller.pinchEnded() },
                 including: controller.zoomEnabled ? .all : .subviews)
        // detached at the fitted size when `onSwipe` is nil, so an enclosing pager keeps its swipe
        .highPriorityGesture(DragGesture(minimumDistance: 12)
            .onChanged { v in controller.pan(v.translation) }
            .onEnded { v in controller.panEnded(v.translation, velocity: v.velocity) },
                             including: controller.isZoomed || controller.onSwipe != nil ? .all : .subviews)
        .gesture(LongPressGesture(minimumDuration: 0.35)
            .sequenced(before: DragGesture(minimumDistance: 0))
            .onChanged { v in if case .second(true, let drag) = v, let d = drag { controller.selectTo(d.location) } }
            .onEnded { _ in controller.selectEnded() },
                 including: controller.selectionEnabled ? .all : .subviews)
        .modifier(QvpLongPressAttachment(controller: controller))
    }
}

/// Attaches the long-press recognizer only when a host asked for the callback and
/// selection does not own the gesture.
@available(iOS 17.0, macOS 14.0, *)
private struct QvpLongPressAttachment: ViewModifier {
    let controller: QvpCanvasController

    func body(content: Content) -> some View {
        #if canImport(UIKit)
        if #available(iOS 18.0, *), controller.onLongPress != nil, !controller.selectionEnabled {
            content.gesture(QvpLongPressRecognizer(duration: controller.longPressDuration) { controller.longPress($0) })
        } else {
            content
        }
        #else
        content
        #endif
    }
}

#if canImport(UIKit)
/// UIKit's long-press: it knows where the finger is at recognition (SwiftUI's
/// `LongPressGesture` does not) and fails on movement, so a pager's pan is never taken.
@available(iOS 18.0, *)
private struct QvpLongPressRecognizer: UIGestureRecognizerRepresentable {
    let duration: Double
    let action: (CGPoint) -> Void

    func makeUIGestureRecognizer(context: Context) -> UILongPressGestureRecognizer {
        let recognizer = UILongPressGestureRecognizer()
        recognizer.minimumPressDuration = duration
        return recognizer
    }

    func updateUIGestureRecognizer(_ recognizer: UILongPressGestureRecognizer, context: Context) {
        recognizer.minimumPressDuration = duration
    }

    func handleUIGestureRecognizerAction(_ recognizer: UILongPressGestureRecognizer, context: Context) {
        guard recognizer.state == .began else { return }
        action(context.converter.localLocation)
    }
}
#endif
#endif
