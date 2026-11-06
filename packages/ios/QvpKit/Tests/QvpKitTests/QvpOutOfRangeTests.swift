// An index or count out of range reaches the engine as the C ABI reads it: an unsigned value of
// the parameter's width, cut from the caller's Int. The engine answers ("", nil, false, 0). The
// wrapper never stops the program, as JNI's casts never do on Android.
import XCTest
import QvpFFI
@testable import QvpKit

final class QvpOutOfRangeTests: XCTestCase {
    /// Both cut to the widest unsigned value of every parameter, which no page or atlas holds.
    static let bad = [-1, Int.max]
    /// These cut to small values, which a page does hold: the wrapper passes what the C ABI reads.
    static let wide = [Int(UInt32.max) + 5, Int.min, Int.min + 3]
    var page: QvpPage!
    var atlas: QvpAtlas!

    override func setUpWithError() throws {
        page = try QvpPage(bytes: Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp")))
        atlas = try QvpAtlas(bytes: Data(contentsOf: QvpKitTests.pages.appendingPathComponent("atlas.qva")))
    }
    override func tearDown() { page.close(); atlas.close() }

    private func u32(_ i: Int) -> UInt32 { UInt32(truncatingIfNeeded: i) }
    private func u16(_ i: Int) -> UInt16 { UInt16(truncatingIfNeeded: i) }

    func testPageCallsGiveTheEngineAnswer() throws {
        let p = page!
        let twin = try QvpPage(bytes: Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp")))
        defer { twin.close() }
        for i in Self.bad {
            var s = QvpStr()
            qvp_word_label(p.p, u32(i), &s)
            XCTAssertEqual(p.wordLabel(i), s.string)
            var t = QvpStr()
            qvp_ayah_label(p.p, u32(i), &t)
            XCTAssertEqual(p.ayahLabel(i), t.string)
            XCTAssertEqual(p.wordForm(i), "")
            XCTAssertNil(p.wordBoundsView(i))
            XCTAssertEqual(p.findWord(i, i, i), Int(qvp_find_word(p.p, u16(i), u16(i), u16(i))))
            XCTAssertEqual(p.revealStepOf(i), Int(qvp_reveal_step_of(p.p, u32(i))))
            // these change the mask, so a twin page takes the C call
            XCTAssertEqual(p.unmaskWord(i), qvp_unmask_word(twin.p, u32(i)) != 0)
            XCTAssertEqual(p.maskWord(i), qvp_mask_word(twin.p, u32(i)) != 0)
            XCTAssertEqual(p.targetWords(.words([i])), [])
            XCTAssertNil(p.cropBounds(.word(i)))
            _ = p.reciteMap(i, i, nSegments: i)
            _ = p.ayahWordCount(i, i)
        }
    }

    func testAWideIntReachesTheEngineCut() {
        let p = page!
        for i in Self.wide {
            let c = Int(UInt32(truncatingIfNeeded: i))
            XCTAssertEqual(p.wordLabel(i), p.wordLabel(c))
            XCTAssertEqual(p.wordForm(i), p.wordForm(c))
            XCTAssertEqual(p.wordBoundsView(i).map { [$0.x0, $0.y0, $0.x1, $0.y1] }, p.wordBoundsView(c).map { [$0.x0, $0.y0, $0.x1, $0.y1] })
            XCTAssertEqual(p.targetWords(.words([i])), p.targetWords(.words([c])))
            XCTAssertEqual(p.cropBounds(.word(i)), p.cropBounds(.word(c)))
        }
    }

    func testCountsReachTheEngineUnsigned() throws {
        // A twin page takes the C call, so both start from the same mask.
        let twin = try QvpPage(bytes: Data(contentsOf: QvpKitTests.pages.appendingPathComponent("042.qvp")))
        defer { twin.close() }
        for p in [page!, twin] { p.mask(.page()) }
        XCTAssertEqual(page.unmaskNext(-1), Int(qvp_unmask_next(twin.p, UInt32.max)))
        XCTAssertEqual(page.maskBack(-1), Int(qvp_mask_back(twin.p, UInt32.max)))
        XCTAssertEqual(page.maskWords(), twin.maskWords())
    }

    /// A key whose numbers are not ASCII digits that fit an Int32 is no key: `Target(parsing:)`
    /// is nil, as Kotlin's `Target.parse` throws for it, and a page call with it never stops.
    func testATargetKeyWithAnOddNumberIsNoKey() {
        let huge = "99999999999999999999"
        let keys = ["line:\(huge)", "surah:\(huge)", "\(huge):1", "2:\(huge)", "2:255-\(huge)", "2:\(huge):3", "line:2147483648",
                    "line:٧", "surah:٢", "٢:٢٥٥", "2:٢٥٥", "٢:٢٥٥-٢٥٧", "2:255:٣", "١:١:١", "page ", "pagé", "line:-1", "line:+7", "line:"]
        for k in keys {
            XCTAssertNil(Target(parsing: k, page: page), k)
            XCTAssertNil(Target(parsing: k), k)
        }
        // the same forms with ASCII digits that fit still parse
        for k in ["page", "line:7", "surah:2", "2:255", "2:255-257", "line:2147483647"] { XCTAssertNotNil(Target(parsing: k), k) }
        XCTAssertNotNil(Target(parsing: "42:1:1", page: page))
    }

    func testAtlasCallsGiveTheEngineAnswer() {
        let a = atlas!
        for i in Self.bad {
            XCTAssertEqual(a.pageOf(i, i), { let v = qvp_atlas_page_of(a.p, u16(i), u16(i)); return v < 0 ? nil : Int(v) }())
            _ = a.pageRange(i)
            _ = a.surah(i)
            _ = a.pageOfSurah(i)
            _ = a.juz(i)
            _ = a.divisionOf(.hizb, i, i)
            _ = a.pagesOfJuz(i)
        }
    }
}
