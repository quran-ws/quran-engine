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
        XCTAssertEqual(QvpEngine.markName(1), "fatha")
        XCTAssertEqual(QvpEngine.markName(7), QvpEngine.markName(markId("shadda")))
        XCTAssertEqual(QvpEngine.markFromName("shadda"), 7)
        XCTAssertFalse(QvpEngine.familyName(QvpFamily.DIACRITIC).isEmpty)
        XCTAssertFalse(QvpEngine.categoryName(QvpCategory.HARAKA).isEmpty)
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
        XCTAssertEqual(page.findWord(w0.sura, w0.ayah, w0.word), 0)
        XCTAssertEqual(page.wid(0), w0.wid)
        XCTAssertGreaterThan(page.naturalPitch, 0)
        XCTAssertTrue(page.surahs().map { $0.number }.contains(2))
        XCTAssertTrue(page.ayahKeys().contains { $0 == (2, 255) })
        XCTAssertFalse(page.wordLabel(0).isEmpty)
        XCTAssertFalse(page.ayahLabel(page.words[0].ayahIdx).isEmpty)
        XCTAssertFalse(page.markers().isEmpty)
        XCTAssertEqual(page.lineBands().count, 15)
        XCTAssertEqual(page.hitBoxes().count, 147)
        XCTAssertTrue(page.text("page").contains(w0.text))
    }

    func testSearchAllah7() {
        let m = page.search("الله")
        XCTAssertEqual(m.count, 7)
        XCTAssertFalse(m[0].text.isEmpty)
        XCTAssertEqual(m[0].wid, page.wid(m[0].word))
    }

    func testResolve2_255() {
        let ws = page.resolve("2:255")
        XCTAssertEqual(ws.count, 50)
        XCTAssertEqual(page.resolve(Target.ayah(2, 255)), ws)
        XCTAssertEqual(page.resolve(ws), ws)
        XCTAssertEqual(page.resolve("2:255:1"), [ws[0]])
        XCTAssertEqual(page.citation(ws), "2:255")
        XCTAssertEqual(page.ayahWordCount(2, 255).count, 50)
        XCTAssertFalse(page.text("2:255").isEmpty)
    }

    func testAttachWordsSidecar() throws {
        let url = Self.pages.appendingPathComponent("042.words.json")
        try XCTSkipUnless(FileManager.default.fileExists(atPath: url.path), "no sidecar")
        XCTAssertGreaterThanOrEqual(page.attachWords(try Data(contentsOf: url)), 0)
        XCTAssertEqual(page.attachWords("not json"), -1)
        XCTAssertTrue(page.hasForm(.imlaei))
        XCTAssertFalse(page.wordForm(0, .imlaei).isEmpty)
        XCTAssertFalse(page.wordForm(0, .search).isEmpty)
    }

    func testHitTestEx() {
        let w = page.words[0]
        let cx = (w.x0 + w.x1) / 2, cy = (w.y0 + w.y1) / 2
        let h = page.hitTestEx(cx, cy)
        XCTAssertNotNil(h)
        XCTAssertEqual(h?.word, 0)
        XCTAssertEqual(h?.distance, 0)
        XCTAssertEqual(h?.line, w.lineIdx)
        XCTAssertEqual(page.hitTest(cx, cy)?.word, 0)
        var inside: QvpHitEx?
        var y = w.y0
        outer: while y <= w.y1 {
            var x = w.x0
            while x <= w.x1 { if let e = page.hitTestEx(x, y), e.word == 0, e.exact { inside = e; break outer }; x += 0.5 }
            y += 0.5
        }
        XCTAssertNotNil(inside, "no point of word 0 is inside its ink")
        XCTAssertGreaterThanOrEqual(inside!.path, 0)
        XCTAssertEqual(page.pathWord(inside!.path), 0)
        XCTAssertNil(page.hitTestEx(-500, -500, QvpHitOptions(maxDistance: 6)))
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
        let h = page.hitTestViewEx(vx, vy, QvpHitOptions(maxDistance: 6))
        XCTAssertEqual(h?.word, 0)
        let box = page.wordBoxView(0)
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
        XCTAssertTrue(page.styled().isEmpty)
        let h = page.style(Selector.wordMark(0, 1), 0xef6c00ff)
        XCTAssertGreaterThan(h, 0)
        let s = page.styled()
        XCTAssertEqual(s.count, 1)
        XCTAssertEqual(s[0].color, 0xef6c00ff)
        XCTAssertEqual(page.pathWord(s[0].path), 0)
        XCTAssertEqual(page.pathNthMark(s[0].path), 1)
        XCTAssertEqual(page.colorOf(s[0].path), 0xef6c00ff)
        XCTAssertEqual(page.paint()[s[0].path], 0xef6c00ff)
        XCTAssertTrue(page.styleHandles().contains(h))
        XCTAssertGreaterThan(page.unstyle(h), 0)
        XCTAssertTrue(page.styled().isEmpty)
        let th = page.theme(QvpTheme(diacritics: 0x1a73e8ff, dots: 0xc62828ff, marks: ["shadda": 0x0a7d32ff]))
        XCTAssertFalse(page.styled().isEmpty)
        let hh = page.hide(Selector.kind(QvpKind.MARK))
        XCTAssertTrue(page.styled().contains { $0.color & 0xff == 0 })
        page.unstyle(hh); page.unstyle(th)
        let ht = page.styleTarget("2:255", 0x0a7d32ff, layer: QvpLayer.TOP)
        XCTAssertGreaterThan(page.styled().count, 50)
        page.unstyle(ht)
        XCTAssertTrue(page.styled().isEmpty)
        page.setDefaultInk(0x3b2a14ff)
        XCTAssertEqual(page.defaultInk, 0x3b2a14ff)
        XCTAssertEqual(page.paint()[0], 0x3b2a14ff)
        page.setDefaultInk(0x231f20ff)
    }

    func testHighlightTick() {
        page.clearHighlights()
        page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true))
        let h = page.highlight("2:255", QvpHighlightStyle(mode: .both, transitionMs: 200))
        XCTAssertGreaterThan(h, 0)
        XCTAssertTrue(page.tick(100))
        let boxes = page.highlightBoxes()
        XCTAssertEqual(boxes.count, 6)
        XCTAssertTrue(boxes.allSatisfy { $0.id == h })
        XCTAssertEqual(page.highlightHandles(), [h])
        XCTAssertEqual(page.highlightWords(h).count, 50)
        XCTAssertFalse(page.tick(1000))
        XCTAssertTrue(page.rehighlight(h, Target.word(0)))
        XCTAssertTrue(page.restyleHighlight(h, QvpHighlightStyle(mode: .band)))
        XCTAssertTrue(page.unhighlight(h))
        _ = page.tick(5000)
        page.clearHighlights()
        XCTAssertTrue(page.highlightHandles().isEmpty)
        XCTAssertEqual(page.bandBoxes(page.resolve("2:255")).count, 6)
    }

    func testMaskReveal() {
        page.mask("2:255")
        XCTAssertEqual(page.maskHidden().count, 50)
        XCTAssertEqual(page.maskWords().count, 50)
        XCTAssertEqual(page.revealNext(1), 1)
        XCTAssertEqual(page.maskHidden().count, 49)
        XCTAssertEqual(page.hideBack(1), 1)
        XCTAssertEqual(page.maskHidden().count, 50)
        page.unmask()
        XCTAssertTrue(page.maskHidden().isEmpty)
        page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100))
        page.mask("2:255", .block)
        XCTAssertFalse(page.maskBoxes().isEmpty)
        page.unmask()
        let steps = page.revealStart(lit: 2)
        XCTAssertGreaterThan(steps, 0)
        XCTAssertEqual(page.revealSteps(), steps)
        XCTAssertEqual(page.revealAt(), -1)
        XCTAssertTrue(page.revealGoto(0))
        XCTAssertEqual(page.revealAt(), 0)
        XCTAssertGreaterThanOrEqual(page.revealStepOf(0), 0)
        page.revealStop()
        XCTAssertNil(page.revealAt())
    }

    func testSelection() {
        page.select(0, 3)
        XCTAssertEqual(page.selection(), [0, 1, 2, 3])
        XCTAssertTrue(page.selectionText(.uthmani, citation: true).contains(":"))
        page.clearSelection()
        XCTAssertTrue(page.selection().isEmpty)
    }

    func testCropSvg() {
        let svg = page.cropSvg("2:255:1", background: 0xfffdf7ff)
        XCTAssertNotNil(svg)
        XCTAssertTrue(svg!.hasPrefix("<svg"))
        let box = page.cropBox("2:255")
        XCTAssertEqual(box?.nWords, 50)
    }

    func testAtlas() throws {
        let url = Self.pages.appendingPathComponent("atlas.qva")
        try XCTSkipUnless(FileManager.default.fileExists(atPath: url.path), "no atlas")
        let atlas = try QvpAtlas(bytes: try Data(contentsOf: url))
        XCTAssertEqual(atlas.pages(), 604)
        XCTAssertEqual(atlas.pageOf(2, 255), 42)
        XCTAssertTrue(atlas.pagesOfJuz(30)! == (582, 604))
        let cow = atlas.findSurah("cow")
        XCTAssertFalse(cow.isEmpty)
        XCTAssertEqual(cow.first?.n, 2)
        XCTAssertFalse(atlas.surah(36)!.latin.isEmpty)
        XCTAssertEqual(atlas.surahs().count, 114)
        XCTAssertEqual(atlas.pageOfSurah(36), atlas.surah(36)!.page)
        XCTAssertEqual(atlas.juz(30)!.page, 582)
        XCTAssertEqual(atlas.juzAt(2, 255), 3)
        XCTAssertEqual(atlas.pageRange(42)!.first.0, 2)
        atlas.close()
    }
}
