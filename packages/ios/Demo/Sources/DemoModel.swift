// Mushaf Vector Reader — iOS demo model. Every visual state is engine state; the view only paints.
// A port of packages/android/demo MainActivity.kt over QvpKit.
import SwiftUI
import UIKit
import QvpKit

struct ThemeSpec { let ink: UInt32; let paper: UInt32; let bg: UInt32 }

@MainActor
final class DemoModel: ObservableObject {
    static let pages: [Int] = Array(1...21) + Array(440...445) + [582, 604]
    static let themes: [String: ThemeSpec] = [
        "light": ThemeSpec(ink: 0x231f20ff, paper: 0xfffdf7ff, bg: 0xf6f1e7ff),
        "sepia": ThemeSpec(ink: 0x3b2a14ff, paper: 0xf3e7cfff, bg: 0xe9dcc3ff),
        "dark": ThemeSpec(ink: 0xe8e4dcff, paper: 0x1e2126ff, bg: 0x15171bff),
    ]

    let view = QvpPageView()
    private(set) var page: QvpPage?
    private(set) var atlas: QvpAtlas?

    @Published var pageNo = 1
    @Published var pageField = "1"
    @Published var gotoField = ""
    @Published var searchField = "" { didSet { if searchField != oldValue { runSearch() } } }
    @Published var results: [QvpMatch] = []
    @Published var searchedEmpty = false
    @Published var title = ""
    @Published var subtitle = ""
    @Published var hlModeIdx = 0 { didSet { if hlModeIdx != oldValue, selWordIdx >= 0 { let i = selWordIdx; selWordIdx = -1; selectWord(i) } } }
    @Published var hlMs: Double = 250
    @Published var playing = false { didSet { if playing != oldValue { playing ? startPlay() : stopPlay() } } }
    @Published var markColours = false { didSet { if markColours != oldValue { toggleTajweed(markColours) } } }
    @Published var hideMarks = false { didSet { if hideMarks != oldValue { toggleHideMarks(hideMarks) } } }
    @Published var goldMarkers = false { didSet { if goldMarkers != oldValue { toggleMarkers(goldMarkers) } } }
    @Published var theme = "light" { didSet { if theme != oldValue { applyTheme() } } }
    @Published var maskModeIdx = 0
    @Published var revealOn = false { didSet { if revealOn != oldValue { toggleReveal(revealOn) } } }
    @Published var revealPos: Double = 0 { didSet { if revealOn, let p = page { p.revealGoto(Int(revealPos)); revealVal = "\(Int(revealPos) + 1)/\(p.revealSteps())"; view.setNeedsDisplay() } } }
    @Published var revealMax: Double = 1
    @Published var revealVal = ""
    @Published var lineSpacing: Double = 100 { didSet { if lineSpacing != oldValue { view.lineGap = 0; fillHeight = false; view.lineSpacing = Float(lineSpacing / 100); view.relayout(); view.resetView(); hud() } } }
    @Published var padTop: Double = 12 { didSet { if padTop != oldValue { view.padTop = CGFloat(padTop); view.relayout(); view.resetView(); hud() } } }
    @Published var padBottom: Double = 12 { didSet { if padBottom != oldValue { view.padBottom = CGFloat(padBottom); view.relayout(); view.resetView(); hud() } } }
    @Published var fillHeight = true { didSet { if fillHeight != oldValue { if fillHeight { controlsShown = false }; view.fillHeight = fillHeight; view.relayout(); view.resetView(); hud() } } }
    /// While `fillHeight` is on the bars are hidden; this brings them back until the reader hides them again.
    @Published var controlsShown = false
    @Published var meta = ""
    @Published var hudText = ""
    @Published var toast: String?
    /// Page-flip animation: a snapshot of the page that slides away while the new page is already drawn beneath.
    @Published var flipImage: UIImage?
    @Published var flipOffset: CGFloat = 0

    private var loadMs = 0.0, pageBytes = 0
    private var selWordIdx = -1, selAyah: (Int, Int)?
    private var hlSel = 0, hlAyah = 0, hlSearch = 0, hlPlay = 0
    private var tajweed = 0, hideMarksH = 0, markersH = 0
    private var playIdx = 0
    private var playTimer: Timer?, hudTimer: Timer?, toastTimer: Timer?
    private var hlMode: HighlightMode { [.both, .band, .ink][hlModeIdx] }
    var themeSpec: ThemeSpec { Self.themes[theme]! }
    var bgColor: Color { Color(uiColor: UIColor(rgba: themeSpec.bg)) }

    init() {
        if let u = Bundle.main.url(forResource: "atlas", withExtension: "qva", subdirectory: "pages"), let d = try? Data(contentsOf: u) { atlas = try? QvpAtlas(bytes: d) }
        view.padTop = 12; view.padBottom = 12; view.padSide = 8; view.fillHeight = true
        view.onSwipe = { [weak self] dir in self?.flip(dir) }   // mushaf order: finger right → next page
        view.isAccessibilityElement = true; view.accessibilityIdentifier = "qvpPage"; view.accessibilityLabel = "mushaf page"
        view.selectionEnabled = false                       // a reader: tap highlights, no text selection
        view.onWordTap = { [weak self] w, _ in self?.selectWord(w.idx) }
        view.onDecoTap = { [weak self] d, _ in if d.ayah != 0 { self?.selectAyah(d.sura, d.ayah) } }
        view.onEmptyTap = { [weak self] in self?.selectWord(-1) }
        applyTheme()
        loadPage(Self.pages[0])
        hudTimer = Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { [weak self] _ in Task { @MainActor in self?.hud() } }
        applyLaunchArguments()
    }
    /// Scripted states for screenshots / QA, e.g. `-qvpPage 582 -qvpSearch الله -qvpWord 5 -qvpTheme dark -qvpMarks 1`.
    /// `-qvpFill 0` turns fill-screen off; `-qvpControls 1` starts with the bars visible in fill-screen mode.
    private func applyLaunchArguments() {
        let d = UserDefaults.standard
        if let t = d.string(forKey: "qvpTheme"), Self.themes[t] != nil { theme = t }
        if let n = d.string(forKey: "qvpPage").flatMap(Int.init) { loadPage(n) }
        if let g = d.string(forKey: "qvpGoto") { gotoField = g; goto() }
        if let s = d.string(forKey: "qvpSearch") { searchField = s }
        if let a = d.string(forKey: "qvpAyah") { let g = a.split(separator: ":").compactMap { Int($0) }; if g.count == 2 { selectAyah(g[0], g[1]) } }
        if let w = d.string(forKey: "qvpWord").flatMap(Int.init) { selectWord(w) }
        if d.bool(forKey: "qvpMarks") { markColours = true }
        if d.bool(forKey: "qvpGold") { goldMarkers = true }
        if d.bool(forKey: "qvpMask") { maskModeIdx = 1; maskAyah() }
        if d.object(forKey: "qvpFill") != nil { fillHeight = d.bool(forKey: "qvpFill") }
        if d.bool(forKey: "qvpControls") { controlsShown = true }
    }

    // ── navigation ──
    /// Slide the current page out (+1 = to the right, the next page in a right-to-left book) and load the neighbour.
    func flip(_ dir: Int) {
        guard let i = Self.pages.firstIndex(of: pageNo), Self.pages.indices.contains(i + dir) else { showToast(dir > 0 ? "Last bundled page" : "First bundled page"); return }
        let r = UIGraphicsImageRenderer(bounds: view.bounds)
        flipImage = r.image { _ in view.drawHierarchy(in: view.bounds, afterScreenUpdates: false) }
        flipOffset = 0
        loadPage(Self.pages[i + dir])
        withAnimation(.easeInOut(duration: 0.28)) { flipOffset = CGFloat(dir) * view.bounds.width }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { [weak self] in self?.flipImage = nil; self?.flipOffset = 0 }
    }
    func stepPage(_ dir: Int) { if let i = Self.pages.firstIndex(of: pageNo), Self.pages.indices.contains(i + dir) { loadPage(Self.pages[i + dir]) } }
    func gotoPageField() { if let n = Int(pageField.trimmingCharacters(in: .whitespaces)) { loadPage(n) } }
    func goto() {
        guard let a = atlas else { return }
        let s = gotoField.trimmingCharacters(in: .whitespaces)
        if let m = s.range(of: "^(\\d+):(\\d+)", options: .regularExpression) {
            let g = s[m].split(separator: ":").compactMap { Int($0) }
            if g.count == 2, let pg = a.pageOf(g[0], g[1]) { loadPage(pg); selectAyah(g[0], g[1]) }
            return
        }
        if let m = s.range(of: "^juz\\s*(\\d+)", options: [.regularExpression, .caseInsensitive]) {
            if let n = Int(s[m].filter { $0.isNumber }), let j = a.juz(n) { loadPage(j.page) }
            return
        }
        if let su = a.findSurah(s).first { loadPage(su.page) }
    }
    func loadPage(_ n: Int) {
        let target = Self.pages.contains(n) ? n : Self.pages.min { abs($0 - n) < abs($1 - n) }!
        let name = String(format: "%03d", target)
        guard let u = Bundle.main.url(forResource: name, withExtension: "qvp", subdirectory: "pages"), let bytes = try? Data(contentsOf: u) else { showToast("no page \(name) in the app bundle — run Demo/sync-pages.sh"); return }
        let t0 = CACurrentMediaTime()
        guard let p = try? QvpPage(bytes: bytes) else { showToast("qvp_page_load failed for \(name)"); return }
        _ = p.buildPaths()
        loadMs = (CACurrentMediaTime() - t0) * 1000; pageBytes = bytes.count
        if let wu = Bundle.main.url(forResource: "\(name).words", withExtension: "json", subdirectory: "pages"), let wd = try? Data(contentsOf: wu) { _ = p.attachWords(wd) }
        stopPlay(); playing = false; page?.close(); page = p; pageNo = target
        pageField = "\(target)"
        selWordIdx = -1; selAyah = nil; hlSel = 0; hlAyah = 0; hlSearch = 0; hlPlay = 0
        tajweed = 0; hideMarksH = 0; markersH = 0; markColours = false; hideMarks = false; goldMarkers = false; revealOn = false
        p.setDefaultInk(themeSpec.ink)
        view.page = p
        announce(); showMeta(); showTitle(); runSearch(); hud()
    }
    private func showTitle() {
        guard let p = page else { return }
        let first = p.words.first
        let su = first.flatMap { w in atlas?.surah(w.sura) }
        title = su.map { $0.latin.isEmpty ? $0.arabic : $0.latin } ?? p.surahs().first.map { $0.latin } ?? "Page \(pageNo)"
        let j = first.flatMap { atlas?.juzAt($0.sura, $0.ayah) }
        subtitle = "Page \(pageNo)" + (j.map { " · Juz \($0)" } ?? "")
    }
    private func showMeta() {
        guard let p = page else { return }
        let su = p.surahs().map { "\($0.number)\($0.latin.isEmpty ? "" : " " + $0.latin)\($0.hasBanner ? " (banner)" : "")" }.joined(separator: ", ")
        let dv = p.divisions().map { "\($0.kind) \($0.n) at \($0.sura):\($0.ayah)" }.joined(separator: ", ")
        var s = "surahs: \(su)"
        if !dv.isEmpty { s += "\nstarts here: \(dv)" }
        if let a = atlas, let w = p.words.first, let j = a.juzAt(w.sura, w.ayah) { s += "\njuz \(j) · pages \(a.pagesOfJuz(j).map { "\($0.0)–\($0.1)" } ?? "?")" }
        s += "\nayahs: " + p.ayahKeys().map { "\($0.0):\($0.1)" }.joined(separator: " ")
        meta = s
    }

    // ── tap highlights ──
    func selectWord(_ i: Int) {
        guard let p = page else { return }
        selAyah = nil; if hlAyah != 0 { p.unhighlight(hlAyah); hlAyah = 0 }
        if i < 0 || i == selWordIdx { selWordIdx = -1; if hlSel != 0 { p.unhighlight(hlSel); hlSel = 0 } }
        else {
            selWordIdx = i
            let st = QvpHighlightStyle(mode: hlMode, ink: 0x1a73e8ff, band: QvpColor.withAlpha(0x1a73e8ff, 0.18), radius: 1.5, transitionMs: Int(hlMs), layer: QvpLayer.SELECTION)
            if hlSel != 0 { p.rehighlight(hlSel, Target.word(i)) } else { hlSel = p.highlight(Target.word(i), st) }
        }
        announce(); view.setNeedsDisplay()
    }
    func selectAyah(_ s: Int, _ a: Int) {
        guard let p = page else { return }
        guard !p.resolve(Target.ayah(s, a)).isEmpty else { showToast("Ayah \(s):\(a) is not on this page"); return }
        if hlSel != 0 { p.unhighlight(hlSel); hlSel = 0 }; selWordIdx = -1
        selAyah = (s, a)
        let st = QvpHighlightStyle(mode: hlMode, ink: 0x0a7d32ff, band: QvpColor.withAlpha(0x0a7d32ff, 0.14), radius: 1.5, transitionMs: Int(hlMs), layer: QvpLayer.SELECTION)
        if hlAyah != 0 { p.rehighlight(hlAyah, Target.ayah(s, a)) } else { hlAyah = p.highlight(Target.ayah(s, a), st) }
        announce(); view.setNeedsDisplay()
    }
    /// VoiceOver value for the page: the highlighted word or ayah.
    private func announce() {
        guard let p = page else { return }
        if selWordIdx >= 0 { view.accessibilityValue = "word \(p.wid(selWordIdx)) · \(p.wordLabel(selWordIdx))" }
        else if let (s, a) = selAyah { view.accessibilityValue = "ayah \(s):\(a)" }
        else { view.accessibilityValue = nil }
    }

    // ── search ──
    private func runSearch() {
        guard let p = page else { return }
        results = []; searchedEmpty = false
        if hlSearch != 0 { p.unhighlight(hlSearch); hlSearch = 0 }
        let q = searchField.trimmingCharacters(in: .whitespaces)
        if q.isEmpty { view.setNeedsDisplay(); return }
        let m = p.search(q)
        if !m.isEmpty { hlSearch = p.highlight(Target.words(m.map { $0.word }), QvpHighlightStyle(mode: .both, ink: 0xc62828ff, band: QvpColor.withAlpha(0xc62828ff, 0.12), height: .ink, padY: 1, radius: 1, transitionMs: Int(hlMs))) }
        results = Array(m.prefix(8)); searchedEmpty = m.isEmpty
        view.setNeedsDisplay()
    }

    // ── styling ──
    private func toggleTajweed(_ on: Bool) {
        guard let p = page else { return }
        if tajweed != 0 { p.unstyle(tajweed); tajweed = 0 }
        if on { tajweed = p.theme(QvpTheme(diacritics: 0x1a73e8ff, dots: 0xc62828ff, waqf: 0x0a7d32ff, sifr: 0xef6c00ff, transitionMs: 200)) }
        view.setNeedsDisplay()
    }
    private func toggleHideMarks(_ on: Bool) {
        guard let p = page else { return }
        if hideMarksH != 0 { p.unstyle(hideMarksH); hideMarksH = 0 }
        if on { hideMarksH = p.hide(Selector.kind(QvpKind.MARK)) }
        view.setNeedsDisplay()
    }
    private func toggleMarkers(_ on: Bool) {
        guard let p = page else { return }
        if markersH != 0 { p.unstyle(markersH); markersH = 0 }
        if on { markersH = p.style(Selector.deco(QvpDeco.AYAH_MARKER), 0xb8860bff, transitionMs: 300, layer: QvpLayer.THEME + 1) }
        view.setNeedsDisplay()
    }
    private func applyTheme() {
        view.paperColor = UIColor(rgba: themeSpec.paper)
        page?.setDefaultInk(themeSpec.ink); view.setNeedsDisplay()
    }
    func clearAll() {
        guard let p = page else { return }
        p.clearStyles(); p.clearHighlights(); p.unmask(); p.revealStop()
        hlSel = 0; hlAyah = 0; hlSearch = 0; hlPlay = 0; tajweed = 0; hideMarksH = 0; markersH = 0; selWordIdx = -1; selAyah = nil
        markColours = false; hideMarks = false; goldMarkers = false; revealOn = false; playing = false
        searchField = ""; stopPlay(); announce(); view.setNeedsDisplay()
    }

    // ── memorisation ──
    func maskAyah() {
        guard let p = page else { return }
        let t: Target = selAyah.map { Target.ayah($0.0, $0.1) } ?? (selWordIdx >= 0 ? Target.ayah(p.words[selWordIdx].sura, p.words[selWordIdx].ayah) : Target.page())
        p.maskOptions(blockColor: themeSpec.bg)
        p.mask(t, maskModeIdx == 1 ? .block : .hide); view.setNeedsDisplay()
    }
    func revealNext() { page?.revealNext(1); view.setNeedsDisplay() }
    func hideBack() { page?.hideBack(1); view.setNeedsDisplay() }
    func unmask() { page?.unmask(); view.setNeedsDisplay() }
    private func toggleReveal(_ on: Bool) {
        guard let p = page else { return }
        if on {
            let steps = p.revealStart(lit: 2, grey: theme == "dark" ? 0x4a4f57ff : 0xc9c4b8ff, ink: themeSpec.ink, transitionMs: 150)
            revealMax = Double(max(steps - 1, 1)); revealPos = 0; p.revealGoto(-1); revealVal = "0/\(steps)"
        } else { p.revealStop(); revealVal = "" }
        view.setNeedsDisplay()
    }

    // ── layout ──
    func leadingToFill() {
        guard let p = page else { return }
        fillHeight = false; lineSpacing = 100; view.lineSpacing = 1
        view.lineGap = QvpEngine.gapToFill(pageW: p.width, pageH: p.height, lines: p.nLines, viewW: Float(view.bounds.width - 2 * view.padSide), viewH: Float(view.bounds.height - view.padTop - view.padBottom))
        view.relayout(); view.resetView(); hud()
    }

    // ── follow words ──
    private func startPlay() {
        guard let p = page else { return }
        playIdx = selWordIdx >= 0 ? selWordIdx : 0
        let st = QvpHighlightStyle(mode: hlMode, ink: 0xd81b60ff, band: QvpColor.withAlpha(0xd81b60ff, 0.14), radius: 1.5, transitionMs: Int(hlMs))
        hlPlay = p.highlight(Target.word(playIdx), st)
        playTimer?.invalidate()
        playTimer = Timer.scheduledTimer(withTimeInterval: 0.32, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self, self.playing, let pg = self.page else { return }
                self.playIdx += 1
                if self.playIdx >= pg.nWords { self.stepPage(+1); return }
                pg.rehighlight(self.hlPlay, Target.word(self.playIdx)); self.view.setNeedsDisplay()
            }
        }
        view.setNeedsDisplay()
    }
    private func stopPlay() {
        playTimer?.invalidate(); playTimer = nil
        if let p = page, hlPlay != 0 { p.unhighlight(hlPlay); hlPlay = 0 }
        view.setNeedsDisplay()
    }

    // ── HUD ──
    func hud() {
        guard let p = page else { return }
        let l = p.currentLayout
        let layout = view.fillHeight ? "fill height" : view.lineGap > 0 ? String(format: "gap +%.1f u", view.lineGap) : String(format: "spacing ×%.2f", view.lineSpacing)
        hudText = "engine v\(QvpEngine.version()) · Swift over qvp.h\(atlas != nil ? " · atlas" : "")\n"
            + String(format: "page %03d      %d KB, load %.2f ms\n", pageNo, pageBytes / 1024, loadMs)
            + "content       \(p.nWords) words · \(p.nPaths) paths · \(p.nLines) lines\n"
            + String(format: "base layer    %d paths in %.2f ms (cached)\n", view.lastBasePaths, view.lastBaseMs)
            + String(format: "overlay       %d styled + %d bands in %.2f ms\n", view.lastOverlayPaths, view.lastBands, view.lastOverlayMs)
            + String(format: "hit-test      %.1f µs · %d handles · %d highlights\n", view.lastHitUs, p.styleHandles().count, p.highlightHandles().count)
            + String(format: "layout        %@ · pitch %.1f u", layout, l?.pitch ?? 0)
    }
    func showToast(_ s: String) {
        toast = s
        toastTimer?.invalidate()
        toastTimer = Timer.scheduledTimer(withTimeInterval: 2.5, repeats: false) { [weak self] _ in Task { @MainActor in self?.toast = nil } }
    }
}

extension UIColor {
    convenience init(rgba: UInt32) {
        let c = QvpColor.components(rgba)
        self.init(red: c.r, green: c.g, blue: c.b, alpha: c.a)
    }
}
