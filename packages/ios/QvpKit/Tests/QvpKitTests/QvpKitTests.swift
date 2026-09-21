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
        XCTAssertGreaterThan(QvpEngine.formatVersion(), 0)
        XCTAssertTrue(QvpEngine.version().split(separator: ".").count == 3)
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
        XCTAssertEqual(page.decorations.count, page.nDecorations)
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
        XCTAssertGreaterThan(page.lineSpacing, 0)
        XCTAssertTrue(page.surahs().map { $0.number }.contains(2))
        XCTAssertTrue(page.ayahKeys().contains { $0 == (2, 255) })
        XCTAssertFalse(page.wordLabel(0).isEmpty)
        XCTAssertFalse(page.ayahLabel(page.words[0].ayahIndex).isEmpty)
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
        XCTAssertEqual(h?.line, w.lineIndex)
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
        let viewX = l.offsetX + (w.x0 + w.x1) / 2 * l.scale
        let viewY = l.offsetY + ((w.y0 + w.y1) / 2 + l.lineDy[w.lineIndex]) * l.scale
        let h = page.hitTestView(viewX, viewY, QvpHitOptions(maxDistance: 6))
        XCTAssertEqual(h?.word, 0)
        let box = page.wordBoundsView(0)
        XCTAssertNotNil(box)
        XCTAssertLessThan(box!.x0, viewX)
        XCTAssertGreaterThan(box!.x1, viewX)
        XCTAssertGreaterThanOrEqual(page.layoutLineSpacingToFill(QvpLayoutSpec(viewportW: 600, viewportH: 1000)), 1)
        XCTAssertEqual(page.grid.lines, 15)
        let wf = page.layoutWastedFraction(QvpLayoutSpec(viewportW: 600, viewportH: 1000))
        XCTAssertTrue(wf >= 0 && wf <= 1)
        // The printed height is the flat layout's contentH, and grows with the width.
        let flat = QvpLayoutSpec(viewportW: 600, viewportH: 1000)
        XCTAssertEqual(page.layoutPrintedHeight(flat), page.layout(flat).contentH, accuracy: 0.01)
        XCTAssertGreaterThan(page.layoutPrintedHeight(QvpLayoutSpec(viewportW: 900, viewportH: 1000)),
                             page.layoutPrintedHeight(flat))
        XCTAssertEqual(QvpLayoutSpec(viewportW: 600, viewportH: 1000).c.surah_frames, 1)
        XCTAssertEqual(QvpLayoutSpec(viewportW: 600, viewportH: 1000, surahFrames: false).c.surah_frames, 0)
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
        XCTAssertTrue(page.selectionText(.rasmUthmani, includeCitation: true).contains(":"))
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
        XCTAssertEqual(cow.first?.number, 2)
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
        c.setBounds(CGSize(width: 690, height: 1100), fromCanvas: 1)
        let l = try XCTUnwrap(p.currentLayout)
        // fit-and-centre, exactly QvpPageView.resetView()
        let expScale: CGFloat = CGFloat(l.contentH) > 1100 ? 1100 / CGFloat(l.contentH) : 1
        XCTAssertEqual(c.viewScale, expScale, accuracy: 1e-5)
        XCTAssertEqual(c.viewOx, max((690 - CGFloat(l.contentW) * expScale) / 2, 0), accuracy: 1e-3)
        XCTAssertEqual(c.viewOy, max((1100 - CGFloat(l.contentH) * expScale) / 2, 0), accuracy: 1e-3)
        XCTAssertFalse(c.isZoomed)
        // lineTransform composes engine layout + view transform
        let w = p.words[0]
        let t = c.lineTransform(w.lineIndex)
        let pt = CGPoint(x: CGFloat((w.x0 + w.x1) / 2), y: CGFloat((w.y0 + w.y1) / 2)).applying(t)
        // a tap at that view point resolves to the same word through the controller
        var tapped: Int? = nil
        c.onWordTap = { word, _ in tapped = word.index }
        c.tap(pt)
        XCTAssertEqual(tapped, 0)
        // double-tap carries the hit under it
        var doubleTapped: Int? = nil
        c.onDoubleTap = { hit in doubleTapped = hit?.word }
        c.doubleTap(pt)
        XCTAssertEqual(doubleTapped, 0)
        // pinch past the fitted scale flips isZoomed; resetView clears it
        c.zoomMode = .magnify
        c.pinch(2.0, at: CGPoint(x: 345, y: 550))
        XCTAssertTrue(c.isZoomed)
        c.resetView()
        XCTAssertFalse(c.isZoomed)
        // line spacing only opens up: the engine clamps values below 1 to the printed lineSpacing
        let printed = try XCTUnwrap(p.currentLayout).lineSpacing
        c.lineSpacing = 0.5
        XCTAssertEqual(try XCTUnwrap(p.currentLayout).lineSpacing, printed)
        c.lineSpacing = 1.5
        XCTAssertGreaterThan(try XCTUnwrap(p.currentLayout).lineSpacing, printed)
    }

    /// A canvas that a newer canvas replaced cannot resize the page: in a rotation the outgoing canvas
    /// reports its half-rotated size after the incoming canvas reported the real one.
    @MainActor func testCanvasControllerIgnoresSizeFromReplacedCanvas() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try QvpPage(bytes: try Data(contentsOf: Self.pages.appendingPathComponent("042.qvp")))
        defer { p.close() }
        let c = QvpCanvasController()
        c.page = p
        c.setBounds(CGSize(width: 690, height: 1100), fromCanvas: 2)
        let scale = try XCTUnwrap(p.currentLayout).scale
        c.setBounds(CGSize(width: 345, height: 550), fromCanvas: 1)
        XCTAssertEqual(try XCTUnwrap(p.currentLayout).scale, scale, "the replaced canvas's size is ignored")
        c.setBounds(CGSize(width: 345, height: 550), fromCanvas: 3)
        XCTAssertNotEqual(try XCTUnwrap(p.currentLayout).scale, scale, "the newest canvas's size lays the page out")
    }

    /// A zoom handed to a canvas that cannot take it up yet is REMEMBERED, not dropped: a host
    /// hands the reading size to a page the reader has not reached, whose canvas has not
    /// reported a size, and the reader must not see it open printed and then jump to size.
    @MainActor func testCarriedZoomWaitsForTheCanvasSize() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let reading = try readingZoom()
        let p = try Self.loadPage(); defer { p.close() }
        let c = QvpCanvasController()
        c.hostScrolls = true
        c.page = p
        c.carryZoom(reading)
        XCTAssertEqual(c.zoom.step, 0, "no size yet: there is nothing to lay the page out against")
        c.setBounds(Self.readingBox, fromCanvas: 1)
        XCTAssertEqual(c.zoom.step, reading.step, "the size arrived, and the carry with it")
        XCTAssertTrue(try XCTUnwrap(p.currentLayout).reflowed)
    }

    /// The other order: the canvas has a size and the PAGE arrives later — page data is cached
    /// and reattaches to a controller that is already on screen.
    @MainActor func testCarriedZoomWaitsForThePageData() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let reading = try readingZoom()
        let p = try Self.loadPage(); defer { p.close() }
        let c = QvpCanvasController()
        c.hostScrolls = true
        c.setBounds(Self.readingBox, fromCanvas: 1)
        c.carryZoom(reading)
        XCTAssertEqual(c.zoom.step, 0, "no page yet")
        c.page = p
        XCTAssertEqual(c.zoom.step, reading.step, "the page arrived, and the carry with it")
    }

    /// A carry still waiting is DROPPED by the policy that outranks it. A host turns zooming
    /// off for a box the reading size does not belong in (a landscape page scrolls at the
    /// printed pitch) or switches the pinch to the magnifier — and must not find the page
    /// reflowed anyway a moment later, when its canvas finally reports a size.
    @MainActor func testWaitingCarryIsDroppedByThePolicy() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let reading = try readingZoom()

        let noZoom = try Self.loadPage(); defer { noZoom.close() }
        let a = QvpCanvasController()
        a.page = noZoom
        a.carryZoom(reading)
        a.zoomEnabled = false
        a.setBounds(Self.readingBox, fromCanvas: 1)
        XCTAssertEqual(a.zoom.step, 0, "zooming was turned off while the carry waited")
        XCTAssertFalse(try XCTUnwrap(noZoom.currentLayout).reflowed)

        let glass = try Self.loadPage(); defer { glass.close() }
        let b = QvpCanvasController()
        b.page = glass
        b.carryZoom(reading)
        b.zoomMode = .magnify
        b.setBounds(Self.readingBox, fromCanvas: 1)
        XCTAssertEqual(b.zoom.step, 0, "a step means nothing under the magnifier")
        XCTAssertFalse(try XCTUnwrap(glass.currentLayout).reflowed)

        // An explicit step outranks a waiting carry too, and it is the step that is kept.
        let picked = try Self.loadPage(); defer { picked.close() }
        let d = QvpCanvasController()
        d.page = picked
        d.carryZoom(reading)
        d.zoomToStep(0)
        d.setBounds(Self.readingBox, fromCanvas: 1)
        XCTAssertEqual(d.zoom.step, 0, "the host asked for the printed page after handing over the carry")
    }

    /// A banner grows with the page as the reader zooms in, until the host caps it. A host that
    /// draws its own frame around the printed surah name has a shape to keep, and an uncapped
    /// name reaches about five times the print before the row stops it.
    @MainActor func testBannerZoomCapsTheSurahName() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        // 582 opens سورة النبأ, so it carries a surah name to measure.
        let bytes = try Data(contentsOf: Self.pages.appendingPathComponent("582.qvp"))

        /// The name's drawn size against its printed size: the scale the layout placed it
        /// under, over the scale the printed page is drawn at.
        func nameAgainstPrint(bannerZoom: Float) throws -> CGFloat {
            let p = try QvpPage(bytes: bytes); defer { p.close() }
            let c = QvpCanvasController()
            c.hostScrolls = true
            c.bannerZoom = bannerZoom
            c.page = p
            c.setBounds(Self.readingBox, fromCanvas: 1)
            let printed = try XCTUnwrap(p.currentLayout).scale
            c.zoomToStep(2)
            XCTAssertTrue(try XCTUnwrap(p.currentLayout).reflowed)
            let deco = try XCTUnwrap(p.decorations.firstIndex { $0.decoration == QvpDecorationKind.SURAH_NAME })
            // `decorationTransform` is the engine layout and the view transform together, so the
            // view scale comes back out to leave the name's own size.
            return c.decorationTransform(deco).d / (c.viewScale * CGFloat(printed))
        }

        let uncapped = try nameAgainstPrint(bannerZoom: 0)
        XCTAssertGreaterThan(uncapped, 1.5, "left alone, the name grows with the words around it")
        let held = try nameAgainstPrint(bannerZoom: 1)
        XCTAssertEqual(held, 1, accuracy: 0.01, "capped at 1, it stays the size it is printed at")
        let grown = try nameAgainstPrint(bannerZoom: 1.5)
        XCTAssertEqual(grown, 1.5, accuracy: 0.01, "and at 1.5 it stops half again bigger")
    }

    /// What a printed row is worth in the reader's box, which a host sizes its own furniture
    /// to. It must not change when the page reflows — and under `hostScrolls` the canvas is
    /// sized to the whole laid-out page, so the controller's own bounds are the PAGE and
    /// answer several times the truth.
    @MainActor func testPrintedPitchIsMeasuredInTheReadersBox() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try Self.loadPage(); defer { p.close() }
        let c = QvpCanvasController()
        c.fillHeight = true
        c.hostScrolls = true
        c.hostViewportHeight = Self.readingBox.height
        c.page = p
        c.setBounds(Self.readingBox, fromCanvas: 1)
        // On the printed page the layout's own pitch IS the answer: fill-height opened it up.
        let printed = c.printedPitch
        XCTAssertEqual(printed, try XCTUnwrap(p.currentLayout).lineSpacing, accuracy: 0.01)
        XCTAssertGreaterThan(printed, p.lineSpacing, "fill-height opens the printed pitch up")

        // Reflow, and let the host size the canvas to the page as it really does.
        c.zoomToStep(2)
        let tall = CGFloat(try XCTUnwrap(p.currentLayout).contentH)
        XCTAssertGreaterThan(tall, Self.readingBox.height * 2, "the reflowed page is taller than the box")
        c.setBounds(CGSize(width: Self.readingBox.width, height: tall), fromCanvas: 1)
        XCTAssertEqual(c.printedPitch, printed, accuracy: 0.01, "a printed row is worth the same, whatever the canvas grew to")
    }

    /// The same answer for a host that crops the printed side margins, which is what a reader
    /// replacing ink-cropped page images does: the crop draws the page bigger, so it fills the
    /// box with less leading, and a pitch measured off the uncropped width is simply too large.
    @MainActor func testPrintedPitchFollowsTheCrop() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try Self.loadPage(); defer { p.close() }
        let ink = try XCTUnwrap(p.cropBounds("page", pad: 1))
        let c = QvpCanvasController()
        c.fillHeight = true
        c.hostScrolls = true
        c.hostViewportHeight = Self.readingBox.height
        c.cropLeft = max(ink.x0, 0)
        c.cropRight = max(p.width - ink.x1, 0)
        c.page = p
        c.setBounds(Self.readingBox, fromCanvas: 1)
        XCTAssertGreaterThan(c.cropLeft + c.cropRight, 0, "page 042 has printed margins to crop")
        XCTAssertEqual(c.printedPitch, try XCTUnwrap(p.currentLayout).lineSpacing, accuracy: 0.01)
    }

    /// A host that scrolls the page itself puts a PRINTED page too tall for its box in that
    /// scroll view, whole: the engine still answers `fitScale` for a host that would rather
    /// shrink it, and this renderer no longer applies it. The layout is measured in the box
    /// the host named, so sizing the canvas to the page — which host scrolling requires —
    /// changes nothing.
    @MainActor func testHostScrolledPrintedPageIsNotShrunk() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try Self.loadPage(); defer { p.close() }
        let c = QvpCanvasController()
        c.fillHeight = true
        c.hostScrolls = true
        c.hostViewportHeight = 400   // a box the page cannot fit at its printed pitch
        c.page = p
        c.setBounds(CGSize(width: Self.readingBox.width, height: 400), fromCanvas: 1)
        let l = try XCTUnwrap(p.currentLayout)
        XCTAssertGreaterThan(l.contentH, 400, "the page is taller than the box at its printed pitch")
        XCTAssertLessThan(l.fitScale, 1, "the engine still says how much a host would have to shrink it")
        XCTAssertEqual(c.viewScale, 1, "and this renderer does not")
        XCTAssertEqual(c.viewOy, 0, "the page opens at its top and the host scrolls it")
        XCTAssertGreaterThan(l.fitX, 0, "the engine centres the page it would have shrunk")
        XCTAssertEqual(c.viewOx, 0, accuracy: 0.01,
                       "the unshrunk page fills the width, so that centring is not applied either")
        // The host sizes the canvas to the page it was just told about.
        c.setBounds(CGSize(width: Self.readingBox.width, height: CGFloat(l.contentH)), fromCanvas: 1)
        XCTAssertEqual(try XCTUnwrap(p.currentLayout).contentH, l.contentH, accuracy: 0.01,
                       "measured in the box, the layout does not follow the canvas it drew")
        XCTAssertEqual(c.viewScale, 1)
    }

    /// The layout is measured in `hostViewportHeight`, never in the canvas: a page that fits
    /// is spread to fill the BOX, a canvas the host padded does not grow the page by that
    /// padding on every pass, and a new box is a new layout.
    @MainActor func testHostScrolledLayoutIsMeasuredInTheBox() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try Self.loadPage(); defer { p.close() }
        let c = QvpCanvasController()
        c.fillHeight = true
        c.hostScrolls = true
        c.hostViewportHeight = 800   // a box the page fits, with room fill-height spreads into
        c.page = p
        c.setBounds(CGSize(width: Self.readingBox.width, height: 800), fromCanvas: 1)
        XCTAssertEqual(try XCTUnwrap(p.currentLayout).contentH, 800, accuracy: 0.01, "fill-height fills the box")
        // A host that pads the page hands back a canvas 12pt taller than the page it was told.
        c.setBounds(CGSize(width: Self.readingBox.width, height: 812), fromCanvas: 1)
        XCTAssertEqual(try XCTUnwrap(p.currentLayout).contentH, 800, accuracy: 0.01,
                       "the canvas is the layout's output, not its viewport — no feedback")
        let before = c.layoutRevision
        c.hostViewportHeight = 700
        XCTAssertEqual(try XCTUnwrap(p.currentLayout).contentH, 700, accuracy: 0.01, "a new box is a new fill target")
        XCTAssertGreaterThan(c.layoutRevision, before, "and the host hears of it")
    }

    /// A magnify peek under host scrolling moves the view transform, and the band a pinch
    /// paints is cut in the SCALED page — it must stay under the fingers as the page grows.
    @MainActor func testMagnifyPeekUnderHostScrollsKeepsTheBandUnderTheFingers() async throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try Self.loadPage(); defer { p.close() }
        let c = QvpCanvasController()
        c.fillHeight = true
        c.hostScrolls = true
        c.hostViewportHeight = 400
        c.zoomMode = .magnify
        c.zoomSpringsBack = true
        c.page = p
        c.setBounds(CGSize(width: Self.readingBox.width, height: 400), fromCanvas: 1)
        let pageHeight = CGFloat(try XCTUnwrap(p.currentLayout).contentH)
        c.setBounds(CGSize(width: Self.readingBox.width, height: pageHeight), fromCanvas: 1)
        XCTAssertEqual(c.bandTop, 0); XCTAssertEqual(c.bandHeight, pageHeight, accuracy: 0.01)

        let focal = CGPoint(x: Self.readingBox.width / 2, y: 500)
        c.pinch(2, at: focal)
        XCTAssertEqual(c.viewScale, 2, accuracy: 1e-6)
        // Where the fingers' point on the page is now, in the scaled page.
        let scaledFocal = focal.y - c.viewOy
        XCTAssertEqual(scaledFocal, focal.y * 2, accuracy: 1e-3, "the point under the fingers scaled about them")
        XCTAssertLessThanOrEqual(c.bandTop, scaledFocal, "the band starts at or above the fingers")
        XCTAssertGreaterThanOrEqual(c.bandTop + c.bandHeight, scaledFocal, "and ends at or below them")
        XCTAssertEqual(c.bandHeight, min(800, pageHeight * 2), accuracy: 0.01, "two boxes, or the whole scaled page")

        // The peek springs back and the whole page is painted again.
        c.pinchEnded()
        for _ in 0..<200 where abs(c.viewScale - 1) > 1e-6 { try await Task.sleep(nanoseconds: 10_000_000) }
        XCTAssertEqual(c.viewScale, 1, accuracy: 1e-6)
        XCTAssertEqual(c.viewOy, 0, accuracy: 1e-3)
        XCTAssertEqual(c.bandTop, 0, accuracy: 1e-3); XCTAssertEqual(c.bandHeight, pageHeight, accuracy: 0.01)
    }

    /// The zoom steps are the box's, not the canvas's: a page that reflowed and grew its canvas
    /// offers the same steps it did when the canvas was the box.
    @MainActor func testZoomStepsUnderHostScrollsFollowTheBox() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        func steps(canvasHeight: CGFloat) throws -> [Float] {
            let p = try Self.loadPage(); defer { p.close() }
            let c = QvpCanvasController()
            c.fillHeight = true
            c.hostScrolls = true
            c.hostViewportHeight = Self.readingBox.height
            c.page = p
            c.setBounds(CGSize(width: Self.readingBox.width, height: canvasHeight), fromCanvas: 1)
            return c.zoomSteps
        }
        let short = try steps(canvasHeight: 400), tall = try steps(canvasHeight: 1200)
        XCTAssertFalse(short.isEmpty)
        XCTAssertEqual(short, tall)
    }

    /// The box a reading zoom is carried in, and a page loaded fresh for a controller to own
    /// (the shared `page` is laid out by every test that touches it).
    static let readingBox = CGSize(width: 390, height: 700)
    static func loadPage() throws -> QvpPage {
        try QvpPage(bytes: try Data(contentsOf: pages.appendingPathComponent("042.qvp")))
    }

    /// What a reader pinched to on one page: the control a carry hands to the next.
    @MainActor private func readingZoom() throws -> QvpZoom {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try Self.loadPage(); defer { p.close() }
        let c = QvpCanvasController()
        c.hostScrolls = true
        c.page = p
        c.setBounds(Self.readingBox, fromCanvas: 1)
        c.zoomToStep(2)
        XCTAssertGreaterThan(c.zoom.step, 0, "the source page is zoomed in")
        XCTAssertGreaterThan(c.zoom.zoom, 1, "and its ink is bigger than the print")
        return c.zoom
    }

    /// The view-space geometry a host draws its overlays from. On the printed page every
    /// answer is the page-unit one through the layout's scale and offset; on a reflowed page
    /// it is wherever the rows put the words, and every word still has one.
    func testViewSpaceGeometryFollowsTheLayout() throws {
        let l = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100))
        XCTAssertFalse(l.reflowed)
        let marks = page.ayahMarks(), marksView = page.ayahMarksView()
        XCTAssertEqual(marks.count, marksView.count)
        XCTAssertEqual(marksView[0].cx, l.offsetX + marks[0].cx * l.scale, accuracy: 0.01)
        XCTAssertEqual(marksView[0].cy, l.offsetY + (marks[0].cy + l.lineDy[marks[0].line]) * l.scale, accuracy: 0.01)
        let areas = page.hitAreas(), areasView = page.hitAreasView()
        XCTAssertEqual(areas.count, areasView.count)
        XCTAssertEqual(areasView[0].x0, l.offsetX + areas[0].x0 * l.scale, accuracy: 0.01)
        let bands = page.wordBands([0, 1]), bandsView = page.wordBandsView([0, 1])
        XCTAssertEqual(bands.count, bandsView.count)
        XCTAssertEqual(bandsView[0].x0, l.offsetX + bands[0].x0 * l.scale, accuracy: 0.01)

        let r = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, reflow: QvpReflowSpec(zoom: 1.8)))
        XCTAssertTrue(r.reflowed)
        XCTAssertEqual(page.hitAreasView().count, page.nWords, "every word keeps a hit area on a reflowed page")
        XCTAssertEqual(page.ayahMarksView().count, marks.count)
        XCTAssertFalse(page.wordBandsView(Array(0..<page.nWords)).isEmpty)
        _ = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100))
    }

    /// The two boxes a host draws its own surah frame from: the box a frame fills and the
    /// title ink in it. Neither counts a native frame, so hiding one leaves both alone.
    func testSurahHeaderBoxesHoldTheTitleAndIgnoreTheFrame() throws {
        let p = try QvpPage(bytes: try Data(contentsOf: Self.pages.appendingPathComponent("001.qvp")))
        defer { p.close() }
        let l = p.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100))
        let headers = p.surahHeaders()
        XCTAssertFalse(headers.isEmpty, "page 001 opens al-Fatihah")
        let h = headers[0]
        XCTAssertEqual(h.surah, 1)
        XCTAssertLessThanOrEqual(h.x0, h.titleX0, "the frame box holds the title")
        XCTAssertGreaterThanOrEqual(h.x1, h.titleX1)
        XCTAssertLessThan(h.y0, h.y1)

        // the view boxes are the page-unit ones through the layout's scale, offset and line shift
        let view = p.surahHeadersView()
        XCTAssertEqual(view.count, headers.count)
        XCTAssertEqual(view[0].x0, l.offsetX + h.x0 * l.scale, accuracy: 0.01)
        XCTAssertEqual(view[0].titleY0, l.offsetY + (h.titleY0 + l.lineDy[h.line]) * l.scale, accuracy: 0.01)

        _ = p.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, surahFrames: false))
        XCTAssertEqual(p.surahHeaders(), headers, "the boxes leave a native frame out")
        XCTAssertEqual(p.surahHeadersView(), view)
    }

    /// A decoration's transform is its line's on the printed page, and its own group's once
    /// the page has reflowed — where a surah name keeps a row to itself.
    @MainActor func testDecorationTransformFollowsTheReflow() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let p = try QvpPage(bytes: try Data(contentsOf: Self.pages.appendingPathComponent("042.qvp")))
        defer { p.close() }
        let c = QvpCanvasController()
        c.page = p
        c.setBounds(CGSize(width: 690, height: 1100), fromCanvas: 1)
        let deco = try XCTUnwrap(p.decorations.indices.first { p.decorations[$0].decoration == QvpDecorationKind.AYAH_MARK })
        XCTAssertEqual(c.decorationTransform(deco), c.lineTransform(p.decorations[deco].line))
        c.zoomMode = .continuous
        c.zoomToStep(2)
        XCTAssertTrue(p.currentLayout?.reflowed == true)
        let t = c.decorationTransform(deco)
        XCTAssertNotEqual(t, .identity)
        let printed = CGPoint(x: CGFloat(p.decorations[deco].x0), y: CGFloat(p.decorations[deco].y0)).applying(c.lineTransform(p.decorations[deco].line))
        let placed = CGPoint(x: CGFloat(p.decorations[deco].x0), y: CGFloat(p.decorations[deco].y0)).applying(t)
        XCTAssertNotEqual(printed, placed, "a reflowed page moved the decoration off its printed line")
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
        XCTAssertEqual(end.transform.offsetY, 10, accuracy: 1e-9)

        let p = try QvpPage(bytes: try Data(contentsOf: Self.pages.appendingPathComponent("042.qvp")))
        defer { p.close() }
        let c = QvpCanvasController()
        c.page = p
        c.setBounds(CGSize(width: 690, height: 1100), fromCanvas: 1)
        // springing back is the magnifying pinch's own behaviour: a peek, not a reading zoom
        c.zoomMode = .magnify
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

    /// Beside a header line a slot stops half a lineSpacing out: page 1's first line does not claim the
    /// gap under the banner. Body lines still share their boundaries, and slots never overlap.
    func testSlotsStopAtHeaders() throws {
        let p1 = try QvpPage(bytes: try Data(contentsOf: Self.pages.appendingPathComponent("001.qvp")))
        defer { p1.close() }
        let l = p1.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, fillHeight: true))
        let lineSpacing = l.lineSpacing * l.scale
        let first = try XCTUnwrap(p1.lines.firstIndex { !$0.isHeader })
        XCTAssertGreaterThan(first, 0)
        XCTAssertTrue(p1.lines[first - 1].isHeader)
        XCTAssertLessThan(l.slotBottom[first] - l.slotTop[first], lineSpacing * 1.1, "first line's slot is one row, not half the banner gap")
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
                                     lineSpacing: f("lineSpacing"), fillHeight: s["fillHeight"] as! Bool, gridLines: (s["gridLines"] as! NSNumber).intValue,
                                     cropLeft: f("cropLeft"), cropRight: f("cropRight"), maxAspectSlack: f("maxAspectSlack"))
            let tag = "\(spec.viewportW)x\(spec.viewportH) fill=\(spec.fillHeight) slack=\(spec.maxAspectSlack) crop=\(spec.cropLeft)"
            let l = Self.page.layout(spec)
            let w: (String) -> Double = { (want[$0] as! NSNumber).doubleValue }
            close(l.scale, w("scale"), "\(tag) scale"); close(l.offsetX, w("offsetX"), "\(tag) offsetX"); close(l.offsetY, w("offsetY"), "\(tag) offsetY")
            close(l.contentW, w("contentW"), "\(tag) contentW"); close(l.contentH, w("contentH"), "\(tag) contentH"); close(l.lineSpacing, w("lineSpacing"), "\(tag) lineSpacing")
            close(l.fitScale, w("fitScale"), "\(tag) fitScale"); close(l.fitX, w("fitX"), "\(tag) fitX"); close(l.fitY, w("fitY"), "\(tag) fitY")
            close(l.lineDy.first!, w("lineDy0"), "\(tag) lineDy[0]"); close(l.lineDy.last!, w("lineDyLast"), "\(tag) lineDy[last]")
            close(Self.page.layoutLineSpacingToFill(spec), (c["lineSpacingToFill"] as! NSNumber).doubleValue, "\(tag) lineSpacingToFill")
            close(Self.page.layoutWastedFraction(spec), (c["wastedFraction"] as! NSNumber).doubleValue, "\(tag) wastedFraction")
        }
        XCTAssertEqual(cases.count, 40)
    }
}
