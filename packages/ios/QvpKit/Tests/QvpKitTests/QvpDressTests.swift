// Dress over the Swift wrapper: the same checks as `dress_over_the_abi` in
// crates/qvp-ffi/tests/abi.rs. Data: <repo>/dist/pages/604.qvp.
import XCTest
import SwiftUI
@testable import QvpKit

final class QvpDressTests: XCTestCase {
    /// `ornament_bytes()` of crates/qvp-ffi/tests/abi.rs, encoded: three ornaments of the
    /// style "test" (a font medallion, a scanned band and a scanned border), each one filled
    /// and one stroked square, with parts c1 (0x00649bff) and line (0x303641ff, stroked).
    /// Rebuild it from that helper when the QVO1 encoding changes.
    static let ornamentBytes = Data(base64Encoded: "UVZPMegDAwABBHRlc3QOSGFmcyAnYW4gJ0FzaW0PQ0MtQlktTkMtU0EtNC4wC3Byb3Zpc2lvbmFsAAZhIHRlc3QCAmMx/5tkAAAEbGluZf9BNjABAAAAAAAAAAAAAKBCAADIQgEAAKBBAACgQQAAIEIAAHBCAAIAAQAEdGVzdA5IYWZzICdhbiAnQXNpbQ9DQy1CWS1OQy1TQS00LjALcHJvdmlzaW9uYWwABmEgdGVzdAICYzH/m2QAAARsaW5l/0E2MAEAAAAAAAAAAAAASEQAAMhCAQAAyEIAACBBAAAWRAAAoEICAgACAAR0ZXN0DkhhZnMgJ2FuICdBc2ltD0NDLUJZLU5DLVNBLTQuMAtwcm92aXNpb25hbAAGYSB0ZXN0AgJjMf+bZAAABGxpbmX/QTYwAQAAAAAAAAAAAACMQgAAyEIBAACgQAAAoEAAAHBCAAC0QgQCAAYAAAAAEwECyAETEwAAACYTAQLIATkTAAAATBMBAsgBXxNyAAAAAYDiCQABAMCaDAH/4QkABAAAAAGA4gkAAQDAmgwB/+EJAAQAAAABgNRhAAEAwJoMAf/TYQAEAAAAAYDUYQABAMCaDAH/02EABAAAAAHgxQgAAQDAmgwB38UIAAQAAAAB4MUIAAEAwJoMAd/FCAAE")!

    func testOrnamentSet() throws {
        XCTAssertThrowsError(try QvpOrnaments(bytes: Data([1, 2, 3])))
        // ornamentsLoad, ornamentCount, ornamentInfo, ornamentPart, ornamentFind, ornamentsFree
        let set = try QvpOrnaments(bytes: Self.ornamentBytes)
        defer { set.close() }
        XCTAssertEqual(set.count, 3)
        XCTAssertEqual(set.list.map(\.kind), [.ayahMark, .surahHeader, .pageFrame])
        let mark = set.list[0]
        XCTAssertEqual(mark.lineage, .font)
        XCTAssertEqual(set.list[1].lineage, .scan)
        XCTAssertEqual(mark.style, "test")
        XCTAssertEqual(mark.riwayah, "Hafs 'an 'Asim")
        XCTAssertFalse(mark.hasSlices)
        XCTAssertEqual(mark.licence, OrnamentLicence(id: "CC-BY-NC-SA-4.0", status: "provisional", redistributable: false, attribution: "a test"))
        XCTAssertEqual(mark.parts, [OrnamentPart(index: 0, color: 0x00649bff, stroke: false, name: "c1"),
                                    OrnamentPart(index: 1, color: 0x303641ff, stroke: true, name: "line")])
        XCTAssertEqual(set.find(.pageFrame, "test")?.index, 2)
        XCTAssertNil(set.find(.ayahMark, "nope"))
        XCTAssertEqual(set.ofKind(.surahHeader).map(\.index), [1])
        let styles = set.styles()
        XCTAssertEqual(styles.count, 1)
        XCTAssertEqual(styles[0].style, "test")
        XCTAssertEqual([styles[0].ayahMark?.index, styles[0].surahHeader?.index, styles[0].pageFrame?.index], [0, 1, 2])
        set.close(); set.close()
        XCTAssertFalse(set.isOpen)
        XCTAssertNil(set.find(.pageFrame, "test"))
    }

    func testDressPage604() throws {
        let url = QvpKitTests.pages.appendingPathComponent("604.qvp")
        guard let bytes = try? Data(contentsOf: url) else { throw XCTSkip("no page data at \(url.path); run scripts/sync-test-data.sh") }
        let page = try QvpPage(bytes: bytes)
        defer { page.close() }
        let set = try QvpOrnaments(bytes: Self.ornamentBytes)
        XCTAssertNil(page.dressInfo(), "undressed")
        XCTAssertEqual(page.dressRevision(), 0)
        XCTAssertTrue(page.dressGeometry().isEmpty)
        let ring = try XCTUnwrap((0..<page.nPaths).first { page.pathKind($0) == QvpKind.AYAH_MARK_ORNAMENT && page.colorOf($0) & 0xff != 0 })

        // a refused spec: nothing to dress, or an ornament of another kind
        XCTAssertNil(page.dress(set, DressSpec()))
        XCTAssertNil(page.dress(set, DressSpec(ayahMark: 2)))
        XCTAssertNil(page.dress(set, DressSpec(style: "nope")))

        // a style fills all three kinds, and a colour repaints the border's fill
        let info = try XCTUnwrap(page.dress(set, DressSpec(style: "test", ayahMarkScale: 1, colors: [OrnamentColor(set.list[2], part: 0, color: 0xff0000ff)])))
        XCTAssertEqual([info.ayahMark, info.surahHeader, info.pageFrame], [0, 1, 2])
        XCTAssertGreaterThan(info.ayahMarks, 0)
        XCTAssertGreaterThan(info.surahHeaders, 0)
        XCTAssertTrue(info.frameStretched, "no slices in this set")
        XCTAssertEqual(info.frameRepeats, 0)
        XCTAssertLessThan(info.viewBox[0], 0)
        XCTAssertGreaterThan(info.viewBox[2], page.width)
        XCTAssertEqual(page.colorOf(ring) & 0xff, 0, "the print's own ring is hidden")
        XCTAssertEqual(page.dressInfo(), info)

        // a layout rebuilds the ornament layer and the border reaches past the content
        page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 20, padBottom: 20))
        let laid = try XCTUnwrap(page.dressInfo())
        XCTAssertNotEqual(laid.revision, info.revision)
        XCTAssertEqual(page.dressRevision(), laid.revision)
        XCTAssertTrue(laid.overflow.allSatisfy { $0 > 0 }, "\(laid.overflow)")

        let draws = page.dressGeometry()
        XCTAssertEqual(draws.count, laid.draws)
        XCTAssertEqual(draws.count, (laid.ayahMarks + laid.surahHeaders + 1) * 2, "one filled and one stroked outline per placed asset")
        let places = page.layoutPlacements().count
        var red = 0
        for d in draws {
            XCTAssertFalse(d.path.isEmpty)
            if d.stroke {
                XCTAssertGreaterThan(d.strokeWidth, 0)
                XCTAssertEqual(d.color, 0x303641ff)
            } else if d.kind == .pageFrame {
                XCTAssertEqual(d.color, 0xff0000ff)
                red += 1
            } else {
                XCTAssertEqual(d.color, 0x00649bff)
            }
            // the border belongs to no placement; the rest are drawn under one in range
            XCTAssertEqual(d.placement == nil, d.kind == .pageFrame)
            if let q = d.placement { XCTAssertLessThan(q, places) }
        }
        XCTAssertEqual(red, 1)
        XCTAssertEqual(page.dressGeometry().count, draws.count, "read once per revision")

        // a kind set on its own overrides the style; line art keeps the strokes alone
        let mixed = try XCTUnwrap(page.dress(set, DressSpec(style: "test", pageFrame: .absent, lineArt: Set(OrnamentKind.allCases))))
        XCTAssertNil(mixed.pageFrame)
        XCTAssertTrue(page.dressGeometry().allSatisfy { $0.stroke && $0.placement != nil })

        // the set can go: the page keeps its dress
        set.close()
        XCTAssertNotNil(page.dressInfo())
        page.undress()
        XCTAssertNil(page.dressInfo())
        XCTAssertEqual(page.dressRevision(), 0)
        XCTAssertNotEqual(page.colorOf(ring) & 0xff, 0, "undress brings the printed ring back")
    }

    /// The release's own set, when the data folder has one.
    func testReleaseOrnamentSet() throws {
        let url = QvpKitTests.pages.appendingPathComponent("ornaments.qvo")
        guard FileManager.default.fileExists(atPath: url.path) else { throw XCTSkip("no ornaments.qvo beside the pages") }
        let set = try QvpOrnaments(path: url.path)
        XCTAssertGreaterThan(set.count, 0)
        XCTAssertEqual(set.list.count, set.count)
        for o in set.list { XCTAssertEqual(set.find(o.kind, o.style)?.kind, o.kind) }
    }

    // ── the renderers ──

    static func page604() throws -> QvpPage {
        let url = QvpKitTests.pages.appendingPathComponent("604.qvp")
        guard let bytes = try? Data(contentsOf: url) else { throw XCTSkip("no page data at \(url.path); run scripts/sync-test-data.sh") }
        return try QvpPage(bytes: bytes)
    }
    static let box = CGSize(width: 390, height: 700)
    /// The fixture's border filled red, so a probe can tell it from the ink.
    static func redBorder(_ set: QvpOrnaments) -> DressSpec {
        DressSpec(style: "test", ayahMarkScale: 1, colors: [OrnamentColor(set.list[2], part: 0, color: 0xff0000ff)])
    }

    /// A frame of `c` drawn through the canvas's own drawing path, as RGBA bytes.
    @available(macOS 14.0, iOS 17.0, *)
    @MainActor static func render(_ c: QvpCanvasController) throws -> (pixels: [UInt8], width: Int, height: Int) {
        let view = Canvas { ctx, size in c.draw(in: ctx, size: size, displayScale: 1) }
            .frame(width: box.width, height: box.height)
        let r = ImageRenderer(content: view)
        r.scale = 1
        let img = try XCTUnwrap(r.cgImage)
        let w = img.width, h = img.height
        var px = [UInt8](repeating: 0, count: w * h * 4)
        let ctx = try XCTUnwrap(CGContext(data: &px, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                          space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        ctx.draw(img, in: CGRect(x: 0, y: 0, width: w, height: h))
        return (px, w, h)
    }
    static func redPixels(_ f: (pixels: [UInt8], width: Int, height: Int)) -> Int {
        stride(from: 0, to: f.pixels.count, by: 4).filter { f.pixels[$0] > 200 && f.pixels[$0 + 1] < 60 && f.pixels[$0 + 2] < 60 && f.pixels[$0 + 3] > 200 }.count
    }

    /// QvpPageCanvas paints the ornament layer, fitted with its border, as QvpPageView does.
    @MainActor func testCanvasDrawsTheDress() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let page = try Self.page604(); defer { page.close() }
        let set = try QvpOrnaments(bytes: Self.ornamentBytes)
        let c = QvpCanvasController()
        c.page = page
        c.setBounds(Self.box, fromCanvas: 1)
        let bare = try Self.render(c)
        XCTAssertNil(c.ornaments.last, "an undressed page has no ornament layer")
        XCTAssertEqual(Self.redPixels(bare), 0)

        XCTAssertNotNil(page.dress(set, Self.redBorder(set)))
        c.resetView()
        let l = try XCTUnwrap(page.currentLayout)
        let o = try XCTUnwrap(page.dressOverflow)
        // the fit is the engine's: the layout's own, read again after the dress
        XCTAssertEqual(c.viewScale, CGFloat(l.fitScale), accuracy: 0.0001)
        XCTAssertEqual(c.viewOx, CGFloat(l.fitX), accuracy: 0.5)
        XCTAssertEqual(c.viewOy, CGFloat(l.fitY), accuracy: 0.5)
        // and it holds the border, not only the text
        let s = c.viewScale
        XCTAssertGreaterThanOrEqual(c.viewOy - o.top * s, -0.5)
        XCTAssertLessThanOrEqual(c.viewOy + (CGFloat(l.contentH) + o.bottom) * s, Self.box.height + 0.5)
        XCTAssertGreaterThanOrEqual(c.viewOx - o.left * s, -0.5)
        XCTAssertLessThanOrEqual(c.viewOx + (CGFloat(l.contentW) + o.right) * s, Self.box.width + 0.5)

        let dressed = try Self.render(c)
        let layer = try XCTUnwrap(c.ornaments.last, "the canvas rasterised the ornament layer")
        XCTAssertEqual(layer.width, Int(Self.box.width))
        XCTAssertGreaterThan(Self.redPixels(dressed), 0, "the red border is on the canvas")
        // a second frame takes the same raster
        _ = try Self.render(c)
        XCTAssertTrue(c.ornaments.last === layer, "one raster per dress revision and view")

        page.undress(); c.resetView()
        XCTAssertEqual(Self.redPixels(try Self.render(c)), 0, "undress takes the layer off")
        XCTAssertNil(c.ornaments.last)
    }

    /// A reflowed, dressed page scrolls to the border's top and bottom, never past them and
    /// never short of them. The clamp holds content plus overflow.
    @MainActor func testCanvasScrollKeepsTheBorder() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let page = try Self.page604(); defer { page.close() }
        let set = try QvpOrnaments(bytes: Self.ornamentBytes)
        let c = QvpCanvasController()
        c.page = page
        c.setBounds(Self.box, fromCanvas: 1)
        c.zoomToStep(c.zoomSteps.count)
        XCTAssertTrue(try XCTUnwrap(page.currentLayout).reflowed)
        XCTAssertNotNil(page.dress(set, Self.redBorder(set)))
        c.resetView()
        // the dress lays the reflowed page out again, with rows narrow enough for the border
        let l = try XCTUnwrap(page.currentLayout)
        XCTAssertTrue(l.reflowed)
        let o = try XCTUnwrap(page.dressOverflow)
        XCTAssertGreaterThanOrEqual(c.viewOx - o.left * c.viewScale, -0.5, "the border's left is on the screen")
        XCTAssertLessThanOrEqual(c.viewOx + (CGFloat(l.contentW) + o.right) * c.viewScale, Self.box.width + 0.5,
                                 "and so is its right")
        XCTAssertGreaterThan(o.top, 0)
        XCTAssertGreaterThan(CGFloat(l.contentH) + o.top + o.bottom, Self.box.height, "the page scrolls")
        XCTAssertEqual(c.viewOy, o.top * c.viewScale, accuracy: 0.5, "it opens at the border's top")

        let held = page.dressClamp(c.currentView, viewportW: Float(Self.box.width), viewportH: Float(Self.box.height))
        XCTAssertEqual(CGFloat(held.offsetY), c.viewOy, accuracy: 0.5, "the engine's clamp agrees with the view")
        // towards the top, past the end
        c.pan(CGSize(width: 0, height: 5000)); c.panEnded(.zero, velocity: .zero)
        XCTAssertEqual(c.viewOy, o.top * c.viewScale, accuracy: 0.5, "the border's top stays on the screen")
        // towards the bottom, past the end
        c.pan(CGSize(width: 0, height: -100_000)); c.panEnded(.zero, velocity: .zero)
        XCTAssertEqual(c.viewOy + (CGFloat(l.contentH) + o.bottom) * c.viewScale, Self.box.height, accuracy: 0.5, "and so does its bottom")
    }

    /// A host that scrolls a dressed printed page itself: the canvas draws it at scale 1 with
    /// the border's top and left on the canvas, and `hostPageSize` holds the whole border.
    @MainActor func testCanvasHostScrollsKeepsTheBorderOnTheCanvas() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpPageCanvas needs macOS 14 / iOS 17") }
        let page = try Self.page604(); defer { page.close() }
        let set = try QvpOrnaments(bytes: Self.ornamentBytes)
        let c = QvpCanvasController()
        c.hostScrolls = true
        c.hostViewportHeight = Self.box.height
        c.page = page
        c.setBounds(Self.box, fromCanvas: 1)
        XCTAssertNotNil(page.dress(set, Self.redBorder(set)))
        c.resetView()
        let l = try XCTUnwrap(page.currentLayout)
        XCTAssertFalse(l.reflowed)
        let o = try XCTUnwrap(page.dressOverflow)
        XCTAssertGreaterThan(o.top, 0)
        XCTAssertGreaterThan(o.left, 0)
        // the dressed box is wider than the canvas, so the engine shrinks it to the width
        let s = c.viewScale
        let full = CGFloat(l.contentW) + o.left + o.right
        XCTAssertEqual(s, min(1, Self.box.width / full), accuracy: 0.0001)
        XCTAssertLessThan(s, 1)
        // the border's top is the canvas's top, and its left and right are on the canvas
        XCTAssertEqual(c.viewOy - o.top * s, 0, accuracy: 0.5)
        XCTAssertGreaterThanOrEqual(c.viewOx - o.left * s, -0.5)
        XCTAssertLessThanOrEqual(c.viewOx + (CGFloat(l.contentW) + o.right) * s, Self.box.width + 0.5)
        // the size the host gives the canvas holds the whole border
        let size = try XCTUnwrap(c.hostPageSize)
        XCTAssertEqual(size.height, (CGFloat(l.contentH) + o.top + o.bottom) * s, accuracy: 0.5)
        XCTAssertEqual(size.width, full * s, accuracy: 0.5)
        XCTAssertLessThanOrEqual(size.width, Self.box.width + 0.5)
        // sized so, the border's bottom is on the canvas too
        c.setBounds(CGSize(width: Self.box.width, height: size.height), fromCanvas: 1)
        let after = try XCTUnwrap(page.currentLayout), o2 = try XCTUnwrap(page.dressOverflow)
        let s2 = c.viewScale
        XCTAssertEqual(c.viewOy - o2.top * s2, 0, accuracy: 0.5)
        XCTAssertLessThanOrEqual(c.viewOy + (CGFloat(after.contentH) + o2.bottom) * s2, size.height + 0.5)
        XCTAssertLessThanOrEqual(c.viewOx + (CGFloat(after.contentW) + o2.right) * s2, Self.box.width + 0.5)
        XCTAssertEqual(try XCTUnwrap(c.hostPageSize).height, size.height, accuracy: 0.5, "the box, not the canvas, sizes the page")
    }

    #if canImport(UIKit)
    /// The same scroll through QvpPageView's `scroll(by:)`.
    @MainActor func testPageViewScrollKeepsTheBorder() throws {
        let page = try Self.page604(); defer { page.close() }
        let set = try QvpOrnaments(bytes: Self.ornamentBytes)
        let v = QvpPageView(frame: CGRect(origin: .zero, size: Self.box))
        v.page = page
        v.zoomToStep(v.zoomSteps.count)
        XCTAssertTrue(try XCTUnwrap(page.currentLayout).reflowed)
        XCTAssertNotNil(page.dress(set, Self.redBorder(set)))
        v.resetView()
        // the dress lays the reflowed page out again, with rows narrow enough for the border
        let l = try XCTUnwrap(page.currentLayout)
        XCTAssertTrue(l.reflowed)
        let o = try XCTUnwrap(page.dressOverflow)
        XCTAssertGreaterThan(CGFloat(l.contentH) + o.top + o.bottom, Self.box.height, "the page scrolls")
        XCTAssertEqual(v.viewOy, o.top * v.viewScale, accuracy: 0.5, "it opens at the border's top")
        v.scroll(by: 5000)
        XCTAssertGreaterThanOrEqual(v.viewOy, o.top * v.viewScale - 0.5, "the border's top stays on the screen")
        XCTAssertEqual(v.viewOy, o.top * v.viewScale, accuracy: 0.5)
        v.scroll(by: -100_000)
        XCTAssertEqual(v.viewOy + (CGFloat(l.contentH) + o.bottom) * v.viewScale, Self.box.height, accuracy: 0.5, "and so does its bottom")
    }
    #endif

    /// `dressView` at scale 1: the size is content plus overflow, and the border's top is at 0.
    func testDressViewHoldsTheBorder() throws {
        let page = try Self.page604(); defer { page.close() }
        let set = try QvpOrnaments(bytes: Self.ornamentBytes)
        _ = page.layout(QvpLayoutSpec(viewportW: Float(Self.box.width), viewportH: Float(Self.box.height)))
        XCTAssertNotNil(page.dress(set, Self.redBorder(set)))
        let l = try XCTUnwrap(page.currentLayout), o = try XCTUnwrap(page.dressInfo()).overflow
        let d = page.dressView(1, viewportW: Float(Self.box.width))
        XCTAssertEqual(d.width, l.contentW + o[0] + o[2], accuracy: 0.001)
        XCTAssertEqual(d.height, l.contentH + o[1] + o[3], accuracy: 0.001)
        XCTAssertEqual(d.view.offsetY - d.view.scale * o[1], 0, accuracy: 0.001)
        // scale 0 fits the dressed box to the width, and the whole border is inside it
        let f = page.dressView(0, viewportW: Float(Self.box.width))
        XCTAssertEqual(f.view.scale, min(1, Float(Self.box.width) / d.width), accuracy: 0.0001)
        XCTAssertLessThanOrEqual(f.width, Float(Self.box.width) + 0.5)
        XCTAssertGreaterThanOrEqual(f.view.offsetX - f.view.scale * o[0], -0.5)
        XCTAssertLessThanOrEqual(f.view.offsetX + f.view.scale * (l.contentW + o[2]), Float(Self.box.width) + 0.5)
    }

    /// An undressed page gets the plain clamp from `dressClamp`.
    func testDressClampOnABarePageIsTheViewClamp() throws {
        let page = try Self.page604(); defer { page.close() }
        let l = page.layout(QvpLayoutSpec(viewportW: Float(Self.box.width), viewportH: Float(Self.box.height)))
        let v = QvpView(scale: 1, offsetX: 500, offsetY: -9000)
        let a = page.dressClamp(v, viewportW: Float(Self.box.width), viewportH: Float(Self.box.height))
        let b = page.viewClamp(v, contentW: l.contentW, contentH: l.contentH, viewportW: Float(Self.box.width), viewportH: Float(Self.box.height))
        XCTAssertEqual(a.offsetX, b.offsetX, accuracy: 0.001)
        XCTAssertEqual(a.offsetY, b.offsetY, accuracy: 0.001)
    }
}
