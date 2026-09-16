#if canImport(UIKit)
import UIKit
import QuartzCore

/// Host-canvas renderer for a `QvpPage` (CoreGraphics). Draw order per frame:
/// highlight bands (one path per highlight id, nonzero, behind the ink) → cached base ink
/// (a bitmap of every non-styled path at the current transform, rebuilt only when the styled
/// set / layout / transform changes) → styled ink from `styledPaths()` → mask boxes.
/// Each frame calls `page.tick(now)`; a CADisplayLink keeps running while the engine says so.
/// Gestures: tap → gap-aware hit-test → `onWordTap` / `onDecorationTap` / `onEmptyTap`; long-press +
/// drag → whole-word selection (engine `select`, band in the selection layer); pinch / pan on
/// top of the engine layout; double-tap resets the view (or runs `onDoubleTap` when set).
public final class QvpPageView: UIView, UIGestureRecognizerDelegate, UIScrollViewDelegate {
    public var page: QvpPage? {
        didSet {
            base = nil; baseKey = ""; selectionHandle = 0; drawList = nil
            // A step means the same size to the reader on any page: the engine chose every
            // page's steps so the ink barely changes across a turn. So the step carries over,
            // and the new page says what it means. Carrying the size instead and rounding it to
            // the new page's steps loses a step on every turn, because a step that sits a
            // little higher on the next page rounds down.
            if let p = page, p.isOpen, bounds.width > 0 { zoom = p.zoomCarried(baseSpec, zoom) }
            relayout(); resetView(); invalidateContent()
        }
    }
    // layout knobs (viewport size comes from the view)
    public var padTop: CGFloat = 0 { didSet { relayout() } }
    public var padBottom: CGFloat = 0 { didSet { relayout() } }
    public var padSide: CGFloat = 0 { didSet { relayout() } }
    public var lineSpacing: Float = 1 { didSet { relayout() } }
    public var fillHeight = false { didSet { relayout() } }
    /// Paper behind the page content (nil = transparent).
    public var paperColor: UIColor? { didSet { invalidateContent() } }
    /// 0xRRGGBBAA band colour of the drag selection.
    public var selectionBand: UInt32 = QvpDefaults.SELECTION_BAND
    public var onWordTap: ((QvpWord, QvpHit) -> Void)?
    public var onDecorationTap: ((QvpDecoration, QvpHit) -> Void)?
    public var onEmptyTap: (() -> Void)?
    public var onSelectionChanged: (([Int]) -> Void)?
    /// Horizontal swipe while the page is not zoomed in: +1 = finger moved right, -1 = left. The host flips pages.
    public var onSwipe: ((Int) -> Void)?
    /// Double-tap. nil (the default) resets the view; a host that repurposes the gesture
    /// (e.g. marking reading progress) can still call `resetView()` itself — `isZoomed` says when.
    public var onDoubleTap: ((QvpHit?) -> Void)?
    /// Long-press, with the gap-aware hit under the finger. Fires once, on recognition,
    /// only while `selectionEnabled` is false — selection owns the long-press otherwise.
    public var onLongPress: ((QvpHit?) -> Void)?
    public var longPressDuration: Double = 0.35 { didSet { longPressRecognizer?.minimumPressDuration = longPressDuration } }
    private weak var longPressRecognizer: UILongPressGestureRecognizer?
    public var zoomEnabled = true
    /// Zoom lasts only while the fingers are down: on release the page eases back to its fitted
    /// size — a peek, not a reading zoom — so a pinch never leaves the page holding the pan.
    public var zoomSpringsBack = false
    /// What a pinch does to the page. `.stepped` is what a reader gets: the pinch lands on one
    /// of the page's own zoom steps and the page breaks its rows again at that size. `.magnify`
    /// scales the printed page instead. The engine owns the behaviour; this picks which.
    public var zoomMode: QvpZoomMode = .stepped {
        didSet {
            guard let p = page, p.isOpen, oldValue != zoomMode else { return }
            zoom = p.zoomMode(baseSpec, zoom, zoomMode); relayout(); resetView()
        }
    }
    /// Where the reader's zoom control stands, as the engine last answered.
    public private(set) var zoom = QvpZoom()
    /// The zoom steps this page ships with: what `.stepped` lands on, lowest first.
    public var zoomSteps: [Float] { page.map { $0.isOpen && bounds.width > 0 ? $0.zoomSteps(baseSpec) : [] } ?? [] }
    /// Scroll the page by `dy` points, held against the viewport: what a reflowed page, which
    /// is taller than the screen, is read with.
    public func scroll(by dy: CGFloat) {
        guard let p = page, p.isOpen, let l = p.currentLayout else { return }
        viewOy += dy
        let v = p.viewClamp(currentView, contentW: l.contentW, contentH: l.contentH, viewportW: Float(bounds.width), viewportH: Float(bounds.height))
        viewOx = CGFloat(v.offsetX); viewOy = CGFloat(v.offsetY)
        refresh()
    }

    /// Move the control straight to a step. 0 is the printed page.
    ///
    /// A step asked for before the view has a size is remembered and taken as soon as it has
    /// one, so a host can set the reader's size while building the screen.
    public func zoomToStep(_ step: Int) {
        guard let p = page, p.isOpen, bounds.width > 0, bounds.height > 0 else { pendingStep = step; return }
        pendingStep = nil
        apply(p.zoomToStep(layoutSpec, zoom, step, currentView))
    }
    private var pendingStep: Int?
    public var selectionEnabled = true
    public var hitOptions = QvpHitOptions(maxDistance: QvpDefaults.TAP_DISTANCE)

    public private(set) var viewScale: CGFloat = 1
    public private(set) var viewOx: CGFloat = 0
    public private(set) var viewOy: CGFloat = 0
    // stats for a HUD
    public private(set) var lastBaseMs = 0.0, lastOverlayMs = 0.0, lastHitUs = 0.0
    public private(set) var lastBasePaths = 0, lastOverlayPaths = 0, lastBands = 0
    public private(set) var animating = false

    private var base: CGImage?
    private var baseKey = ""
    /// The band of page content, as one image in a layer of its own. Scrolling moves the layer
    /// and draws nothing: the reader's finger only changes where the band sits on screen.
    private let contentLayer = CALayer()
    /// A reflowed page is taller than the screen, so the platform's own scroll view carries it:
    /// the reader gets the deceleration, the rubber band at the ends and the indicator they get
    /// everywhere else on the phone, and the band layer is moved by the compositor rather than
    /// by us. It is `contentHost` that the band sits in, and the scroll view that moves it.
    private let scroller = UIScrollView()
    private let contentHost = UIView()
    private var selectionHandle = 0
    private var selAnchor = -1
    private var selecting = false
    private var link: CADisplayLink?
    private var spring: QvpZoomSpring?
    private var springLink: CADisplayLink?
    private var pinchStart: CGFloat = 1
    private var pinchZoom = QvpZoom()
    private var pinchView = QvpView()
    private var fitScale: CGFloat = 1
    /// True once the reader has zoomed in, by either road: the view magnified past the fitted
    /// size, or the page reflowed to a size above the printed one. The engine decides, so every
    /// platform draws the line in the same place.
    public var isZoomed: Bool { zoom.isZoomed(currentView, fitScale: Float(fitScale)) }
    /// What a sideways drag means here: pan a magnified page, or turn the page.
    private var sideways: QvpSideways {
        page?.sidewaysDrag(zoom, currentView, fitScale: Float(fitScale)) ?? .turnPage
    }

    public override init(frame: CGRect) { super.init(frame: frame); setup() }
    public required init?(coder: NSCoder) { super.init(coder: coder); setup() }
    private func setup() {
        // the band is taller than the view and hangs past both edges: it is the view that says
        // how much of it the reader sees
        layer.masksToBounds = true
        contentLayer.actions = ["contents": NSNull(), "position": NSNull(), "bounds": NSNull()]
        contentHost.layer.addSublayer(contentLayer)
        scroller.addSubview(contentHost)
        scroller.delegate = self
        scroller.backgroundColor = .clear
        scroller.showsHorizontalScrollIndicator = false
        scroller.isDirectionalLockEnabled = true
        scroller.contentInsetAdjustmentBehavior = .never
        scroller.alwaysBounceVertical = false
        addSubview(scroller)
        isOpaque = false
        backgroundColor = .clear
        contentMode = .redraw
        isMultipleTouchEnabled = true
        let dbl = UITapGestureRecognizer(target: self, action: #selector(handleDoubleTap)); dbl.numberOfTapsRequired = 2
        let tap = UITapGestureRecognizer(target: self, action: #selector(onTap)); tap.require(toFail: dbl)
        let pinch = UIPinchGestureRecognizer(target: self, action: #selector(onPinch))
        let pan = UIPanGestureRecognizer(target: self, action: #selector(onPan)); pan.maximumNumberOfTouches = 2
        let long = UILongPressGestureRecognizer(target: self, action: #selector(handleLongPress)); long.minimumPressDuration = longPressDuration; longPressRecognizer = long
        pan.require(toFail: long)
        for g in [dbl, tap, pinch, pan, long] { g.delegate = self; addGestureRecognizer(g) }
        // A page turn answers the moment UIKit recognises the swipe, rather than waiting for
        // the finger to lift and reading the drag afterwards.
        for dir in [UISwipeGestureRecognizer.Direction.left, .right] {
            let sw = UISwipeGestureRecognizer(target: self, action: #selector(onSwipeGesture))
            sw.direction = dir
            sw.delegate = self
            addGestureRecognizer(sw)
            // The scroll view waits for the swipe to fail before it scrolls. A swipe fails as
            // soon as the finger is going the other way, so an up-and-down drag loses almost
            // nothing of its start, and a sideways flick is never eaten by the scroll.
            scroller.panGestureRecognizer.require(toFail: sw)
        }
    }
    deinit { link?.invalidate(); springLink?.invalidate() }

    public func gestureRecognizer(_ a: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith b: UIGestureRecognizer) -> Bool {
        // The page-turn swipe runs beside the scroll view's drag rather than waiting for it to
        // fail, which would put a hesitation at the start of every scroll. It only answers a
        // quick sideways flick, and the scroll view is locked to the other axis, so the two
        // cannot both mean something.
        if a is UISwipeGestureRecognizer || b is UISwipeGestureRecognizer { return true }
        return (a is UIPinchGestureRecognizer && b is UIPanGestureRecognizer) || (a is UIPanGestureRecognizer && b is UIPinchGestureRecognizer)
    }

    public override func gestureRecognizerShouldBegin(_ g: UIGestureRecognizer) -> Bool {
        // A reflowed page is scrolled by the scroll view, so our own drag stands aside rather
        // than moving the page twice.
        if g is UIPanGestureRecognizer, g !== scroller.panGestureRecognizer, isReflowed { return false }
        return true
    }

    // ── gestures ──
    private func hitAt(_ pt: CGPoint, _ o: QvpHitOptions? = nil) -> QvpHit? {
        guard let p = page, p.isOpen else { return nil }
        let t0 = CACurrentMediaTime()
        let h = p.hitTestView(Float((pt.x - viewOx) / viewScale), Float((pt.y - viewOy) / viewScale), o ?? hitOptions)
        lastHitUs = (CACurrentMediaTime() - t0) * 1e6
        return h
    }
    @objc private func onTap(_ g: UITapGestureRecognizer) {
        guard let p = page else { return }
        let hit = hitAt(g.location(in: self))
        if let h = hit, h.word >= 0 { onWordTap?(p.words[h.word], h) }
        else if let h = hit, h.decoration >= 0 { onDecorationTap?(p.decorations[h.decoration], h) }
        else { onEmptyTap?() }
    }
    @objc private func handleDoubleTap(_ g: UITapGestureRecognizer) { if let cb = onDoubleTap { cb(hitAt(g.location(in: self))) } else { resetView() } }
    @objc private func onPinch(_ g: UIPinchGestureRecognizer) {
        guard zoomEnabled, !selecting else { return }
        if g.state == .began { stopSpring(); pinchStart = viewScale; pinchZoom = zoom; pinchView = currentView }
        // A reflowed page is a size the reader chose, not a peek: only a magnifying pinch springs back.
        if zoomSpringsBack, zoomMode == .magnify, [.ended, .cancelled, .failed].contains(g.state) { springBack(); return }
        let f = g.location(in: self)
        guard let p = page, p.isOpen, zoomMode != .magnify else {
            let ns = QvpViewPolicy.clampZoom(pinchStart * g.scale); let k = ns / viewScale
            viewOx = f.x - (f.x - viewOx) * k; viewOy = f.y - (f.y - viewOy) * k; viewScale = ns
            invalidateContent()
            return
        }
        // `pinchZoom` stays where the gesture began: the engine measures the fingers against
        // that, so a slow pinch walks the steps one at a time instead of running away.
        apply(p.zoomPinch(layoutSpec, pinchZoom, pinchView, factor: Float(g.scale), focalX: Float(f.x), focalY: Float(f.y)))
    }
    @objc private func onPan(_ g: UIPanGestureRecognizer) {
        guard zoomEnabled, !selecting, spring == nil else { return }
        // A reflowed page is the screen's own width and the scroll view owns the one axis it
        // has. A printed page nobody has zoomed is not dragged either: a sideways flick on it
        // turns the page, and that is the swipe recogniser's to answer, not this one's. Both
        // answering it turned two pages at a time.
        guard sideways == .pan else { return }
        let d = g.translation(in: self)
        viewOx += d.x
        viewOy += d.y
        g.setTranslation(.zero, in: self)
        refresh()
    }
    @objc private func handleLongPress(_ g: UILongPressGestureRecognizer) {
        if !selectionEnabled {
            if g.state == .began { onLongPress?(hitAt(g.location(in: self))) }
            return
        }
        guard let p = page else { return }
        let pt = g.location(in: self)
        switch g.state {
        case .began:
            guard let h = hitAt(pt), h.word >= 0 else { return }
            selecting = true; selAnchor = h.word
            p.select(h.word, h.word); paintSelection()
        case .changed:
            guard selecting, let h = hitAt(pt, QvpHitOptions()), h.word >= 0 else { return }
            p.select(selAnchor, h.word); paintSelection()
        default:
            selecting = false
        }
    }
    private func paintSelection() {
        guard let p = page else { return }
        let ws = p.selection()
        let t = Target.words(ws)
        if selectionHandle != 0 { p.moveHighlight(selectionHandle, t) }
        else { selectionHandle = p.highlight(t, QvpHighlightStyle(mode: .band, band: selectionBand, padX: 0.6, layer: QvpLayer.SELECTION)) }
        onSelectionChanged?(ws); invalidateContent()
    }
    /// Clear the selection band and the engine selection.
    public func clearSelection() {
        guard let p = page else { return }
        p.clearSelection(); if selectionHandle != 0 { p.removeHighlight(selectionHandle); selectionHandle = 0 }
        onSelectionChanged?([]); invalidateContent()
    }

    // ── layout ──
    public override func layoutSubviews() {
        super.layoutSubviews()
        guard bounds.size != lastSize else { return }
        lastSize = bounds.size; relayout(); resetView()
        if let s = pendingStep { zoomToStep(s) }
        invalidateContent()
    }
    private var lastSize = CGSize.zero
    /// The layout the current size and knobs ask for, before the zoom control has its say.
    var baseSpec: QvpLayoutSpec {
        QvpLayoutSpec(viewportW: Float(bounds.width), viewportH: Float(bounds.height), padTop: Float(padTop), padBottom: Float(padBottom), padLeft: Float(padSide), padRight: Float(padSide), lineSpacing: lineSpacing, fillHeight: fillHeight)
    }
    /// The spec in force: the knobs above with the reader's zoom control folded in.
    public var layoutSpec: QvpLayoutSpec {
        guard let p = page, p.isOpen else { return baseSpec }
        return p.zoomSpec(baseSpec, zoom)
    }
    /// Recompute the engine layout for the current size / knobs.
    public func relayout() {
        guard let p = page, p.isOpen, bounds.width > 0, bounds.height > 0 else { return }
        let sp = layoutSpec
        p.layout(sp)
        drawList = nil
        baseKey = ""; invalidateContent()
    }
    /// Fit the content height and centre it.
    public func resetView() {
        stopSpring()
        let f = fittedView()
        viewScale = f.scale; fitScale = f.scale; viewOx = f.offsetX; viewOy = f.offsetY
        invalidateContent()
    }
    /// The transform `resetView()` applies: content height fitted, centred.
    private func fittedView() -> QvpZoomSpring.ViewTransform {
        let l = page?.currentLayout
        // A reflowed page is taller than the screen on purpose: fitting its height would undo
        // the size the reader asked for. It opens at its top and the reader scrolls.
        if let l, l.reflowed { return (1, CGFloat(l.fitX), 0) }
        return (CGFloat(l?.fitScale ?? 1), CGFloat(l?.fitX ?? 0), CGFloat(l?.fitY ?? 0))
    }
    /// Ease from the released transform to the fitted one on a display link of its own.
    private func springBack() {
        let to = fittedView()
        fitScale = to.scale
        spring = QvpZoomSpring(from: (viewScale, viewOx, viewOy), to: to, start: CACurrentMediaTime())
        if springLink == nil { let l = CADisplayLink(target: self, selector: #selector(onSpringFrame)); l.add(to: .main, forMode: .common); springLink = l }
    }
    @objc private func onSpringFrame() {
        guard let s = spring else { stopSpring(); return }
        let (v, done) = s.value(at: CACurrentMediaTime())
        viewScale = v.scale; viewOx = v.offsetX; viewOy = v.offsetY
        invalidateContent()
        if done { stopSpring() }
    }
    private func stopSpring() { spring = nil; springLink?.invalidate(); springLink = nil }
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
    /// What the layout in hand draws and where: the engine's own answer, read once per layout.
    /// A printed page and a reflowed one come back in the same shape.
    struct DrawList { let draws: [QvpDraw], places: [QvpPlacement], band: ClosedRange<Float> }
    private var drawList: DrawList?
    /// The band of the laid-out page worth drawing: the screen, with a screen of slack either
    /// side so a fast drag has somewhere to go before the next frame asks again.
    /// How much ink to keep ready at once. A page that fits inside this is drawn once — the
    /// parts past both ends of the screen included — and then scrolling never draws again: the
    /// scroll view moves the finished image and the compositor does the rest. A page too tall
    /// for the budget keeps a band of itself instead, two screens tall, which is redrawn when
    /// the reader scrolls out of it.
    ///
    /// The whole of a page of this muṣḥaf is about 17 MB at the first zoom step and 44 MB at
    /// the last, on a phone at three pixels to the point.
    public var inkBudget = 64 << 20
    /// How much ink already drawn to keep beside the page in hand, so a page the reader turns
    /// to — or back to — is ready without being drawn again. Two pages' worth by default: the
    /// one before and the one after, which is what a reader moves between.
    public var inkCacheBudget = 2 * (64 << 20)
    /// Ink drawn earlier, most recent first. The key is the page and the size it was drawn at,
    /// so a page kept at one zoom step is not mistaken for the same page at another.
    private var inkCache: [(key: String, image: CGImage, bytes: Int)] = []

    private func cachedInk(_ key: String) -> CGImage? {
        guard let i = inkCache.firstIndex(where: { $0.key == key }) else { return nil }
        let hit = inkCache.remove(at: i)
        inkCache.insert(hit, at: 0)
        return hit.image
    }

    private func keepInk(_ key: String, _ image: CGImage) {
        inkCache.removeAll { $0.key == key }
        inkCache.insert((key, image, image.height * image.bytesPerRow), at: 0)
        var total = 0
        inkCache = inkCache.filter { entry in
            total += entry.bytes
            return total <= inkCacheBudget
        }
    }

    /// Draw a page the reader has not reached yet, at the size they are reading, and keep it. A
    /// turn onto that page then shows ink that is already drawn.
    ///
    /// This is work, so a host calls it when nothing else is happening — after a turn has
    /// settled, not while one is running.
    public func prepare(_ other: QvpPage) {
        guard other.isOpen, bounds.width > 0, bounds.height > 0, other !== page else { return }
        var z = zoom
        if zoomMode == .stepped, z.step > 0 {
            z = QvpZoom(mode: .stepped, step: min(z.step, other.zoomSteps(baseSpec).count), zoom: 1)
        }
        // a page the reader has not reached opens at its top, which is where it will be drawn
        _ = renderInk(other, z, offsetY: 0)
    }
    /// Where the cached ink starts and how tall it is, for the page and place in hand.
    private var currentBand: (top: CGFloat, height: CGFloat) {
        guard let l = page?.currentLayout else { return (0, max(bounds.height, 1)) }
        return band(for: l, offsetY: viewOy, scale: window?.screen.scale ?? contentScaleFactor)
    }
    private var bandTop: CGFloat { currentBand.top }
    private var bandHeight: CGFloat { currentBand.height }
    /// True when the page is broken onto rows of its own and so is taller than the screen.
    private var isReflowed: Bool { page?.currentLayout?.reflowed ?? false }
    /// Where the band sits in the layer that holds it: down the scroll view's content while it
    /// owns the axis, and on the screen itself otherwise.
    private var bandOriginY: CGFloat { isReflowed ? bandTop : viewOy + bandTop }
    /// True while the ink in hand already covers what the reader can see, so a scroll has
    /// nothing to do but let the scroll view move it.
    private var inkCoversScreen: Bool {
        let top = -viewOy, bottom = top + bounds.height
        return bandTop <= top + 0.5 && bandTop + bandHeight >= bottom - 0.5
    }
    /// The same band in layout points: what the engine is asked to draw.
    private var visibleBand: (top: Float, bottom: Float) {
        let s = viewScale == 0 ? 1 : viewScale
        return (Float(bandTop / s), Float((bandTop + bandHeight) / s))
    }
    private func drawListNow() -> DrawList? {
        let b = visibleBand
        if let q = drawList, q.band == b.top...b.bottom { return q }
        guard let p = page, p.isOpen, p.currentLayout != nil else { return nil }
        let q = DrawList(draws: p.layoutDrawList(band: b), places: p.layoutPlacements(), band: b.top...b.bottom)
        drawList = q; return q
    }
    private func transform(_ q: DrawList, _ placement: Int, offsetY: CGFloat? = nil) -> CGAffineTransform {
        transform(placement < q.places.count ? q.places[placement] : .identity, offsetY: offsetY)
    }
    /// The reader's pan and zoom as the engine has it.
    var currentView: QvpView { QvpView(scale: Float(viewScale), offsetX: Float(viewOx), offsetY: Float(viewOy)) }
    /// Take what a gesture produced: the page is already laid out at the new size, and the view
    /// already holds the word the fingers were on.
    private func apply(_ c: QvpZoomChange) {
        zoom = c.zoom
        viewScale = CGFloat(c.view.scale); viewOx = CGFloat(c.view.offsetX); viewOy = CGFloat(c.view.offsetY)
        if c.relaid { drawList = nil; baseKey = "" }
        invalidateContent()
    }
    /// Layout viewport px (highlight / mask boxes) → view points.
    private func boxTransform(offsetY: CGFloat) -> CGAffineTransform {
        CGAffineTransform(a: viewScale, b: 0, c: 0, d: viewScale, tx: viewOx, ty: offsetY)
    }

    private func drawBoxes(_ ctx: CGContext, _ boxes: [QvpBox], offsetY: CGFloat) {
        if boxes.isEmpty { return }
        ctx.saveGState(); ctx.concatenate(boxTransform(offsetY: offsetY))
        var curId = Int.min, curColor: UInt32 = 0, path: CGMutablePath?
        func flush() { if let p = path { ctx.setFillColor(QvpColor.cgColor(curColor)); ctx.addPath(p); ctx.fillPath(using: .winding) }; path = nil }
        for b in boxes {
            if path == nil || b.id != curId || b.color != curColor { flush(); path = CGMutablePath(); curId = b.id; curColor = b.color }
            let r = CGRect(x: CGFloat(b.x0), y: CGFloat(b.y0), width: CGFloat(b.x1 - b.x0), height: CGFloat(b.y1 - b.y0))
            if b.radius > 0 { path!.addRoundedRect(in: r, cornerWidth: min(CGFloat(b.radius), r.width / 2), cornerHeight: min(CGFloat(b.radius), r.height / 2)) } else { path!.addRect(r) }
        }
        flush(); ctx.restoreGState()
    }

    // ── frame ──
    /// The page content is one image in `contentLayer`, covering a band of the page. Scrolling
    /// repositions that layer and draws nothing; everything else rebuilds it.
    public override func draw(_ rect: CGRect) {}

    /// A host that has changed what the page shows — a style, a highlight, the reveal — says so
    /// the way it says so to any view. The content is drawn again, since this view's drawing is
    /// an image it keeps rather than something UIKit asks it for each time.
    public override func setNeedsDisplay() {
        super.setNeedsDisplay()
        invalidateContent()
    }

    /// The content has changed — a style, the selection, the layout, the page — so the band is
    /// built again.
    private func invalidateContent() { baseKey = ""; refresh() }

    /// Put the band where the reader is, building it again first if it no longer covers them.
    private func refresh() {
        syncScroller()
        rebuildBand()
        CATransaction.begin(); CATransaction.setDisableActions(true)
        contentLayer.frame = CGRect(x: 0, y: bandOriginY, width: bounds.width, height: bandHeight)
        CATransaction.commit()
    }

    /// Hand the scroll view the page's height, so the reader scrolls the page rather than a
    /// view of it. A page that fits the screen does not scroll at all, and a magnified one is
    /// dragged in both directions by our own pan instead.
    private func syncScroller() {
        scroller.frame = bounds
        let contentH = (page?.currentLayout).map { CGFloat($0.contentH) * viewScale } ?? bounds.height
        let reflowed = isReflowed
        scroller.isScrollEnabled = reflowed
        scroller.showsVerticalScrollIndicator = reflowed
        let size = CGSize(width: bounds.width, height: reflowed ? max(contentH, bounds.height) : bounds.height)
        if scroller.contentSize != size { scroller.contentSize = size }
        contentHost.frame = CGRect(origin: .zero, size: size)
        // the reader's place is one number, kept on both sides of it
        if reflowed, !scroller.isDragging, !scroller.isDecelerating, abs(scroller.contentOffset.y + viewOy) > 0.5 {
            scroller.setContentOffset(CGPoint(x: 0, y: -viewOy), animated: false)
        }
    }

    /// The scroll view moved the page: take the reader's place from it and follow with the band.
    public func scrollViewDidScroll(_ sv: UIScrollView) {
        guard isReflowed else { return }
        viewOy = -sv.contentOffset.y
        // The ink is laid out down the scroll view's own content, so the scroll view has already
        // moved it. There is nothing left to do unless the reader has scrolled off the end of
        // what is drawn.
        if inkCoversScreen { return }
        rebuildBand()
        CATransaction.begin(); CATransaction.setDisableActions(true)
        contentLayer.frame = CGRect(x: 0, y: bandOriginY, width: bounds.width, height: bandHeight)
        CATransaction.commit()
    }

    /// A sideways swipe turns the page. The scroll view owns the up-and-down axis and locks the
    /// sideways one, so the gesture is free to mean this whether or not the reader has zoomed in.
    @objc private func onSwipeGesture(_ g: UISwipeGestureRecognizer) {
        guard let cb = onSwipe, !selecting, sideways == .turnPage else { return }
        cb(g.direction == .right ? 1 : -1)
    }

    private func rebuildBand() {
        guard let p = page, p.isOpen else { contentLayer.contents = nil; return }
        if p.currentLayout == nil { relayout() }
        let moving = p.tick(CACurrentMediaTime() * 1000)
        // a page whose colours are still moving is drawn again on every frame, which is what a
        // fade is, so it is never taken from what was kept
        guard let (key, image) = renderInk(p, zoom, offsetY: viewOy, reuse: !moving) else { return }
        base = image; baseKey = key
        CATransaction.begin(); CATransaction.setDisableActions(true)
        contentLayer.contents = image
        contentLayer.contentsScale = window?.screen.scale ?? contentScaleFactor
        CATransaction.commit()
        setAnimating(moving)
    }

    /// Draw one page's ink, or hand back what was drawn before. `offsetY` is where the reader
    /// stands on it: the band is taken from there.
    @discardableResult
    private func renderInk(_ p: QvpPage, _ z: QvpZoom, offsetY: CGFloat, reuse: Bool = true) -> (String, CGImage)? {
        guard p.isOpen else { return nil }
        let spec = p.zoomSpec(baseSpec, z)
        guard let l = p.currentLayout?.reflowed == (spec.reflow != nil) && p === page ? p.currentLayout : p.layout(spec) else { return nil }
        let styled = p.styledPaths()
        let styledSet = Set(styled.map { $0.path })
        let ink = p.defaultInk
        let scale = window?.screen.scale ?? contentScaleFactor
        let (top, bandH) = band(for: l, offsetY: offsetY, scale: scale)
        let W = Int(bounds.width * scale), bandPx = Int((bandH * scale).rounded(.up))
        guard W > 0, bandPx > 0 else { return nil }
        let bands = p.highlightBoxesView()
        let masks = p.maskBoxesView()
        var hasher = Hasher()
        hasher.combine(styledSet.sorted()); hasher.combine(l.lineDy)
        hasher.combine(bands.map { $0.id }); hasher.combine(masks.map { $0.id })
        // The key holds where the band starts, never where the reader has scrolled to, so a
        // drag inside the band changes nothing.
        let key = "\(p.pageNo)|\(viewScale)|\(viewOx)|\(top)|\(ink)|\(l.lineSpacing)|\(l.scale)|\(hasher.finalize())|\(W)x\(bandPx)|\(z.zoom)|\(l.rows)"
        if reuse, let kept = cachedInk(key) { return (key, kept) }

        let t0 = CACurrentMediaTime()
        let cs = CGColorSpaceCreateDeviceRGB()
        guard let bc = CGContext(data: nil, width: W, height: bandPx, bitsPerComponent: 8, bytesPerRow: 0, space: cs,
                                 bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
        else { return nil }
        // flip to UIKit's y-down space, and put the band's own top row at y = 0
        bc.translateBy(x: 0, y: CGFloat(bandPx)); bc.scaleBy(x: scale, y: -scale)
        bc.setAllowsAntialiasing(true); bc.setShouldAntialias(true)
        let off = -top
        if let paper = paperColor {
            bc.setFillColor(paper.cgColor)
            bc.fill(CGRect(x: viewOx, y: off, width: CGFloat(l.contentW) * viewScale, height: CGFloat(l.contentH) * viewScale))
        }
        drawBoxes(bc, bands, offsetY: off)
        bc.setFillColor(QvpColor.cgColor(ink))
        let draws = p.layoutDrawList(band: (top: Float(top / max(viewScale, 0.001)), bottom: Float((top + bandH) / max(viewScale, 0.001))))
        let places = p.layoutPlacements()
        let place = { (i: Int) -> QvpPlacement in i < places.count ? places[i] : .identity }
        var cur = -1, n = 0
        for d in draws where !styledSet.contains(d.path) {
            if d.placement != cur { if cur >= 0 { bc.restoreGState() }; bc.saveGState(); bc.concatenate(transform(place(d.placement), offsetY: off)); cur = d.placement }
            bc.addPath(paths(p)[d.path]); bc.fillPath(using: p.pathEvenOdd(d.path) ? .evenOdd : .winding); n += 1
        }
        if cur >= 0 { bc.restoreGState() }
        lastBaseMs = (CACurrentMediaTime() - t0) * 1000; lastBasePaths = n
        let t1 = CACurrentMediaTime()
        cur = -1
        let colors = Dictionary(styled, uniquingKeysWith: { a, _ in a })
        for d in draws {
            guard let col = colors[d.path], col & 0xff != 0 else { continue }
            if d.placement != cur { if cur >= 0 { bc.restoreGState() }; bc.saveGState(); bc.concatenate(transform(place(d.placement), offsetY: off)); cur = d.placement }
            bc.setFillColor(QvpColor.cgColor(col)); bc.addPath(paths(p)[d.path]); bc.fillPath(using: p.pathEvenOdd(d.path) ? .evenOdd : .winding)
        }
        if cur >= 0 { bc.restoreGState() }
        drawBoxes(bc, masks, offsetY: off)
        lastOverlayMs = (CACurrentMediaTime() - t1) * 1000; lastOverlayPaths = styled.count; lastBands = bands.count

        guard let image = bc.makeImage() else { return nil }
        keepInk(key, image)
        return (key, image)
    }

    /// The band of one page worth drawing: the whole of it when it fits the ink budget, and two
    /// screens around the reader when it does not.
    private func band(for l: QvpLayout, offsetY: CGFloat, scale: CGFloat) -> (top: CGFloat, height: CGFloat) {
        let contentH = CGFloat(l.contentH) * viewScale
        let whole = (contentH * scale).rounded(.up) <= 16384
            && Int((bounds.width * scale).rounded(.up) * (contentH * scale).rounded(.up)) * 4 <= inkBudget
        if l.reflowed, whole { return (0, max(contentH, 1)) }
        if !l.reflowed { return (0, max(bounds.height, 1)) }
        let step = max(bounds.height / 2, 1)
        return (((-offsetY - step) / step).rounded(.down) * step, max(bounds.height * 2, 1))
    }

    private func paths(_ p: QvpPage) -> [CGPath] { p.buildPaths() }

    private func setAnimating(_ on: Bool) {
        animating = on
        if on, link == nil { let l = CADisplayLink(target: self, selector: #selector(onFrame)); l.add(to: .main, forMode: .common); link = l }
        if !on, let l = link { l.invalidate(); link = nil }
    }
    @objc private func onFrame() { invalidateContent() }
}
#endif
