// A closed page, a second close and an array larger than the first buffer: the same three
// rules as every other wrapper (docs/API.md, conventions). Data: <repo>/dist/pages/042.qvp.
import Foundation
import XCTest
@testable import QvpKit
#if compiler(>=6.2) && canImport(Testing) && os(macOS)
import Testing
#endif

private func page042() throws -> Data {
    try Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp"))
}

final class QvpLifetimeTests: XCTestCase {
    func testASecondCloseDoesNothing() throws {
        let page = try QvpPage(bytes: try page042())
        let passage = try QvpPassage(pages: [page], surah: 2, from: 255)
        page.close()
        page.close()
        XCTAssertFalse(page.isOpen)
        passage.close()
        passage.close()
        XCTAssertFalse(passage.isOpen)
        let atlas = try QvpAtlas(bytes: try Data(contentsOf: QvpKitTests.pages.appendingPathComponent("atlas.qva")))
        atlas.close()
        atlas.close()
    }

    /// 1,100 highlights: more handles than the 64 the first buffer holds, and more than the
    /// 1,024 another wrapper once stopped at. The wrapper asks again with room for the total.
    func testArrayResultsGrowToTheTotal() throws {
        let page = try QvpPage(bytes: try page042())
        defer { page.close() }
        let added = (0..<1100).map { page.highlight(.word($0 % page.nWords)) }
        XCTAssertEqual(page.highlightHandles(), added)
        XCTAssertEqual(page.hitAreas().count, page.nWords)
    }
}

#if compiler(>=6.2) && canImport(Testing) && os(macOS)
/// A call on a closed object stops the program with a fatal error that names it. Each case runs
/// in a child process, which must exit with that failure.
struct QvpClosedObjectTests {
    /// The child's standard error, so a child that failed for another reason (no page data,
    /// say) does not pass.
    private func says(_ result: ExitTest.Result?, _ message: String) -> Bool {
        String(decoding: result?.standardErrorContent ?? [], as: UTF8.self).contains(message)
    }

    @Test func aCallOnAClosedPageIsAFatalError() async {
        let result = await #expect(processExitsWith: .failure, observing: [\.standardErrorContent]) {
            let page = try QvpPage(bytes: try page042())
            page.close()
            _ = page.hitAreas()
        }
        #expect(says(result, "QvpPage is closed"))
    }

    @Test func aCallOnAClosedPassageIsAFatalError() async {
        let result = await #expect(processExitsWith: .failure, observing: [\.standardErrorContent]) {
            let page = try QvpPage(bytes: try page042())
            let passage = try QvpPassage(pages: [page], surah: 2, from: 255)
            passage.close()
            _ = passage.layout(QvpPassageSpec(width: 360))
        }
        #expect(says(result, "QvpPassage is closed"))
    }

    @Test func aCallOnAClosedOrnamentSetIsAFatalError() async {
        let result = await #expect(processExitsWith: .failure, observing: [\.standardErrorContent]) {
            let set = try QvpOrnaments(bytes: QvpDressTests.ornamentBytes)
            set.close()
            _ = set.find(.pageFrame, "test")
        }
        #expect(says(result, "QvpOrnaments is closed"))
    }

    @Test func aDressWithAClosedOrnamentSetIsAFatalError() async {
        let result = await #expect(processExitsWith: .failure, observing: [\.standardErrorContent]) {
            let page = try QvpPage(bytes: try page042())
            let set = try QvpOrnaments(bytes: QvpDressTests.ornamentBytes)
            set.close()
            _ = page.dress(set, DressSpec(ayahMark: .index(0)))
        }
        #expect(says(result, "QvpOrnaments is closed"))
    }
}
#endif
