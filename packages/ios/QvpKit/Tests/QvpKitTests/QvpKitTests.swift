// Same assertions as packages/flutter/qvp_flutter/test/qvp_flutter_test.dart, over the Swift wrapper.
// Data: <repo>/dist/pages/042.qvp, 042.words.json, atlas.qva
// (`cargo run -p qvp-convert --release -- batch pages dist/pages`).
// Run: `swift test` (macOS slice) or `xcodebuild test -scheme QvpKit -destination 'platform=iOS Simulator,name=iPhone 17'`.
import XCTest
@testable import QvpKit

final class QvpKitTests: XCTestCase {
    static var repo: URL {
        var d = URL(fileURLWithPath: #filePath)
        for _ in 0..<8 {
            d.deleteLastPathComponent()
            if FileManager.default.fileExists(atPath: d.appendingPathComponent("crates/qvp-ffi/include/qvp.h").path) { return d }
        }
        return URL(fileURLWithPath: ProcessInfo.processInfo.environment["QVP_REPO"] ?? ".")
    }
    static let pages = repo.appendingPathComponent("dist/pages")
    static var page: QvpPage!

    override class func setUp() {
        super.setUp()
        let url = pages.appendingPathComponent("042.qvp")
        guard let bytes = try? Data(contentsOf: url) else { XCTFail("page not found at \(url.path) — run the converter batch first"); return }
        page = try? QvpPage(bytes: bytes)
        XCTAssertNotNil(page)
    }
    override class func tearDown() { page?.close(); page = nil; super.tearDown() }
    var page: QvpPage { Self.page }

    func testEngineVersionNamesArabicTools() {
        XCTAssertGreaterThan(QvpEngine.version(), 0)
        XCTAssertEqual(QvpEngine.engineName(), "qvp")
        XCTAssertEqual(QvpEngine.kindName(QvpKind.MARK), "mark")
        XCTAssertEqual(QvpEngine.markName(1), "fathah")
        XCTAssertEqual(QvpEngine.markName(7), QvpEngine.markName(markId("shaddah")))
        XCTAssertEqual(QvpEngine.markFromName("shaddah"), 7)
        XCTAssertFalse(QvpEngine.familyName(QvpFamily.DIACRITIC).isEmpty)
        XCTAssertFalse(QvpEngine.categoryName(QvpCategory.HARAKAH).isEmpty)
        XCTAssertEqual(QvpEngine.strip("بِسْمِ"), "بسم")
        XCTAssertFalse(QvpEngine.normalize("ٱللَّهِ").isEmpty)
        XCTAssertFalse(QvpEngine.looseKey("الله").isEmpty)
        XCTAssertEqual(QvpColor.parse("#1a73e8"), 0x1a73e8ff)
        XCTAssertEqual(QvpColor.parse("#d6a326", alpha: 0.3), 0xd6a3264d)
        XCTAssertEqual(QvpColor.rgba(QvpColor.cgColor(0x1a73e8cc)), 0x1a73e8cc)
        XCTAssertEqual(QvpColor.withAlpha(0x1a73e8ff, 0.18), 0x1a73e82e)
    }

    func testPage042Counts() {
        XCTAssertEqual(page.pageNo, 42)
        XCTAssertEqual(page.nWords, 147)
        XCTAssertEqual(page.nPaths, 1061)
        XCTAssertEqual(page.nLines, 15)
        XCTAssertEqual(page.width, 345, accuracy: 0.01)
        XCTAssertEqual(page.height, 550, accuracy: 0.01)
        XCTAssertEqual(page.words.count, 147)
        XCTAssertEqual(page.lines.count, 15)
        XCTAssertEqual(page.ayahs.count, page.nAyahs)
        XCTAssertEqual(page.decos.count, page.nDecos)
        XCTAssertEqual(page.table.count, 1061 * 8)
        for i in 0..<page.nPaths {
            XCTAssertLessThanOrEqual(page.pathOpStart(i) + page.pathOpCount(i), page.ops.count)
            XCTAssertLessThanOrEqual(page.pathPtStart(i) + page.pathPtCount(i), page.pts.count)
            XCTAssertLessThan(page.pathLine(i), page.nLines)
        }
        XCTAssertEqual(page.buildPaths().count, 1061)
        XCTAssertFalse(page.buildPaths()[0].isEmpty)
        let w0 = page.words[0]
        XCTAssertFalse(w0.text.isEmpty)
        XCTAssertGreaterThan(w0.nPaths, 0)
        XCTAssertEqual(page.findWord(w0.surah, w0.ayah, w0.word), 0)
        XCTAssertEqual(page.wordKey(0), w0.wordKey)
        XCTAssertGreaterThan(page.naturalPitch, 0)
        XCTAssertTrue(page.surahs().map { $0.number }.contains(2))
        XCTAssertTrue(page.ayahKeys().contains { $0 == (2, 255) })
        XCTAssertFalse(page.wordLabel(0).isEmpty)
        XCTAssertFalse(page.ayahLabel(page.words[0].ayahIdx).isEmpty)
        XCTAssertFalse(page.ayahMarks().isEmpty)
        XCTAssertEqual(page.lineBands().count, 15)
        XCTAssertEqual(page.hitAreas().count, 147)
        XCTAssertTrue(page.text("page").contains(w0.text))
    }

    func testSearchAllah7() {
        let m = page.search("الله")
        XCTAssertEqual(m.count, 7)
        XCTAssertFalse(m[0].text.isEmpty)
        XCTAssertEqual(m[0].wordKey, page.wordKey(m[0].word))
    }

    func testResolve2_255() {
        let ws = page.targetWords("2:255")
        XCTAssertEqual(ws.count, 50)
        XCTAssertEqual(page.targetWords(Target.ayah(2, 255)), ws)
        XCTAssertEqual(page.targetWords(ws), ws)
        XCTAssertEqual(page.targetWords("2:255:1"), [ws[0]])
        XCTAssertEqual(page.citation(ws), "2:255")
        XCTAssertEqual(page.ayahWordCount(2, 255).count, 50)
        XCTAssertFalse(page.text("2:255").isEmpty)
    }

    func testAttachWordsSidecar() throws {
        let url = Self.pages.appendingPathComponent("042.words.json")
        try XCTSkipUnless(FileManager.default.fileExists(atPath: url.path), "no sidecar")
        XCTAssertGreaterThanOrEqual(page.attachWords(try Data(contentsOf: url)), 0)
        XCTAssertEqual(page.attachWords("not json"), -1)
        XCTAssertTrue(page.hasForm(.rasmImlai))
        XCTAssertFalse(page.wordForm(0, .rasmImlai).isEmpty)
        XCTAssertFalse(page.wordForm(0, .search).isEmpty)
    }

    func testHitTestEx() {
        let w = page.words[0]
        let cx = (w.x0 + w.x1) / 2, cy = (w.y0 + w.y1) / 2
        let h = page.hitTest(cx, cy)
        XCTAssertNotNil(h)
        XCTAssertEqual(h?.word, 0)
        XCTAssertEqual(h?.distance, 0)
        XCTAssertEqual(h?.line, w.lineIdx)
        XCTAssertEqual(page.hitTestExact(cx, cy)?.word, 0)
        var inside: QvpHit?
        var y = w.y0
        outer: while y <= w.y1 {
            var x = w.x0
            while x <= w.x1 { if let e = page.hitTest(x, y), e.word == 0, e.isExact { inside = e; break outer }; x += 0.5 }
            y += 0.5
        }
        XCTAssertNotNil(inside, "no point of word 0 is inside its ink")
        XCTAssertGreaterThanOrEqual(inside!.path, 0)
        XCTAssertEqual(page.pathWord(inside!.path), 0)
        XCTAssertNil(page.hitTest(-500, -500, QvpHitOptions(maxDistance: 6)))
    }

    func testLayoutFillHeight() {
        let l = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true))
        XCTAssertEqual(l.scale, 2.0, accuracy: 1e-5)
        XCTAssertEqual(l.lineDy.count, 15)
        XCTAssertEqual(l.slotTop.count, 15)
        XCTAssertTrue(page.currentLayout === l)
        let w = page.words[0]
        let vx = l.ox + (w.x0 + w.x1) / 2 * l.scale
        let vy = l.oy + ((w.y0 + w.y1) / 2 + l.lineDy[w.lineIdx]) * l.scale
        let h = page.hitTestView(vx, vy, QvpHitOptions(maxDistance: 6))
        XCTAssertEqual(h?.word, 0)
        let box = page.wordBoundsView(0)
        XCTAssertNotNil(box)
        XCTAssertLessThan(box!.x0, vx)
        XCTAssertGreaterThan(box!.x1, vx)
        _ = QvpEngine.gapToFill(pageW: page.width, pageH: page.height, lines: page.nLines, viewW: 600, viewH: 1000)
        let wf = QvpEngine.wastedFraction(pageW: page.width, pageH: page.height, viewW: 600, viewH: 1000)
        XCTAssertTrue(wf >= 0 && wf <= 1)
    }

    func testStyles() {
        // XCTest runs alphabetically over one shared page: drop what other tests left behind
        page.clearHighlights(); page.unmask(); page.revealStop(); page.clearSelection(); _ = page.tick(1e9)
        page.clearStyles()
        XCTAssertTrue(page.styledPaths().isEmpty)
        let h = page.style(Selector.wordMark(0, 1), 0xef6c00ff)
        XCTAssertGreaterThan(h, 0)
        let s = page.styledPaths()
        XCTAssertEqual(s.count, 1)
        XCTAssertEqual(s[0].color, 0xef6c00ff)
        XCTAssertEqual(page.pathWord(s[0].path), 0)
        XCTAssertEqual(page.pathNthMark(s[0].path), 1)
        XCTAssertEqual(page.colorOf(s[0].path), 0xef6c00ff)
        XCTAssertEqual(page.colors()[s[0].path], 0xef6c00ff)
        XCTAssertTrue(page.styleHandles().contains(h))
        XCTAssertGreaterThan(page.removeStyle(h), 0)
        XCTAssertTrue(page.styledPaths().isEmpty)
        let th = page.theme(QvpTheme(diacritics: 0x1a73e8ff, dots: 0xc62828ff, marks: ["shaddah": 0x0a7d32ff]))
        XCTAssertFalse(page.styledPaths().isEmpty)
        let hh = page.hide(Selector.kind(QvpKind.MARK))
        XCTAssertTrue(page.styledPaths().contains { $0.color & 0xff == 0 })
        page.removeStyle(hh); page.removeStyle(th)
        let ht = page.styleTarget("2:255", 0x0a7d32ff, layer: QvpLayer.TOP)
        XCTAssertGreaterThan(page.styledPaths().count, 50)
        page.removeStyle(ht)
        XCTAssertTrue(page.styledPaths().isEmpty)
        page.setDefaultColor(0x3b2a14ff)
        XCTAssertEqual(page.defaultInk, 0x3b2a14ff)
        XCTAssertEqual(page.colors()[0], 0x3b2a14ff)
        page.setDefaultColor(0x231f20ff)
    }

    func testHighlightTick() {
        page.clearHighlights()
        page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true))
        let h = page.highlight("2:255", QvpHighlightStyle(mode: .both, transitionMs: 200))
        XCTAssertGreaterThan(h, 0)
        XCTAssertTrue(page.tick(100))
        let boxes = page.highlightBoxesView()
        XCTAssertEqual(boxes.count, 6)
        XCTAssertTrue(boxes.allSatisfy { $0.id == h })
        XCTAssertEqual(page.highlightHandles(), [h])
        XCTAssertEqual(page.highlightWords(h).count, 50)
        XCTAssertFalse(page.tick(1000))
        XCTAssertTrue(page.moveHighlight(h, Target.word(0)))
        XCTAssertTrue(page.restyleHighlight(h, QvpHighlightStyle(mode: .band)))
        XCTAssertTrue(page.removeHighlight(h))
        _ = page.tick(5000)
        page.clearHighlights()
        XCTAssertTrue(page.highlightHandles().isEmpty)
        XCTAssertEqual(page.wordBands(page.targetWords("2:255")).count, 6)
    }

    func testMaskReveal() {
        page.mask("2:255")
        XCTAssertEqual(page.maskHidden().count, 50)
        XCTAssertEqual(page.maskWords().count, 50)
        XCTAssertEqual(page.unmaskNext(1), 1)
        XCTAssertEqual(page.maskHidden().count, 49)
        XCTAssertEqual(page.maskBack(1), 1)
        XCTAssertEqual(page.maskHidden().count, 50)
        page.unmask()
        XCTAssertTrue(page.maskHidden().isEmpty)
        page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100))
        page.mask("2:255", .block)
        XCTAssertFalse(page.maskBoxesView().isEmpty)
        page.unmask()
        let steps = page.revealStart(lit: 2)
        XCTAssertGreaterThan(steps, 0)
        XCTAssertEqual(page.revealStepCount(), steps)
        XCTAssertEqual(page.revealPosition(), -1)
        XCTAssertTrue(page.revealGoto(0))
        XCTAssertEqual(page.revealPosition(), 0)
        XCTAssertGreaterThanOrEqual(page.revealStepOf(0), 0)
        page.revealStop()
        XCTAssertNil(page.revealPosition())
    }

    func testSelection() {
        page.select(0, 3)
        XCTAssertEqual(page.selection(), [0, 1, 2, 3])
        XCTAssertTrue(page.selectionText(.rasmUthmani, citation: true).contains(":"))
        page.clearSelection()
        XCTAssertTrue(page.selection().isEmpty)
    }

    func testCropSvg() {
        let svg = page.cropSvg("2:255:1", background: 0xfffdf7ff)
        XCTAssertNotNil(svg)
        XCTAssertTrue(svg!.hasPrefix("<svg"))
        let box = page.cropBounds("2:255")
        XCTAssertEqual(box?.nWords, 50)
    }

    func testAtlas() throws {
        let url = Self.pages.appendingPathComponent("atlas.qva")
        try XCTSkipUnless(FileManager.default.fileExists(atPath: url.path), "no atlas")
        let atlas = try QvpAtlas(bytes: try Data(contentsOf: url))
        XCTAssertEqual(atlas.pageCount(), 604)
        XCTAssertEqual(atlas.pageOf(2, 255), 42)
        XCTAssertTrue(atlas.pagesOfJuz(30)! == (582, 604))
        let cow = atlas.searchSurahs("cow")
        XCTAssertFalse(cow.isEmpty)
        XCTAssertEqual(cow.first?.n, 2)
        XCTAssertFalse(atlas.surah(36)!.latin.isEmpty)
        XCTAssertEqual(atlas.surahs().count, 114)
        XCTAssertEqual(atlas.pageOfSurah(36), atlas.surah(36)!.page)
        XCTAssertEqual(atlas.juz(30)!.page, 582)
        XCTAssertEqual(atlas.juzOf(2, 255), 3)
        XCTAssertEqual(atlas.pageRange(42)!.first.0, 2)
        atlas.close()
    }

    /// QvpCanvasController mirrors QvpPageView's fit / transform / tap maths over the same engine layout.
    @MainActor func testCanvasController() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let bytes = try Data(contentsOf: Self.pages.appendingPathComponent("042.qvp"))
        let p = try QvpPage(bytes: bytes)
        defer { p.close() }
        let c = QvpCanvasController()
        c.page = p
        c.setBounds(CGSize(width: 690, height: 1100))
        let l = try XCTUnwrap(p.currentLayout)
        // fit-and-centre, exactly QvpPageView.resetView()
        let expScale: CGFloat = CGFloat(l.contentH) > 1100 ? 1100 / CGFloat(l.contentH) : 1
        XCTAssertEqual(c.viewScale, expScale, accuracy: 1e-5)
        XCTAssertEqual(c.viewOx, max((690 - CGFloat(l.contentW) * expScale) / 2, 0), accuracy: 1e-3)
        XCTAssertEqual(c.viewOy, max((1100 - CGFloat(l.contentH) * expScale) / 2, 0), accuracy: 1e-3)
        XCTAssertFalse(c.isZoomed)
        // lineTransform composes engine layout + view transform
        let w = p.words[0]
        let t = c.lineTransform(w.lineIdx)
        let pt = CGPoint(x: CGFloat((w.x0 + w.x1) / 2), y: CGFloat((w.y0 + w.y1) / 2)).applying(t)
        // a tap at that view point resolves to the same word through the controller
        var tapped: Int? = nil
        c.onWordTap = { word, _ in tapped = word.idx }
        c.tap(pt)
        XCTAssertEqual(tapped, 0)
        // double-tap carries the hit under it
        var doubleTapped: Int? = nil
        c.onDoubleTap = { hit in doubleTapped = hit?.word }
        c.doubleTap(pt)
        XCTAssertEqual(doubleTapped, 0)
        // pinch past the fitted scale flips isZoomed; resetView clears it
        c.pinch(2.0, at: CGPoint(x: 345, y: 550))
        XCTAssertTrue(c.isZoomed)
        c.resetView()
        XCTAssertFalse(c.isZoomed)
        // line spacing only opens up: the engine clamps values below 1 to the printed pitch
        let printed = try XCTUnwrap(p.currentLayout).pitch
        c.lineSpacing = 0.5
        XCTAssertEqual(try XCTUnwrap(p.currentLayout).pitch, printed)
        c.lineSpacing = 1.5
        XCTAssertGreaterThan(try XCTUnwrap(p.currentLayout).pitch, printed)
    }

    /// zoomSpringsBack: a released pinch eases back to the fitted transform; without it the zoom stays.
    @MainActor func testZoomSpringsBack() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        // the easing itself: starts where the pinch left off, arrives exactly, ease-out past halfway at half time
        let s = QvpZoomSpring(from: (2, -100, -50), to: (1, 0, 10), start: 10)
        XCTAssertEqual(s.value(at: 10).transform.scale, 2)
        XCTAssertFalse(s.value(at: 10).done)
        XCTAssertLessThan(s.value(at: 10 + QvpZoomSpring.duration / 2).transform.scale, 1.5)
        let end = s.value(at: 10 + QvpZoomSpring.duration)
        XCTAssertTrue(end.done)
        XCTAssertEqual(end.transform.scale, 1, accuracy: 1e-9)
        XCTAssertEqual(end.transform.oy, 10, accuracy: 1e-9)

        let p = try QvpPage(bytes: try Data(contentsOf: Self.pages.appendingPathComponent("042.qvp")))
        defer { p.close() }
        let c = QvpCanvasController()
        c.page = p
        c.setBounds(CGSize(width: 690, height: 1100))
        let fitted = (c.viewScale, c.viewOx, c.viewOy)
        c.pinch(2.0, at: CGPoint(x: 345, y: 550)); c.pinchEnded()
        XCTAssertTrue(c.isZoomed, "without zoomSpringsBack the zoom stays")
        c.resetView()
        c.zoomSpringsBack = true
        c.pinch(2.0, at: CGPoint(x: 345, y: 550)); c.pinchEnded()
        XCTAssertTrue(c.isZoomed, "the page eases back, it does not snap")
        for _ in 0..<200 where abs(c.viewScale - fitted.0) > 1e-6 { try await Task.sleep(nanoseconds: 10_000_000) }
        XCTAssertFalse(c.isZoomed)
        XCTAssertEqual(c.viewScale, fitted.0, accuracy: 1e-6)
        XCTAssertEqual(c.viewOx, fitted.1, accuracy: 1e-3)
        XCTAssertEqual(c.viewOy, fitted.2, accuracy: 1e-3)
    }

    /// Beside a header line a slot stops half a pitch out: page 1's first line does not claim the
    /// gap under the banner. Body lines still share their boundaries, and slots never overlap.
    func testSlotsStopAtHeaders() throws {
        let p1 = try QvpPage(bytes: try Data(contentsOf: Self.pages.appendingPathComponent("001.qvp")))
        defer { p1.close() }
        let l = p1.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, fillHeight: true))
        let pitch = l.pitch * l.scale
        let first = try XCTUnwrap(p1.lines.firstIndex { !$0.isHeader })
        XCTAssertGreaterThan(first, 0)
        XCTAssertTrue(p1.lines[first - 1].isHeader)
        XCTAssertLessThan(l.slotBottom[first] - l.slotTop[first], pitch * 1.1, "first line's slot is one row, not half the banner gap")
        for i in 1..<l.slotTop.count { XCTAssertGreaterThanOrEqual(l.slotTop[i], l.slotBottom[i - 1] - 1e-3, "slots never overlap") }
        let body = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true))
        for i in 0..<(page.lines.count - 1) where !page.lines[i].isHeader && !page.lines[i + 1].isHeader {
            XCTAssertEqual(body.slotBottom[i], body.slotTop[i + 1], accuracy: 1e-3, "body lines share a boundary")
        }
    }

    /// QvpPageCache: one PERMANENT controller per page; page data cycles
    /// through the LRU and reattaches to that same controller on return.
    @MainActor func testPageCache() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCache needs macOS 14 / iOS 17") }
        let bytes = try Data(contentsOf: Self.pages.appendingPathComponent("042.qvp"))
        var configured = 0
        let cache = QvpPageCache(capacity: 4, data: { _ in bytes }, configure: { _, _ in configured += 1 })
        func waitUntil(_ cond: @MainActor () -> Bool) async {
            for _ in 0..<100 { if cond() { return }; try? await Task.sleep(nanoseconds: 10_000_000) }
        }
        cache.setCurrentPage(42)
        let c42 = cache.controller(for: 42)
        await waitUntil { c42.page != nil }
        XCTAssertNotNil(c42.page)
        XCTAssertTrue(cache.controller(for: 42) === c42, "controller must be permanent")
        // move far and fill beyond capacity → 42's DATA evicts, controller survives detached
        cache.setCurrentPage(100)
        for n in [99, 100, 101, 102] { _ = cache.controller(for: n) }
        await waitUntil { c42.page == nil }
        XCTAssertNil(c42.page, "evicted page must be detached from its controller")
        // come back → SAME controller, data reattached
        cache.setCurrentPage(42)
        await waitUntil { c42.page != nil }
        XCTAssertNotNil(c42.page, "returning must reattach data to the permanent controller")
        XCTAssertTrue(cache.controller(for: 42) === c42)
        XCTAssertGreaterThanOrEqual(configured, 2, "configure runs on every (re)attach")
    }

    /// Every wrapper replays conformance/scenarios/layout.json and must match the engine's numbers.
    func testLayoutScenariosMatchTheEngine() throws {
        let url = Self.repo.appendingPathComponent("conformance/scenarios/layout.json")
        let json = try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as! [String: Any]
        let tolerance = json["tolerance"] as! Double
        let cases = json["cases"] as! [[String: Any]]
        func close(_ a: Float, _ b: Double, _ what: String) {
            XCTAssertLessThanOrEqual(abs(Double(a) - b), tolerance * max(1, abs(b)), "\(what): got \(a), engine says \(b)")
        }
        for c in cases {
            let s = c["spec"] as! [String: Any], want = c["layout"] as! [String: Any]
            let f: (String) -> Float = { Float((s[$0] as! NSNumber).doubleValue) }
            let spec = QvpLayoutSpec(viewportW: f("viewportW"), viewportH: f("viewportH"), padTop: f("padTop"), padBottom: f("padBottom"), padLeft: f("padLeft"), padRight: f("padRight"),
                                     lineSpacing: f("lineSpacing"), lineGap: f("lineGap"), fillHeight: s["fillHeight"] as! Bool, nominalLines: (s["nominalLines"] as! NSNumber).intValue,
                                     cropLeft: f("cropLeft"), cropRight: f("cropRight"), maxAspectSlack: f("maxAspectSlack"))
            let tag = "\(spec.viewportW)x\(spec.viewportH) fill=\(spec.fillHeight) slack=\(spec.maxAspectSlack) crop=\(spec.cropLeft)"
            let l = Self.page.layout(spec)
            let w: (String) -> Double = { (want[$0] as! NSNumber).doubleValue }
            close(l.scale, w("scale"), "\(tag) scale"); close(l.ox, w("ox"), "\(tag) ox"); close(l.oy, w("oy"), "\(tag) oy")
            close(l.contentW, w("contentW"), "\(tag) contentW"); close(l.contentH, w("contentH"), "\(tag) contentH"); close(l.pitch, w("pitch"), "\(tag) pitch")
            close(l.fitScale, w("fitScale"), "\(tag) fitScale"); close(l.fitX, w("fitX"), "\(tag) fitX"); close(l.fitY, w("fitY"), "\(tag) fitY")
            close(l.lineDy.first!, w("lineDy0"), "\(tag) lineDy[0]"); close(l.lineDy.last!, w("lineDyLast"), "\(tag) lineDy[last]")
            close(Self.page.layoutGapToFill(spec), (c["gapToFill"] as! NSNumber).doubleValue, "\(tag) gapToFill")
        }
        XCTAssertEqual(cases.count, 40)
    }
}
