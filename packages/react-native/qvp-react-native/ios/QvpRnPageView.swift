// @quran.ws/qvp-react-native, iOS. The React Native host of QvpKit's QvpPageView.
import UIKit
import React

/// The React Native host of `QvpPageView`. It owns the loaded `QvpPage`, loads bytes from the
/// prop-given URI, and reconciles the declarative props (theme, styles, highlights, mask, reveal,
/// dress) against engine handles, so JS never sees a handle. Every decision (hit test, layout,
/// style resolution, bands, masks) is the engine's. This class marshals and forwards, as
/// android/.../QvpRnPageView.kt does.
///
/// The module reads the page from its query queue while this view draws on the main thread (see
/// `read`). Those reads touch only what the page fixed when it loaded, so the two do not meet,
/// except where the page is closed, replaced, or given a words sidecar: each of these holds the
/// lock for writing, and waits for a query in progress to finish.
///
/// Lengths are in points, which are the dp of the JS side.
@objc(QvpRnPageView)
final class QvpRnPageView: UIView {
    /// QvpKit's renderer, hosted rather than subclassed.
    let inner = QvpPageView(frame: .zero)
    var page: QvpPage? { inner.page }

    private var lock = pthread_rwlock_t()
    /// Run `block` on the page in view, held open until the block returns: nil when no page is
    /// loaded. The block must not wait for the main thread, which can be waiting for this lock.
    func read<T>(_ block: (QvpPage?) throws -> T) rethrows -> T {
        pthread_rwlock_rdlock(&lock); defer { pthread_rwlock_unlock(&lock) }
        return try block(page)
    }
    private func write<T>(_ block: () throws -> T) rethrows -> T {
        pthread_rwlock_wrlock(&lock); defer { pthread_rwlock_unlock(&lock) }
        return try block()
    }
    /// Attach a words sidecar. Queries in progress finish first, and the word records built
    /// before it are dropped, since the forms they hold have changed.
    func attachWords(_ p: QvpPage, _ json: String) -> Int { write { let n = p.attachWords(json); dropWords(); return n } }

    private let wordsLock = NSLock()
    private var wordTable: QvpRnWords?
    /// The word records of `p`, built on first use and kept until the page changes or takes a sidecar.
    func words(_ p: QvpPage) -> QvpRnWords {
        wordsLock.lock(); defer { wordsLock.unlock() }
        if let w = wordTable, w.page === p { return w }
        let w = QvpRnWords(p); wordTable = w; return w
    }
    private func dropWords() { wordsLock.lock(); wordTable = nil; wordsLock.unlock() }

    override init(frame: CGRect) {
        super.init(frame: frame)
        pthread_rwlock_init(&lock, nil)
        inner.frame = bounds
        inner.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        addSubview(inner)
        inner.onWordTap = { [weak self] w, h in
            guard let self, let p = self.page else { return }
            self.emit(self.onWordTap, ["word": self.words(p).word(w.index), "hit": QvpRnMarshal.hit(p, h)])
        }
        inner.onDecorationTap = { [weak self] d, h in
            guard let self, let p = self.page else { return }
            self.emit(self.onDecorationTap, ["decoration": QvpRnMarshal.decoration(d), "hit": QvpRnMarshal.hit(p, h)])
        }
        inner.onEmptyTap = { [weak self] in self?.emit(self?.onEmptyTap, [:]) }
        // The engine says a flick turns the page and by how many, in reading order. Which page
        // that is belongs to the app, so this is an event and not a page change.
        inner.onSwipe = { [weak self] pages in self?.emit(self?.onSwipe, ["pages": pages]) }
        inner.onZoomChanged = { [weak self] z in
            self?.emit(self?.onZoomChanged, ["mode": QvpRnMarshal.jsName(z.mode), "step": z.step, "zoom": z.zoom])
        }
        inner.onSelectionChanged = { [weak self] _ in
            guard let self, let p = self.page else { return }
            self.emit(self.onSelectionChanged, QvpRnMarshal.selection(p))
        }
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }
    deinit {
        QvpRnRegistry.unregister(self)
        page?.close()
        pthread_rwlock_destroy(&lock)
    }

    // ── events ──
    @objc var onWordTap: RCTDirectEventBlock?
    @objc var onDecorationTap: RCTDirectEventBlock?
    @objc var onEmptyTap: RCTDirectEventBlock?
    @objc var onSelectionChanged: RCTDirectEventBlock?
    @objc var onPageLoad: RCTDirectEventBlock?
    @objc var onRevealChanged: RCTDirectEventBlock?
    @objc var onSwipe: RCTDirectEventBlock?
    @objc var onZoomChanged: RCTDirectEventBlock?
    @objc var onDressChanged: RCTDirectEventBlock?
    @objc var onError: RCTDirectEventBlock?
    private func emit(_ block: RCTDirectEventBlock?, _ payload: [String: Any?]) { block?(QvpRnMarshal.toJsMap(payload)) }

    // ── props (set by the manager; `commit` applies a batch in one pass) ──
    @objc var pageUri: String? { didSet { changed() } }
    @objc var pageBase64: String? { didSet { changed() } }
    @objc var wordsUri: String? { didSet { changed() } }
    @objc var padTop: CGFloat = 0 { didSet { inner.padTop = padTop; layoutDirty = true; changed() } }
    @objc var padBottom: CGFloat = 0 { didSet { inner.padBottom = padBottom; layoutDirty = true; changed() } }
    @objc var padSide: CGFloat = 0 { didSet { inner.padSide = padSide; layoutDirty = true; changed() } }
    @objc var lineSpacing: CGFloat = 1 { didSet { inner.lineSpacing = Float(lineSpacing); layoutDirty = true; changed() } }
    @objc var reflowBreaks: String? {
        didSet {
            let v = reflowBreaks?.lowercased()
            inner.reflowBreaks = [QvpBreaks.greedy, .even, .fitted, .printed].first { "\($0)" == v } ?? .fitted
            layoutDirty = true; changed()
        }
    }
    @objc var fillHeight = false { didSet { inner.fillHeight = fillHeight; layoutDirty = true; changed() } }
    @objc var surahFrames = true { didSet { inner.surahFrames = surahFrames; layoutDirty = true; changed() } }
    @objc var paperColor: String? {
        didSet {
            let s = paperColor?.trimmingCharacters(in: .whitespaces) ?? ""
            inner.paperColor = s.isEmpty ? nil : UIColor(cgColor: QvpColor.cgColor(QvpRnMarshal.color(s, 0)))
            inner.setNeedsDisplay()
        }
    }
    @objc var defaultInk: String? { didSet { changed() } }
    @objc var selectionBand: String? {
        didSet { if let s = selectionBand, !s.trimmingCharacters(in: .whitespaces).isEmpty { inner.selectionBand = QvpRnMarshal.color(s, inner.selectionBand) } }
    }
    @objc var selectionEnabled = true { didSet { inner.selectionEnabled = selectionEnabled } }
    @objc var zoomEnabled = true { didSet { inner.zoomEnabled = zoomEnabled } }
    @objc var zoomMode: String? { didSet { changed() } }
    @objc var zoomStep: Int = -1 { didSet { changed() } }
    @objc var hitMaxDistance: CGFloat = CGFloat(QvpDefaults.TAP_DISTANCE) { didSet { inner.hitOptions.maxDistance = Float(hitMaxDistance) } }
    @objc var theme: NSDictionary? { didSet { changed() } }
    @objc var styles: NSArray? { didSet { changed() } }
    @objc var highlights: NSArray? { didSet { changed() } }
    /// The `mask` prop. UIView has a `mask` of its own, so QvpModule.m maps the prop to this name.
    @objc var maskProp: NSDictionary? { didSet { changed() } }
    @objc var reveal: NSDictionary? { didSet { changed() } }
    @objc var dress: NSDictionary? { didSet { changed() } }
    private var layoutDirty = true

    // ── applied state ──
    private var loadedKey: String?
    private var appliedTheme: NSDictionary?
    private var themeHandle = 0
    private struct Rule { let handle: Int; let entry: NSDictionary }
    private var styleHandles: [(id: String, rule: Rule)] = []
    private var highlightHandles: [(id: String, rule: Rule)] = []
    private var appliedMask: NSDictionary?
    private var appliedReveal: NSDictionary?   // without "at"
    private var appliedRevealAt: Int?
    private var appliedDress: NSDictionary?
    private var appliedInk: UInt32?
    private(set) var revealStepCount = 0
    private(set) var loadMs = 0.0
    private(set) var pageBytes = 0

    /// A prop changed: commit once, after the rest of the batch has been set.
    private var commitQueued = false
    private func changed() {
        if commitQueued { return }
        commitQueued = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.commitQueued = false
            self.commit()
        }
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil { QvpRnRegistry.register(self) }
    }

    /// Fit and centre, as the engine's layout says.
    func resetView() { inner.resetView() }

    // ── the reader's zoom control ──
    // What a pinch does to the page: reflow onto the page's own steps (the default), reflow to
    // the zoom the fingers ask for, or magnify the printed page. The engine owns the policy;
    // these props only say which one the app wants and which step to sit on.
    private func commitZoom() {
        guard let p = page, p.isOpen else { return }
        if let name = zoomMode {
            let mode: QvpZoomMode = name == "continuous" ? .continuous : name == "magnify" ? .magnify : .stepped
            if mode != inner.zoomMode { inner.zoomMode = mode }
        }
        if zoomStep >= 0, zoomStep != inner.zoom.step { inner.zoomToStep(zoomStep) }
    }

    /// Apply the props set since the last commit.
    func commit() {
        QvpRnRegistry.register(self)
        do {
            try ensurePage()
            let p = page
            if let p {
                applyInk(p); applyTheme(p); applyStyles(p); applyHighlights(p); applyMask(p); applyReveal(p)
            }
            var fit = false
            if layoutDirty { layoutDirty = false; inner.relayout(); fit = true }
            // After the layout: the engine places the ornaments on the layout in force.
            if let p, applyDress(p) { fit = true }
            if fit { resetView() }
            commitZoom()
            inner.setNeedsDisplay()
        } catch {
            NSLog("QvpRn: commit failed: %@", "\(error)")
            emit(onError, ["message": error.localizedDescription])
        }
    }

    private func ensurePage() throws {
        let key = pageBase64.map { "b64:\($0.hashValue)" } ?? pageUri
        if key == loadedKey { return }
        // drop everything bound to the old page: handles die with it
        themeHandle = 0; appliedTheme = nil; styleHandles = []; highlightHandles = []; appliedMask = nil; appliedReveal = nil
        appliedRevealAt = nil; appliedInk = nil; appliedDress = nil; revealStepCount = 0
        loadedKey = key
        guard let key else { swap(nil); return }
        // Load the new page before the swap, so a query sees the old page or the new one and
        // never a view with no page. A load that fails leaves no page.
        let p: QvpPage, bytes: Data, ms: Double
        do {
            if let b64 = pageBase64 {
                guard let d = Data(base64Encoded: b64, options: .ignoreUnknownCharacters) else { throw QvpRnError("pageBase64 is not base64") }
                bytes = d
            } else {
                bytes = try QvpRnBytes.read(key)
            }
            let t0 = CFAbsoluteTimeGetCurrent()
            do { p = try QvpPage(bytes: bytes) } catch { throw QvpRnError("not a QVP page: \(key.prefix(64))") }
            ms = (CFAbsoluteTimeGetCurrent() - t0) * 1000
            // No query sees the page before the swap below, so the sidecar needs no lock.
            if let w = wordsUri {
                do { _ = p.attachWords(try QvpRnBytes.read(w)) } catch { NSLog("QvpRn: words sidecar: %@", "\(error)") }
            }
        } catch {
            swap(nil)
            throw error
        }
        loadMs = ms; pageBytes = bytes.count
        swap(p)
        layoutDirty = true
        emit(onPageLoad, QvpRnMarshal.pageInfo(p, words(p).forms).merging(["loadMs": loadMs, "bytes": pageBytes, "uri": pageUri]) { a, _ in a })
    }
    /// Put `p` in view and close the old page, with no query reading either.
    private func swap(_ p: QvpPage?) {
        write {
            let old = inner.page
            inner.page = p
            dropWords()
            if old !== p { old?.close() }
        }
    }

    private func applyInk(_ p: QvpPage) {
        let ink = QvpRnMarshal.color(defaultInk, QvpDefaults.INK)
        if appliedInk != ink { appliedInk = ink; p.setDefaultColor(ink) }
    }
    private func applyTheme(_ p: QvpPage) {
        if same(theme, appliedTheme) { return }
        if themeHandle != 0 { p.removeStyle(themeHandle); themeHandle = 0 }
        appliedTheme = theme
        if let t = theme { themeHandle = p.theme(QvpRnMarshal.theme(t as? [String: Any])) }
    }
    /// The entries of a list prop by id, in order; an entry without an id is skipped.
    private func entries(_ list: NSArray?) -> [(id: String, entry: NSDictionary)] {
        var out: [(id: String, entry: NSDictionary)] = []
        for case let m as NSDictionary in (list ?? []) {
            guard let raw = QvpRnMarshal.some(m["id"]) else { continue }
            let id = "\(raw)"
            if let i = out.firstIndex(where: { $0.id == id }) { out[i].entry = m } else { out.append((id, m)) }
        }
        return out
    }
    private func applyStyles(_ p: QvpPage) {
        let want = entries(styles)
        let wantIds = Set(want.map(\.id))
        for r in styleHandles where !wantIds.contains(r.id) { p.removeStyle(r.rule.handle) }
        var old = Dictionary(styleHandles.filter { wantIds.contains($0.id) }.map { ($0.id, $0.rule) }) { a, _ in a }
        var next: [(id: String, rule: Rule)] = []
        for (id, e) in want {
            if let o = old[id], same(o.entry, e) { next.append((id, o)); continue }
            let hide = QvpRnMarshal.isTrue(e["hide"])
            if let o = old[id], !hide, same(o.entry["selector"], e["selector"]), same(o.entry["target"], e["target"]),
               same(o.entry["layer"], e["layer"]), same(o.entry["hide"], e["hide"]) {
                p.recolorStyle(o.handle, QvpRnMarshal.color(e["color"], 0), transitionMs: QvpRnMarshal.num(e["ms"]) ?? 0)
                next.append((id, Rule(handle: o.handle, entry: e))); continue
            }
            if let o = old[id] { p.removeStyle(o.handle); old[id] = nil }
            let ms = QvpRnMarshal.num(e["ms"]) ?? 0
            let layer = QvpRnMarshal.num(e["layer"]) ?? QvpLayer.BASE
            let color = QvpRnMarshal.color(e["color"], 0)
            var h = 0
            if hide { h = QvpRnMarshal.selector(e["selector"]).map { p.hide($0) } ?? 0 }
            else if QvpRnMarshal.some(e["selector"]) != nil { h = QvpRnMarshal.selector(e["selector"]).map { p.style($0, color, transitionMs: ms, layer: layer) } ?? 0 }
            else if QvpRnMarshal.some(e["target"]) != nil { h = p.styleTarget(QvpRnMarshal.target(e["target"], p), color, transitionMs: ms, layer: layer) }
            if h != 0 { next.append((id, Rule(handle: h, entry: e))) }
        }
        styleHandles = next
    }
    private func applyHighlights(_ p: QvpPage) {
        let want = entries(highlights)
        let wantIds = Set(want.map(\.id))
        for r in highlightHandles where !wantIds.contains(r.id) { p.removeHighlight(r.rule.handle) }
        let old = Dictionary(highlightHandles.filter { wantIds.contains($0.id) }.map { ($0.id, $0.rule) }) { a, _ in a }
        var next: [(id: String, rule: Rule)] = []
        for (id, e) in want {
            guard let o = old[id] else {
                let h = p.highlight(QvpRnMarshal.target(e["target"], p), QvpRnMarshal.highlightStyle(e["style"]))
                if h != 0 { next.append((id, Rule(handle: h, entry: e))) }
                continue
            }
            if !same(o.entry, e) {
                if !same(o.entry["style"], e["style"]) { p.restyleHighlight(o.handle, QvpRnMarshal.highlightStyle(e["style"])) }
                if !same(o.entry["target"], e["target"]) { p.moveHighlight(o.handle, QvpRnMarshal.target(e["target"], p)) }
            }
            next.append((id, Rule(handle: o.handle, entry: e)))
        }
        highlightHandles = next
    }
    private func applyMask(_ p: QvpPage) {
        if same(maskProp, appliedMask) { return }
        appliedMask = maskProp
        p.unmask()
        guard let m = maskProp else { return }
        p.maskOptions(blockColor: QvpRnMarshal.color(m["blockColor"], QvpDefaults.MASK_BLOCK), padX: QvpRnMarshal.flt(m["padX"], QvpDefaults.MASK_PAD),
                      padY: QvpRnMarshal.flt(m["padY"], QvpDefaults.MASK_PAD), radius: QvpRnMarshal.flt(m["radius"], QvpDefaults.MASK_RADIUS),
                      reverse: QvpRnMarshal.isTrue(m["reverse"]))
        if let from = QvpRnMarshal.num(m["from"]) { p.maskFrom(from, QvpRnMarshal.maskMode(m["mode"])) }
        else { p.mask(QvpRnMarshal.target(m["target"], p), QvpRnMarshal.maskMode(m["mode"])) }
    }
    private func applyReveal(_ p: QvpPage) {
        let r = reveal
        let cfg: NSDictionary? = r.map { d in
            let m = NSMutableDictionary(dictionary: d); m.removeObject(forKey: "at"); return m
        }
        if !same(cfg, appliedReveal) {
            appliedReveal = cfg; appliedRevealAt = nil
            guard let cfg else { p.revealStop(); revealStepCount = 0; emit(onRevealChanged, ["steps": 0, "at": nil]); return }
            revealStepCount = p.revealStart(lit: QvpRnMarshal.num(cfg["lit"]) ?? QvpDefaults.REVEAL_LIT, byAyah: QvpRnMarshal.isTrue(cfg["byAyah"]),
                                            grey: QvpRnMarshal.color(cfg["grey"], QvpDefaults.REVEAL_GREY), ink: QvpRnMarshal.color(cfg["ink"], appliedInk ?? QvpDefaults.INK),
                                            ayahMarks: !QvpRnMarshal.isFalse(cfg["ayahMarks"]), transitionMs: QvpRnMarshal.num(cfg["ms"]) ?? 0)
            emit(onRevealChanged, ["steps": revealStepCount, "at": -1])
        }
        guard let r else { return }
        let at = QvpRnMarshal.num(r["at"]) ?? -1
        if at != appliedRevealAt { appliedRevealAt = at; p.revealGoto(at) }
    }

    /// The `dress` prop: {ornaments: set id, ...spec}. A new object dresses the page again and
    /// null undresses it. The hosted QvpPageView paints the layer behind the ink and fits the
    /// content plus the dress overflow. True when the page changed.
    private func applyDress(_ p: QvpPage) -> Bool {
        if same(dress, appliedDress) { return false }
        appliedDress = dress
        var info: DressInfo?
        if let d = dress {
            if let set = QvpOrnamentSets.get(QvpRnMarshal.num(d["ornaments"])) {
                info = p.dress(set, QvpRnMarshal.dressSpec(d as? [String: Any]))
            } else {
                p.undress()
                emit(onError, ["message": "dress: no ornament set \(QvpRnMarshal.some(d["ornaments"]).map { "\($0)" } ?? "null")"])
            }
        } else {
            p.undress()
        }
        emit(onDressChanged, ["info": info.map { QvpRnMarshal.dressInfo($0) }])
        return true
    }
    /// The module's imperative dress: dress, fit the border in, draw.
    func dressNow(_ p: QvpPage, _ set: QvpOrnaments, _ spec: DressSpec) -> DressInfo? {
        let info = p.dress(set, spec); resetView(); inner.setNeedsDisplay(); return info
    }
    func undressNow(_ p: QvpPage) { p.undress(); resetView(); inner.setNeedsDisplay() }

    func stats() -> [String: Any?] {
        ["loadMs": loadMs, "bytes": pageBytes, "baseMs": inner.lastBaseMs, "overlayMs": inner.lastOverlayMs, "basePaths": inner.lastBasePaths,
         "overlayPaths": inner.lastOverlayPaths, "bands": inner.lastBands, "hitUs": inner.lastHitUs, "animating": inner.animating,
         "styleHandles": page.map { $0.isOpen ? $0.styleHandles().count : 0 } ?? 0, "highlightHandles": page.map { $0.isOpen ? $0.highlightHandles().count : 0 } ?? 0,
         "engineVersion": QvpEngine.version(), "viewScale": Double(inner.viewScale), "layout": page?.currentLayout.map { QvpRnMarshal.layout($0) }]
    }
}

/// Two prop values compared by value, as Kotlin compares its maps: nil and JS null are equal.
func same(_ a: Any?, _ b: Any?) -> Bool {
    let x = QvpRnMarshal.some(a), y = QvpRnMarshal.some(b)
    if x == nil || y == nil { return x == nil && y == nil }
    return (x as AnyObject).isEqual(y)
}

/// Views by react tag, so the module can find the page behind a `viewTag`. The main thread
/// registers views and the module's query queue looks them up. The registry holds them weakly:
/// a view that React Native drops leaves it by itself.
enum QvpRnRegistry {
    private final class Weak { weak var view: QvpRnPageView?; init(_ v: QvpRnPageView) { view = v } }
    private static let lock = NSLock()
    private static var views: [Int: Weak] = [:]
    static func register(_ v: QvpRnPageView) {
        guard let tag = v.reactTag?.intValue, tag > 0 else { return }
        lock.lock(); views[tag] = Weak(v); lock.unlock()
    }
    static func unregister(_ v: QvpRnPageView) {
        lock.lock(); views = views.filter { $0.value.view != nil && $0.value.view !== v }; lock.unlock()
    }
    static func get(_ tag: Int) -> QvpRnPageView? {
        lock.lock(); defer { lock.unlock() }
        return views[tag]?.view
    }
}
