// Dress over the Swift wrapper: the same checks as `dress_over_the_abi` in
// crates/qvp-ffi/tests/abi.rs. Data: <repo>/dist/pages/604.qvp.
import XCTest
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
}
