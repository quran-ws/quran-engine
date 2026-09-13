#if canImport(UIKit)
import UIKit
import QuartzCore

/// Host-canvas renderer for a `QvpPage` (CoreGraphics). Draw order per frame:
/// highlight bands (one path per highlight id, nonzero, behind the ink) → cached base ink
/// (a bitmap of every non-styled path at the current transform, rebuilt only when the styled
/// set / layout / transform changes) → styled ink from `styled()` → mask boxes.
/// Each frame calls `page.tick(now)`; a CADisplayLink keeps running while the engine says so.
/// Gestures: tap → gap-aware hit-test → `onWordTap` / `onDecoTap` / `onEmptyTap`; long-press +
/// drag → whole-word selection (engine `select`, band in the selection layer); pinch / pan on
/// top of the engine layout; double-tap resets the view (or runs `onDoubleTap` when set).
public final class QvpPageView: UIView, UIGestureRecognizerDelegate {
    public var page: QvpPage? {
        didSet { base = nil; baseKey = ""; selectionHandle = 0; relayout(); resetView(); setNeedsDisplay() }
    }
    // layout knobs (viewport size comes from the view)
    public var padTop: CGFloat = 0 { didSet { relayout() } }
    public var padBottom: CGFloat = 0 { didSet { relayout() } }
    public var padSide: CGFloat = 0 { didSet { relayout() } }
    public var lineSpacing: Float = 1 { didSet { relayout() } }
    public var lineGap: Float = 0 { didSet { relayout() } }
    public var fillHeight = false { didSet { relayout() } }
    /// Paper behind the page content (nil = transparent).
    public var paperColor: UIColor? { didSet { setNeedsDisplay() } }
    /// 0xRRGGBBAA band colour of the drag selection.
    public var selectionBand: UInt32 = QvpDefaults.SELECTION_BAND
    public var onWordTap: ((QvpWord, QvpHitEx) -> Void)?
    public var onDecoTap: ((QvpDecoration, QvpHitEx) -> Void)?
    public var onEmptyTap: (() -> Void)?
    public var onSelectionChanged: (([Int]) -> Void)?
    /// Horizontal swipe while the page is not zoomed in: +1 = finger moved right, -1 = left. The host flips pages.
    public var onSwipe: ((Int) -> Void)?
    /// Double-tap. nil (the default) resets the view; a host that repurposes the gesture
    /// (e.g. marking reading progress) can still call `resetView()` itself — `isZoomed` says when.
    public var onDoubleTap: ((QvpHitEx?) -> Void)?
    /// Long-press, with the gap-aware hit under the finger. Fires once, on recognition,
    /// only while `selectionEnabled` is false — selection owns the long-press otherwise.
    public var onLongPress: ((QvpHitEx?) -> Void)?
    public var longPressDuration: Double = 0.35 { didSet { longPressRecognizer?.minimumPressDuration = longPressDuration } }
    private weak var longPressRecognizer: UILongPressGestureRecognizer?
    public var zoomEnabled = true
    /// Zoom lasts only while the fingers are down: on release the page eases back to its fitted
    /// size — a peek, not a reading zoom — so a pinch never leaves the page holding the pan.
    public var zoomSpringsBack = false
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
    private var selectionHandle = 0
    private var selAnchor = -1
    private var selecting = false
    private var link: CADisplayLink?
    private var spring: QvpZoomSpring?
    private var springLink: CADisplayLink?
    private var pinchStart: CGFloat = 1
    private var fitScale: CGFloat = 1
    /// True once the reader pinched in beyond the fitted size (panning then moves the page, not the book).
    public var isZoomed: Bool { viewScale > fitScale * QvpViewPolicy.zoomedThreshold }

    public override init(frame: CGRect) { super.init(frame: frame); setup() }
    public required init?(coder: NSCoder) { super.init(coder: coder); setup() }
    private func setup() {
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
    }
    deinit { link?.invalidate(); springLink?.invalidate() }

    public func gestureRecognizer(_ a: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith b: UIGestureRecognizer) -> Bool {
        (a is UIPinchGestureRecognizer && b is UIPanGestureRecognizer) || (a is UIPanGestureRecognizer && b is UIPinchGestureRecognizer)
    }

    // ── gestures ──
    private func hitAt(_ pt: CGPoint, _ o: QvpHitOptions? = nil) -> QvpHitEx? {
        guard let p = page, p.isOpen else { return nil }
        let t0 = CACurrentMediaTime()
        let h = p.hitTestViewEx(Float((pt.x - viewOx) / viewScale), Float((pt.y - viewOy) / viewScale), o ?? hitOptions)
        lastHitUs = (CACurrentMediaTime() - t0) * 1e6
        return h
    }
    @objc private func onTap(_ g: UITapGestureRecognizer) {
        guard let p = page else { return }
        let hit = hitAt(g.location(in: self))
        if let h = hit, h.word >= 0 { onWordTap?(p.words[h.word], h) }
        else if let h = hit, h.deco >= 0 { onDecoTap?(p.decos[h.deco], h) }
        else { onEmptyTap?() }
    }
    @objc private func handleDoubleTap(_ g: UITapGestureRecognizer) { if let cb = onDoubleTap { cb(hitAt(g.location(in: self))) } else { resetView() } }
    @objc private func onPinch(_ g: UIPinchGestureRecognizer) {
        guard zoomEnabled, !selecting else { return }
        if g.state == .began { stopSpring(); pinchStart = viewScale }
        if zoomSpringsBack, [.ended, .cancelled, .failed].contains(g.state) { springBack(); return }
        let ns = QvpViewPolicy.clampZoom(pinchStart * g.scale); let k = ns / viewScale
        let f = g.location(in: self)
        viewOx = f.x - (f.x - viewOx) * k; viewOy = f.y - (f.y - viewOy) * k; viewScale = ns
        setNeedsDisplay()
    }
    @objc private func onPan(_ g: UIPanGestureRecognizer) {
        guard zoomEnabled, !selecting, spring == nil else { return }
        if !isZoomed, onSwipe != nil {
            if g.state == .ended {
                let t = g.translation(in: self), v = g.velocity(in: self)
                if let dir = QvpViewPolicy.swipeDirection(translation: CGSize(width: t.x, height: t.y), velocity: CGSize(width: v.x, height: v.y)) { onSwipe?(dir) }
            }
            return
        }
        let d = g.translation(in: self)
        viewOx += d.x; viewOy += d.y; g.setTranslation(.zero, in: self)
        setNeedsDisplay()
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
        if selectionHandle != 0 { p.rehighlight(selectionHandle, t) }
        else { selectionHandle = p.highlight(t, QvpHighlightStyle(mode: .band, band: selectionBand, padX: 0.6, layer: QvpLayer.SELECTION)) }
        onSelectionChanged?(ws); setNeedsDisplay()
    }
    /// Clear the selection band and the engine selection.
    public func clearSelection() {
        guard let p = page else { return }
        p.clearSelection(); if selectionHandle != 0 { p.unhighlight(selectionHandle); selectionHandle = 0 }
        onSelectionChanged?([]); setNeedsDisplay()
    }

    // ── layout ──
    public override func layoutSubviews() { super.layoutSubviews(); if bounds.size != lastSize { lastSize = bounds.size; relayout(); resetView() } }
    private var lastSize = CGSize.zero
    /// Recompute the engine layout for the current size / knobs.
    public func relayout() {
        guard let p = page, p.isOpen, bounds.width > 0, bounds.height > 0 else { return }
        p.layout(QvpLayoutSpec(viewportW: Float(bounds.width), viewportH: Float(bounds.height), padTop: Float(padTop), padBottom: Float(padBottom), padLeft: Float(padSide), padRight: Float(padSide), lineSpacing: lineSpacing, lineGap: lineGap, fillHeight: fillHeight))
        baseKey = ""; setNeedsDisplay()
    }
    /// Fit the content height and centre it.
    public func resetView() {
        stopSpring()
        let f = fittedView()
        viewScale = f.scale; fitScale = f.scale; viewOx = f.ox; viewOy = f.oy
        setNeedsDisplay()
    }
    /// The transform `resetView()` applies: content height fitted, centred.
    private func fittedView() -> QvpZoomSpring.ViewTransform {
        let l = page?.currentLayout
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
        viewScale = v.scale; viewOx = v.ox; viewOy = v.oy
        setNeedsDisplay()
        if done { stopSpring() }
    }
    private func stopSpring() { spring = nil; springLink?.invalidate(); springLink = nil }
    /// Page units of `line` → view points (engine layout + pan/zoom).
    public func lineTransform(_ line: Int) -> CGAffineTransform {
        let l = page?.currentLayout
        let ls = CGFloat(l?.scale ?? 1), lox = CGFloat(l?.ox ?? 0)
        let dy = (l.flatMap { line < $0.lineDy.count ? $0.lineDy[line] : nil }) ?? 0
        let loy = CGFloat(l?.oy ?? 0) + CGFloat(dy) * ls
        let s = viewScale * ls
        return CGAffineTransform(a: s, b: 0, c: 0, d: s, tx: viewOx + viewScale * lox, ty: viewOy + viewScale * loy)
    }
    /// Layout viewport px (highlight / mask boxes) → view points.
    private var boxTransform: CGAffineTransform { CGAffineTransform(a: viewScale, b: 0, c: 0, d: viewScale, tx: viewOx, ty: viewOy) }

    private func drawBoxes(_ ctx: CGContext, _ boxes: [QvpBox]) {
        if boxes.isEmpty { return }
        ctx.saveGState(); ctx.concatenate(boxTransform)
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
    public override func draw(_ rect: CGRect) {
        guard let p = page, p.isOpen, let ctx = UIGraphicsGetCurrentContext() else { return }
        if p.currentLayout == nil { relayout() }
        guard let l = p.currentLayout else { return }
        let moving = p.tick(CACurrentMediaTime() * 1000)
        let paths = p.buildPaths()
        let styled = p.styled()
        let styledSet = Set(styled.map { $0.path })
        let ink = p.defaultInk
        let scale = window?.screen.scale ?? contentScaleFactor
        let W = Int(bounds.width * scale), H = Int(bounds.height * scale)
        var hasher = Hasher(); hasher.combine(styledSet.sorted()); hasher.combine(l.lineDy)
        let key = "\(viewScale)|\(viewOx)|\(viewOy)|\(ink)|\(l.pitch)|\(l.scale)|\(hasher.finalize())|\(W)x\(H)"

        if let paper = paperColor {
            ctx.setFillColor(paper.cgColor)
            ctx.fill(CGRect(x: viewOx, y: viewOy, width: CGFloat(l.contentW) * viewScale, height: CGFloat(l.contentH) * viewScale))
        }
        let bands = p.highlightBoxes()
        drawBoxes(ctx, bands)

        if base == nil || key != baseKey || base!.width != W || base!.height != H, W > 0, H > 0 {
            let t0 = CACurrentMediaTime()
            let cs = CGColorSpaceCreateDeviceRGB()
            if let bc = CGContext(data: nil, width: W, height: H, bitsPerComponent: 8, bytesPerRow: 0, space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue) {
                // flip to UIKit's y-down space so the bitmap is drawn back without another flip
                bc.translateBy(x: 0, y: CGFloat(H)); bc.scaleBy(x: scale, y: -scale)
                bc.setFillColor(QvpColor.cgColor(ink))
                bc.setAllowsAntialiasing(true); bc.setShouldAntialias(true)
                var cur = -1, n = 0
                for pi in 0..<p.nPaths where !styledSet.contains(pi) {
                    let ln = p.pathLine(pi)
                    if ln != cur { if cur >= 0 { bc.restoreGState() }; bc.saveGState(); bc.concatenate(lineTransform(ln)); cur = ln }
                    bc.addPath(paths[pi]); bc.fillPath(using: p.pathEvenOdd(pi) ? .evenOdd : .winding); n += 1
                }
                if cur >= 0 { bc.restoreGState() }
                base = bc.makeImage(); baseKey = key
                lastBaseMs = (CACurrentMediaTime() - t0) * 1000; lastBasePaths = n
            }
        }
        if let img = base {
            ctx.saveGState()
            ctx.translateBy(x: 0, y: bounds.height); ctx.scaleBy(x: 1, y: -1)
            ctx.draw(img, in: CGRect(origin: .zero, size: bounds.size))
            ctx.restoreGState()
        }
        let t1 = CACurrentMediaTime()
        var cur = -1
        for (pi, col) in styled where col & 0xff != 0 {
            let ln = p.pathLine(pi)
            if ln != cur { if cur >= 0 { ctx.restoreGState() }; ctx.saveGState(); ctx.concatenate(lineTransform(ln)); cur = ln }
            ctx.setFillColor(QvpColor.cgColor(col)); ctx.addPath(paths[pi]); ctx.fillPath(using: p.pathEvenOdd(pi) ? .evenOdd : .winding)
        }
        if cur >= 0 { ctx.restoreGState() }
        drawBoxes(ctx, p.maskBoxes())
        lastOverlayMs = (CACurrentMediaTime() - t1) * 1000; lastOverlayPaths = styled.count; lastBands = bands.count
        setAnimating(moving)
    }
    private func setAnimating(_ on: Bool) {
        animating = on
        if on, link == nil { let l = CADisplayLink(target: self, selector: #selector(onFrame)); l.add(to: .main, forMode: .common); link = l }
        if !on, let l = link { l.invalidate(); link = nil }
    }
    @objc private func onFrame() { setNeedsDisplay() }
}
#endif
