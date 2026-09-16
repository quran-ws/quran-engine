// SwiftUI renderer for a `QvpPage` — the same frame as QvpPageView (UIKit), drawn with
// `Canvas`: highlight bands → cached base ink (a CGImage of every non-styled path at the
// current per-line transform, rebuilt only when the styled set / layout / transform changes)
// → styled ink from `styledPaths()` → mask boxes. A `TimelineView(.animation)` runs frames only
// while `page.tick(now)` reports a transition in flight.
//
// SwiftUI has no `setNeedsDisplay()`, so the mutable surface lives on `QvpCanvasController`
// (`@Observable`): set the page and the layout knobs there, call `invalidate()` after engine
// calls the controller cannot see (`highlight`, `style`, `mask`, …), and pass the controller
// to `QvpPageCanvas`. Gestures mirror QvpPageView and every one is a callback: tap →
// gap-aware hit-test → `onWordTap` / `onDecorationTap` / `onEmptyTap`; double-tap → `onDoubleTap`
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
    /// Line spacing only opens up; the engine clamps values below 1 to the printed lineSpacing.
    public var lineSpacing: Float = 1 { didSet { relayout() } }
    public var fillHeight = false { didSet { relayout() } }
    /// Paper behind the page content, 0xRRGGBBAA (nil = transparent).
    public var paperColor: UInt32?
    /// 0xRRGGBBAA band colour of the drag selection.
    public var selectionBand: UInt32 = QvpDefaults.SELECTION_BAND
    public var onWordTap: ((QvpWord, QvpHit) -> Void)?
    public var onDecorationTap: ((QvpDecoration, QvpHit) -> Void)?
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
    /// What a pinch does to the page. `.stepped` is what a reader gets: the pinch lands on one
    /// of the page's own zoom steps and the page breaks its rows again at that size. `.magnify`
    /// is the old behaviour, scaling the printed page. The engine owns the policy; this only
    /// says which one.
    public var zoomMode: QvpZoomMode = .stepped {
        didSet {
            guard let p = page, p.isOpen, oldValue != zoomMode else { return }
            zoom = p.zoomMode(baseSpec, zoom, zoomMode); relayout(); resetView()
        }
    }
    /// Where the reader's zoom control stands, as the engine last answered.
    public private(set) var zoom = QvpZoom()
    /// The zoom steps this page ships with: what `.stepped` lands on, lowest first.
    public var zoomSteps: [Float] { page.map { $0.isOpen ? $0.zoomSteps(baseSpec) : [] } ?? [] }
    /// Move the control straight to a step. 0 is the printed page.
    public func zoomToStep(_ step: Int) {
        guard let p = page, p.isOpen, bounds.width > 0 else { return }
        apply(p.zoomToStep(layoutSpec, zoom, step, currentView))
    }
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
    /// Creation order of the newest canvas that reported a size (see `setBounds`).
    @ObservationIgnored private var newestCanvas = 0
    @ObservationIgnored private var fitScale: CGFloat = 1
    @ObservationIgnored private var selectionHandle = 0
    @ObservationIgnored private var selAnchor = -1
    @ObservationIgnored var selecting = false
    @ObservationIgnored var pinching = false
    @ObservationIgnored var pinchStart: CGFloat = 1
    @ObservationIgnored var pinchZoom = QvpZoom()
    @ObservationIgnored var pinchView = QvpView()
    @ObservationIgnored var lastDrag = CGSize.zero
    @ObservationIgnored private var springTask: Task<Void, Never>?
    /// True once the reader pinched in beyond the fitted size (panning then moves the page, not the book).
    /// True once the reader has zoomed in, by either road: the view magnified past the fitted
    /// size, or the page reflowed to a size above the printed one. Panning then moves the page
    /// rather than the book.
    public var isZoomed: Bool { viewScale > fitScale * QvpViewPolicy.zoomedThreshold || zoom.zoom > Float(QvpViewPolicy.zoomedThreshold) }

    final class BaseCache { var image: CGImage?; var key = "" }

    public init() {}

    /// Redraw after engine calls the controller cannot see (`highlight`, `style`, `mask`, `theme`, …).
    /// It also asks the engine clock whether frames must run, so a transition the call just began
    /// animates from this same update — not one draw and a main-actor hop later, by which time a
    /// short fade had already spent its first frames.
    public func invalidate() {
        revision &+= 1
        _ = syncAnimating()
    }
    /// `animating` = the engine reports a transition in flight now. True when it changed.
    private func syncAnimating() -> Bool {
        let moving = page.map { $0.isOpen && $0.tick(now() * 1000) } ?? false
        guard moving != animating else { return false }
        animating = moving
        return true
    }

    /// The layout the current size and knobs ask for, before the zoom control has its say.
    var baseSpec: QvpLayoutSpec {
        QvpLayoutSpec(viewportW: Float(bounds.width), viewportH: Float(bounds.height),
                      padTop: Float(padTop), padBottom: Float(padBottom),
                      padLeft: Float(padSide), padRight: Float(padSide),
                      lineSpacing: lineSpacing, fillHeight: fillHeight,
                      cropLeft: cropLeft, cropRight: cropRight)
    }
    /// The spec in force: the knobs above with the reader's zoom control folded in.
    public var layoutSpec: QvpLayoutSpec {
        guard let p = page, p.isOpen else { return baseSpec }
        return p.zoomSpec(baseSpec, zoom)
    }
    /// Recompute the engine layout for the current size / knobs.
    public func relayout() {
        guard let p = page, p.isOpen, bounds.width > 0, bounds.height > 0 else { return }
        _ = p.layout(layoutSpec)
        placements = nil
        cache.key = ""; invalidate()
    }
    /// Fit the content height and centre it.
    public func resetView() {
        springTask?.cancel(); springTask = nil
        let f = fittedView()
        viewScale = f.scale; fitScale = f.scale; viewOx = f.offsetX; viewOy = f.offsetY
        invalidate()
    }
    /// The transform `resetView()` applies: content height fitted, centred.
    private func fittedView() -> QvpZoomSpring.ViewTransform {
        let l = page?.currentLayout
        // A reflowed page is taller than the screen on purpose: fitting its height would undo
        // the size the reader asked for. It opens at its top and the reader scrolls.
        if let l, l.reflowed { return (1, CGFloat(l.fitX), 0) }
        return (CGFloat(l?.fitScale ?? 1), CGFloat(l?.fitX ?? 0), CGFloat(l?.fitY ?? 0))
    }
    /// Clear the selection band and the engine selection.
    public func clearSelection() {
        guard let p = page, p.isOpen else { return }
        p.clearSelection(); if selectionHandle != 0 { p.removeHighlight(selectionHandle); selectionHandle = 0 }
        onSelectionChanged?([]); invalidate()
    }
    /// Page units of `line` → view points (engine layout + pan/zoom).
    public func lineTransform(_ line: Int) -> CGAffineTransform {
        let l = page?.currentLayout
        let dy = (l.flatMap { line < $0.lineDy.count ? $0.lineDy[line] : nil }) ?? 0
        return transform(QvpPlacement(dx: 0, dy: dy, kx: 1, ky: 1))
    }
    /// Page units → view points under one placement (engine layout + pan/zoom).
    func transform(_ q: QvpPlacement, offsetY: CGFloat? = nil) -> CGAffineTransform {
        let l = page?.currentLayout
        let ls = CGFloat(l?.scale ?? 1)
        let s = viewScale * ls
        return CGAffineTransform(a: s * CGFloat(q.kx), b: 0, c: 0, d: s * CGFloat(q.ky),
                                 tx: viewOx + viewScale * (CGFloat(l?.offsetX ?? 0) + CGFloat(q.dx) * ls),
                                 ty: (offsetY ?? viewOy) + viewScale * (CGFloat(l?.offsetY ?? 0) + CGFloat(q.dy) * ls))
    }
    /// Where the cached band of ink starts, in view points. It moves in half-screen steps, so a
    /// drag inside the band is a blit and nothing is drawn again.
    var bandTop: CGFloat {
        let step = max(bounds.height / 2, 1)
        return ((-viewOy - step) / step).rounded(.down) * step
    }
    var bandHeight: CGFloat { max(bounds.height * 2, 1) }
    /// What the layout in hand draws and where: the engine's own answer, read once per layout.
    /// A printed page and a reflowed one come back in the same shape, so nothing here has to
    /// know which it is looking at.
    struct DrawList { let draws: [QvpDraw], places: [QvpPlacement], band: ClosedRange<Float> }
    var placements: DrawList?
    /// The band of the laid-out page worth drawing: the screen, with a screen of slack either
    /// side so a fast drag has somewhere to go before the next frame asks again.
    /// The same band in layout points: what the engine is asked to draw.
    private var visibleBand: (top: Float, bottom: Float) {
        let s = viewScale == 0 ? 1 : viewScale
        return (Float(bandTop / s), Float((bandTop + bandHeight) / s))
    }
    func drawListNow() -> DrawList? {
        let b = visibleBand
        if let q = placements, q.band == b.top...b.bottom { return q }
        guard let p = page, p.isOpen, p.currentLayout != nil else { return nil }
        let q = DrawList(draws: p.layoutDrawList(band: b), places: p.layoutPlacements(), band: b.top...b.bottom)
        placements = q; return q
    }
    /// The transform one entry of the draw list is drawn under.
    func transform(_ q: DrawList, _ placement: Int, offsetY: CGFloat? = nil) -> CGAffineTransform {
        transform(placement < q.places.count ? q.places[placement] : .identity, offsetY: offsetY)
    }

    // ── input (called by QvpPageCanvas) ──
    /// Takes a size only from the newest canvas of this controller. A host can show an outgoing
    /// canvas and an incoming canvas for a moment, for example when a rotation moves the page from
    /// a static layout to a scrolling layout. The outgoing canvas can report its transitional size
    /// after the incoming canvas reported the real size, and the page then kept the wrong layout.
    func setBounds(_ size: CGSize, fromCanvas order: Int) {
        guard order >= newestCanvas else { return }
        newestCanvas = order
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
        else if let h = hit, h.decoration >= 0 { onDecorationTap?(p.decorations[h.decoration], h) }
        else { onEmptyTap?() }
    }
    func doubleTap(_ pt: CGPoint) { if let cb = onDoubleTap { cb(hitAt(pt)) } else { resetView() } }
    func longPress(_ pt: CGPoint) { onLongPress?(hitAt(pt)) }
    /// The reader's pan and zoom as the engine has it.
    var currentView: QvpView { QvpView(scale: Float(viewScale), offsetX: Float(viewOx), offsetY: Float(viewOy)) }
    /// Take what a gesture produced: the page is already laid out at the new size, and the view
    /// already holds the word the fingers were on.
    private func apply(_ c: QvpZoomChange) {
        zoom = c.zoom
        viewScale = CGFloat(c.view.scale); viewOx = CGFloat(c.view.offsetX); viewOy = CGFloat(c.view.offsetY)
        if c.relaid { placements = nil; cache.key = "" }
        invalidate()
    }
    func pinch(_ magnification: CGFloat, at focus: CGPoint) {
        guard zoomEnabled, !selecting else { return }
        if !pinching {
            springTask?.cancel(); springTask = nil; pinching = true
            pinchStart = viewScale; pinchZoom = zoom; pinchView = currentView
        }
        guard let p = page, p.isOpen, zoomMode != .magnify else {
            // the printed page under a magnifying glass: the rows never move
            let ns = QvpViewPolicy.clampZoom(pinchStart * magnification); let k = ns / viewScale
            viewOx = focus.x - (focus.x - viewOx) * k; viewOy = focus.y - (focus.y - viewOy) * k; viewScale = ns
            return
        }
        // `pinchZoom` stays where the gesture began: the engine measures the fingers against
        // that, so a slow pinch walks the steps one at a time instead of running away.
        apply(p.zoomPinch(layoutSpec, pinchZoom, pinchView, factor: Float(magnification), focalX: Float(focus.x), focalY: Float(focus.y)))
    }
    func pinchEnded() {
        pinching = false
        // A reflowed page is a size the reader chose, not a peek: it stays until they change it.
        if zoomSpringsBack && zoomMode == .magnify { springBack() }
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
                self.viewScale = v.scale; self.viewOx = v.offsetX; self.viewOy = v.offsetY
                self.invalidate()
                if done { self.springTask = nil; return }
                try? await Task.sleep(nanoseconds: 8_000_000)
            }
        }
    }
    /// A reflowed page is the screen's own width: nothing to pan sideways, so it scrolls up and
    /// down only, and a sideways drag turns the page.
    private var isReflowed: Bool { page?.currentLayout?.reflowed ?? false }
    func pan(_ translation: CGSize) {
        guard zoomEnabled, !selecting, isZoomed || isReflowed, springTask == nil else { return }
        if !isReflowed { viewOx += translation.width - lastDrag.width }
        viewOy += translation.height - lastDrag.height
        lastDrag = translation
        if isReflowed, let l = page?.currentLayout, let p = page {
            let v = p.viewClamp(currentView, contentW: l.contentW, contentH: l.contentH, viewportW: Float(bounds.width), viewportH: Float(bounds.height))
            viewOx = CGFloat(v.offsetX); viewOy = CGFloat(v.offsetY)
        }
        invalidate()
    }
    /// A sideways drag turns the page whether or not the reader has zoomed in: a reflowed page
    /// has nothing to pan sideways, so the gesture is free to mean this.
    func panEnded(_ t: CGSize, velocity v: CGSize) {
        lastDrag = .zero
        guard !isZoomed || isReflowed, !selecting, onSwipe != nil else { return }
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
        if selectionHandle != 0 { p.moveHighlight(selectionHandle, t) }
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
        let styled = p.styledPaths()
        let styledSet = Set(styled.map { $0.path })
        let ink = p.defaultInk
        let W = Int(size.width * displayScale), H = Int(size.height * displayScale)
        var hasher = Hasher(); hasher.combine(styledSet.sorted()); hasher.combine(l.lineDy)
        // The cached ink is a band of the page, not the screen, so a drag inside it is one blit
        // and nothing is drawn again.
        let top = bandTop, bandH = bandHeight
        let bandPx = Int((bandH * displayScale).rounded(.up))
        let key = "\(viewScale)|\(viewOx)|\(top)|\(ink)|\(l.lineSpacing)|\(l.scale)|\(hasher.finalize())|\(W)x\(bandPx)|\(zoom.zoom)|\(l.rows)"

        if let paper = paperColor {
            ctx.fill(Path(CGRect(x: viewOx, y: viewOy, width: CGFloat(l.contentW) * viewScale, height: CGFloat(l.contentH) * viewScale)),
                     with: .color(color(paper)))
        }
        let bands = p.highlightBoxesView()
        drawBoxes(ctx, bands)

        if cache.image == nil || key != cache.key || cache.image!.width != W || cache.image!.height != bandPx, W > 0, bandPx > 0 {
            let t0 = now()
            let cs = CGColorSpaceCreateDeviceRGB()
            if let bc = CGContext(data: nil, width: W, height: bandPx, bitsPerComponent: 8, bytesPerRow: 0, space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue) {
                // flip to y-down page space so `makeImage()` comes out upright
                bc.translateBy(x: 0, y: CGFloat(bandPx)); bc.scaleBy(x: displayScale, y: -displayScale)
                bc.setFillColor(QvpColor.cgColor(ink))
                bc.setAllowsAntialiasing(true); bc.setShouldAntialias(true)
                var cur = -1, n = 0
                if let q = drawListNow() {
                    for d in q.draws where !styledSet.contains(d.path) {
                        // one state per placement, not per path: the list runs in that order
                        // the band is drawn under its own vertical offset: its top row is `top`
                        if d.placement != cur { if cur >= 0 { bc.restoreGState() }; bc.saveGState(); bc.concatenate(transform(q, d.placement, offsetY: -top)); cur = d.placement }
                        bc.addPath(paths[d.path]); bc.fillPath(using: p.pathEvenOdd(d.path) ? .evenOdd : .winding); n += 1
                    }
                }
                if cur >= 0 { bc.restoreGState() }
                cache.image = bc.makeImage(); cache.key = key
                lastBaseMs = (now() - t0) * 1000; lastBasePaths = n
            }
        }
        if let img = cache.image {
            ctx.draw(Image(decorative: img, scale: displayScale), in: CGRect(x: 0, y: viewOy + top, width: size.width, height: bandH))
        }
        let t1 = now()
        if let q = drawListNow() {
            let colors = Dictionary(styled, uniquingKeysWith: { a, _ in a })
            for d in q.draws {
                guard let col = colors[d.path], col & 0xff != 0 else { continue }
                var c = ctx
                c.concatenate(transform(q, d.placement))
                c.fill(Path(paths[d.path]), with: .color(color(col)), style: FillStyle(eoFill: p.pathEvenOdd(d.path)))
            }
        }
        drawBoxes(ctx, p.maskBoxesView())
        lastOverlayMs = (now() - t1) * 1000; lastOverlayPaths = styled.count; lastBands = bands.count
        if moving != animating {
            // Ask the engine again at the hop rather than trusting this frame: a transition begun
            // in between must not be paused by a stale "finished".
            Task { @MainActor [weak self] in if let self, self.syncAnimating() { self.revision &+= 1 } }
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
    /// The order in which SwiftUI created this canvas. The controller ignores sizes from a canvas
    /// that a newer canvas replaced.
    @State private var order = QvpCanvasOrder.next()

    public init(controller: QvpCanvasController) { self.controller = controller }

    public var body: some View {
        // read the observable state the renderer depends on, so the canvas redraws on it
        let _ = controller.revision
        let _ = controller.viewScale; let _ = controller.viewOx; let _ = controller.viewOy
        GeometryReader { _ in
            TimelineView(.animation(minimumInterval: nil, paused: !controller.animating)) { timeline in
                Canvas(opaque: false, rendersAsynchronously: false) { ctx, size in
                    // The renderer reads the frame's date so every timeline tick is a different
                    // canvas to SwiftUI. A renderer that captured only the controller looked
                    // unchanged tick to tick, and on device a running transition drew no frames.
                    _ = timeline.date
                    controller.draw(in: ctx, size: size, displayScale: displayScale)
                }
            }
            // `invalidate()` is this view's setNeedsDisplay(). Engine calls the controller cannot
            // see (mask, reveal, style) leave SwiftUI nothing to diff, and on iOS a bumped revision
            // alone did not repaint a canvas whose timeline was paused. A new identity per revision
            // always repaints; `.identity` keeps the swap from fading inside an animated transaction.
            .id(controller.revision)
            .transition(.identity)
        }
        // The size comes from the GeometryReader. Its identity does not change. The view inside
        // gets a new identity at each redraw, so a size change in the same update did not reach
        // its onAppear or its onChange, and the page kept the layout of its old size.
        .onGeometryChange(for: CGSize.self) { $0.size } action: { controller.setBounds($0, fromCanvas: order) }
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

/// Gives each canvas a number that increases in the order SwiftUI creates them.
@MainActor
enum QvpCanvasOrder {
    private static var last = 0
    static func next() -> Int {
        last += 1
        return last
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
