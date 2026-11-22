// The letter layer over the Swift wrapper: the replay of conformance/scenarios/letters.json, which
// every wrapper replays, and one letter attached, coloured and read back. The fixtures are the
// synthetic hex files beside the scenario, so these tests need no page data.
// Run: `swift test` (macOS slice) after scripts/build-engine-ios.sh.
import XCTest
import CoreGraphics
@testable import QvpKit

final class QvpLettersTests: XCTestCase {
    static let conformance = QvpKitTests.repo.appendingPathComponent("conformance")

    // ── reading the scenario ──
    private func scenario() throws -> [String: Any] {
        let url = Self.conformance.appendingPathComponent("scenarios/letters.json")
        return try JSONSerialization.jsonObject(with: Data(contentsOf: url)) as! [String: Any]
    }
    /// The bytes of a hex file in conformance/, whitespace ignored.
    private func hexBytes(_ text: String) -> Data {
        let chars = Array(text.filter { !$0.isWhitespace })
        return Data(stride(from: 0, to: chars.count, by: 2).map { UInt8(String(chars[$0...$0 + 1]), radix: 16)! })
    }
    private func fixtureBytes(_ file: String) throws -> Data {
        hexBytes(try String(contentsOf: Self.conformance.appendingPathComponent(file), encoding: .utf8))
    }
    private func fixture(_ j: [String: Any], _ name: String) -> [String: Any] {
        (j["fixtures"] as! [String: Any])[name] as! [String: Any]
    }
    private func int(_ v: Any?) -> Int { (v as! NSNumber).intValue }
    private func u32(_ v: Any?) -> UInt32 { UInt32(truncatingIfNeeded: (v as! NSNumber).int64Value) }
    private func float(_ v: Any?) -> Float { Float((v as! NSNumber).doubleValue) }
    private func ints(_ v: Any?) -> [Int] { (v as! [NSNumber]).map(\.intValue) }
    private func pairs(_ v: Any?) -> [[Int]] { (v as! [[NSNumber]]).map { $0.map(\.intValue) } }
    private func close(_ got: Float, _ want: Any?, _ tolerance: Double, _ what: String) {
        let w = (want as! NSNumber).doubleValue
        XCTAssertLessThanOrEqual(abs(Double(got) - w), tolerance * max(1, abs(w)), "\(what): got \(got), engine says \(w)")
    }
    private func spec(_ j: [String: Any], _ name: String) -> QvpLayoutSpec {
        let s = (j["layouts"] as! [String: Any])[name] as! [String: Any]
        let f: (String) -> Float = { Float((s[$0] as! NSNumber).doubleValue) }
        let reflow = (s["reflow"] as? [String: Any]).map { QvpReflowSpec(zoom: Float(($0["zoom"] as! NSNumber).doubleValue)) }
        return QvpLayoutSpec(viewportW: f("viewportW"), viewportH: f("viewportH"), padTop: f("padTop"), padBottom: f("padBottom"),
                             padLeft: f("padLeft"), padRight: f("padRight"), reflow: reflow)
    }
    private func selector(_ v: Any?) -> QvpKit.Selector {
        let s = v as! [String: Any]
        return QvpKit.Selector(int(s["selector"]), int(s["a"]), int(s["b"]), int(s["c"]))
    }
    private func query(_ v: Any?) -> QvpLetterQuery {
        let q = v as! [String: Any], s = q["scope"] as! [String: Any]
        let words = (s["words"] as! [NSNumber]).map { UInt32(truncatingIfNeeded: $0.int64Value) }
        let scope = Target(int(s["target"]), int(s["a"]), int(s["b"]), int(s["c"]), words)
        return QvpLetterQuery(scope: scope, codePoint: u32(q["text"]), mark: int(q["mark"]), part: int(q["part"]))
    }
    private func options(_ v: Any?) -> QvpHitOptions {
        // null passes no options: the engine's defaults, which are what QvpHitOptions() holds
        guard let o = v as? [String: Any] else { return QvpHitOptions() }
        return QvpHitOptions(maxDistance: float(o["maxDistance"]), gapBias: float(o["gapBias"]), preferExact: o["preferExact"] as! Bool)
    }
    /// A fresh page for one case: the fixture's page, its layer, the words sidecar when asked,
    /// the layout when named, then a tick at 0.
    private func start(_ j: [String: Any], _ c: [String: Any], layout: String?) throws -> QvpPage {
        let fx = fixture(j, c["fixture"] as! String)
        let p = try QvpPage(bytes: fixtureBytes(fx["page"] as! String))
        XCTAssertTrue(p.attachLetters(try fixtureBytes(fx["letters"] as! String)), "\(c["name"]!): attach")
        if c["words"] as? Bool ?? false { XCTAssertGreaterThanOrEqual(p.attachWords(j["words"] as! String), 0) }
        if let layout { p.layout(spec(j, layout)) }
        _ = p.tick(0)
        return p
    }
    private func red(_ p: QvpPage, _ rgba: UInt32) -> [Int] { p.colors().enumerated().filter { $0.element == rgba }.map(\.offset) }
    private func drawList(_ p: QvpPage) -> [[Int]] { p.layoutDrawList().map { [$0.path, $0.placement] } }

    // ── comparing answers ──
    private func sameMembers(_ got: [QvpLetterMember], _ want: Any?, _ tag: String) {
        let w = want as! [[String: Any]]
        XCTAssertEqual(got.count, w.count, "\(tag) members")
        for (m, e) in zip(got, w) {
            XCTAssertEqual([m.path, m.at], [int(e["path"]), int(e["at"])], "\(tag) member")
            XCTAssertEqual(m.markName, e["mark"] as! String, "\(tag) member mark")
            XCTAssertEqual([m.isBody, m.isCut], [e["isBody"] as! Bool, e["isCut"] as! Bool], "\(tag) member roles")
        }
    }
    private func sameLetter(_ got: QvpLetter?, _ want: Any?, _ tolerance: Double, _ tag: String) {
        guard let w = want as? [String: Any] else { return XCTAssertNil(got, "\(tag): got a letter, engine says none") }
        guard let got else { return XCTFail("\(tag): got no letter") }
        XCTAssertEqual([got.word, got.index, got.at, Int(got.codePoint), got.nMembers, got.nMarks, got.moreAt, got.nMoreAt],
                       [int(w["word"]), int(w["index"]), int(w["at"]), int(w["text"]), int(w["nMembers"]), int(w["nMarks"]), int(w["moreAt"]), int(w["nMoreAt"])], tag)
        XCTAssertEqual(got.text, w["char"] as! String, "\(tag) text")
        XCTAssertEqual(got.hasInk, w["hasInk"] as! Bool, "\(tag) hasInk")
        close(got.x0, w["x0"], tolerance, "\(tag) x0"); close(got.y0, w["y0"], tolerance, "\(tag) y0")
        close(got.x1, w["x1"], tolerance, "\(tag) x1"); close(got.y1, w["y1"], tolerance, "\(tag) y1")
        if let members = w["members"] {
            sameMembers(got.members, members, tag)
            XCTAssertEqual(got.marks, got.members.filter { !$0.isBody }, "\(tag) marks")
        }
    }
    private func sameHit(_ got: QvpLetterHit?, _ want: Any?, _ tolerance: Double, _ tag: String) {
        guard let w = want as? [String: Any] else { return XCTAssertNil(got, "\(tag): got a hit, engine says none") }
        guard let got else { return XCTFail("\(tag): got no hit") }
        XCTAssertEqual([got.word, got.letter, got.path, got.at, got.line],
                       [int(w["word"]), int(w["letter"]), int(w["path"]), int(w["at"]), int(w["line"])], tag)
        close(got.distance, w["distance"], tolerance, "\(tag) distance")
        XCTAssertEqual(got.markName, w["mark"] as! String, "\(tag) mark")
        XCTAssertEqual([got.isExact, got.isBody], [w["isExact"] as! Bool, w["isBody"] as! Bool], "\(tag) isExact, isBody")
    }
    private func samePathLetter(_ got: (letter: Int, at: Int)?, _ want: Any?, _ tag: String) {
        guard let w = want as? [String: Any] else { return XCTAssertNil(got.map { [$0.letter, $0.at] }, tag) }
        XCTAssertEqual(got.map { [$0.letter, $0.at] }, [int(w["letter"]), int(w["at"])], tag)
    }

    // ── the replay ──
    /// `letters`: what a page reads back once its layer is attached.
    func testLetterScenariosMatchTheEngine() throws {
        let j = try scenario(), tolerance = j["tolerance"] as! Double
        let cases = j["letters"] as! [[String: Any]]
        for c in cases {
            let tag = "letters \(c["name"]!)"
            let p = try start(j, c, layout: nil); defer { p.close() }
            XCTAssertEqual(p.hasLetters, c["hasLetters"] as! Bool, tag)
            let n = int(c["nPaths"])
            XCTAssertEqual([p.nPaths, p.table.count / 8, p.colors().count], [n, n, n], "\(tag) nPaths")
            for (w, count) in ints(c["counts"]).enumerated() { XCTAssertEqual(p.letterCount(w), count, "\(tag) count of word \(w)") }
            for l in c["letters"] as! [[String: Any]] {
                let (w, i) = (int(l["word"]), int(l["index"]))
                sameLetter(p.letter(w, i), l, tolerance, "\(tag) letter \(w).\(i)")
                sameMembers(p.letterMembers(w, i), l["members"], "\(tag) letterMembers \(w).\(i)")
            }
            for m in c["missing"] as! [[String: Any]] {
                let (w, i) = (int(m["word"]), int(m["letter"]))
                sameLetter(p.letter(w, i), m["info"], tolerance, "\(tag) missing \(w).\(i)")
                sameMembers(p.letterMembers(w, i), m["members"], "\(tag) missing \(w).\(i)")
            }
            for e in c["pathLetters"] as! [[String: Any]] {
                samePathLetter(p.pathLetter(int(e["path"])), e["value"], "\(tag) pathLetter \(e["path"]!)")
            }
            for o in c["offsets"] as! [[Any]] {
                let want = o[2] as? NSNumber
                XCTAssertEqual(p.letterAtOffset(int(o[0]), int(o[1])), want?.intValue, "\(tag) letterAtOffset \(o[0]), \(o[1])")
            }
        }
        XCTAssertEqual(cases.count, 4)
    }

    /// `selections`: each letter selector, coloured on the printed page.
    func testLetterSelectionScenariosMatchTheEngine() throws {
        let j = try scenario(), rgba = u32(j["red"])
        let cases = j["selections"] as! [[String: Any]]
        for c in cases {
            let tag = "selection \(c["name"]!)"
            let p = try start(j, c, layout: "printed"); defer { p.close() }
            XCTAssertEqual(p.style(selector(c["selector"]), rgba) != 0, c["handle"] as! Bool, "\(tag) handle")
            _ = p.tick(0)
            XCTAssertEqual(red(p, rgba), ints(c["red"]), "\(tag) red")
            XCTAssertEqual(p.layoutDrawRevision(), int(c["drawRevision"]), "\(tag) drawRevision")
            XCTAssertEqual(drawList(p), pairs(c["drawList"]), "\(tag) drawList")
        }
        XCTAssertEqual(cases.count, 42)
    }

    /// `queries`: the letters a query finds, and the rule it adds.
    func testLetterQueryScenariosMatchTheEngine() throws {
        let j = try scenario(), rgba = u32(j["red"])
        let cases = j["queries"] as! [[String: Any]]
        for c in cases {
            let tag = "query \(c["name"]!)"
            let p = try start(j, c, layout: "printed"); defer { p.close() }
            let q = query(c["query"])
            XCTAssertEqual(p.findLetters(q).map { [$0.word, $0.letter] }, pairs(c["found"]), "\(tag) found")
            XCTAssertEqual(p.styleLetters(q, rgba) != 0, c["handle"] as! Bool, "\(tag) handle")
            _ = p.tick(0)
            XCTAssertEqual(red(p, rgba), ints(c["red"]), "\(tag) red")
            XCTAssertEqual(drawList(p), pairs(c["drawList"]), "\(tag) drawList")
        }
        XCTAssertEqual(cases.count, 15)
    }

    /// `hits`: the letter under a point, in page units and in viewport px.
    func testLetterHitScenariosMatchTheEngine() throws {
        let j = try scenario(), tolerance = j["tolerance"] as! Double
        let cases = j["hits"] as! [[String: Any]]
        for c in cases {
            let tag = "hit \(c["name"]!)"
            let p = try start(j, c, layout: c["layout"] as? String); defer { p.close() }
            let (x, y, o) = (float(c["x"]), float(c["y"]), options(c["options"]))
            let got = c["units"] as! String == "view" ? p.hitTestLetterView(x, y, o) : p.hitTestLetter(x, y, o)
            sameHit(got, c["hit"], tolerance, tag)
        }
        XCTAssertEqual(cases.count, 34)
    }

    /// `sequences`: ordered calls on a page that starts with no layer. The order is the test:
    /// some steps read the draw list right after a rule change, with no tick, on purpose.
    func testLetterSequenceScenariosMatchTheEngine() throws {
        let j = try scenario()
        let cases = j["sequences"] as! [[String: Any]]
        for c in cases {
            let fx = fixture(j, c["fixture"] as! String)
            let p = try QvpPage(bytes: fixtureBytes(fx["page"] as! String)); defer { p.close() }
            var handles: [String: Int] = [:]
            for (k, step) in (c["steps"] as! [[String: Any]]).enumerated() {
                try run(step, on: p, j, &handles, "sequence \(c["name"]!) step \(k) \(step["call"]!)")
            }
        }
        XCTAssertEqual(cases.count, 11)
    }

    /// One step of a sequence: the call it names, and its answer compared.
    private func run(_ s: [String: Any], on p: QvpPage, _ j: [String: Any], _ handles: inout [String: Int], _ tag: String) throws {
        let tolerance = j["tolerance"] as! Double
        let value = s["value"]
        switch s["call"] as! String {
        case "tick": XCTAssertEqual(p.tick((s["ms"] as! NSNumber).doubleValue), s["animating"] as! Bool, tag)
        case "attach":
            let bytes = try fixtureBytes(fixture(j, s["letters"] as! String)["letters"] as! String)
            XCTAssertEqual(p.attachLetters(bytes) ? 0 : -1, int(s["result"]), tag)
        case "attachBytes": XCTAssertEqual(p.attachLetters(hexBytes(s["hex"] as! String)) ? 0 : -1, int(s["result"]), tag)
        case "attachWords": XCTAssertEqual(p.attachWords(j["words"] as! String), int(s["result"]), tag)
        case "layout": p.layout(spec(j, s["spec"] as! String))
        case "style":
            let h = p.style(selector(s["selector"]), u32(s["rgba"]), transitionMs: int(s["ms"]))
            handles[s["as"] as! String] = h
            XCTAssertEqual(h != 0, s["handle"] as! Bool, tag)
        case "styleLetters":
            let h = p.styleLetters(query(s["query"]), u32(s["rgba"]), transitionMs: int(s["ms"]))
            handles[s["as"] as! String] = h
            XCTAssertEqual(h != 0, s["handle"] as! Bool, tag)
        case "remove": XCTAssertEqual(p.removeStyle(handles[s["handle"] as! String]!), int(s["result"]), tag)
        case "theme": p.theme(QvpTheme(ink: u32(s["ink"]), dots: u32(s["dots"]), transitionMs: int(s["ms"])))
        case "highlight":
            handles[s["as"] as! String] = p.highlight(Target.word(int(s["word"])), QvpHighlightStyle(mode: .ink, ink: u32(s["ink"]), transitionMs: int(s["ms"])))
        case "removeHighlight": XCTAssertEqual(p.removeHighlight(handles[s["handle"] as! String]!), s["result"] as! Bool, tag)
        case "maskTransition": p.maskTransition(int(s["ms"]))
        case "mask": p.mask(Target.word(int(s["word"])))
        case "unmask": p.unmask()
        case "hasLetters": XCTAssertEqual(p.hasLetters, value as! Bool, tag)
        // the geometry only: reading the colours here would apply a pending rule
        case "nPaths": XCTAssertEqual([p.nPaths, p.table.count / 8], [int(value), int(value)], tag)
        case "rows":
            let (first, count) = (int(s["first"]), int(s["count"]))
            let rows = (first..<first + count).map { i in (0..<8).map { Int(p.table[i * 8 + $0]) } }
            XCTAssertEqual(rows, pairs(value), tag)
        case "colors": XCTAssertEqual(p.colors(), (value as! [NSNumber]).map { UInt32(truncatingIfNeeded: $0.int64Value) }, tag)
        case "styledPaths":
            XCTAssertEqual(p.styledPaths().map { [Int64($0.path), Int64($0.color)] }, (value as! [[NSNumber]]).map { $0.map(\.int64Value) }, tag)
        case "drawRevision": XCTAssertEqual(p.layoutDrawRevision(), int(value), tag)
        case "drawList": XCTAssertEqual(drawList(p), pairs(value), tag)
        case "letterCount": XCTAssertEqual(p.letterCount(int(s["word"])), int(value), tag)
        case "letterInfo": sameLetter(p.letter(int(s["word"]), int(s["letter"])), value, tolerance, tag)
        case "letterMembers": sameMembers(p.letterMembers(int(s["word"]), int(s["letter"])), value, tag)
        case "pathLetter": samePathLetter(p.pathLetter(int(s["path"])), value, tag)
        case "letterAtOffset": XCTAssertEqual(p.letterAtOffset(int(s["word"]), int(s["at"])), (value as? NSNumber)?.intValue, tag)
        case "findLetters": XCTAssertEqual(p.findLetters(query(s["query"])).map { [$0.word, $0.letter] }, pairs(value), tag)
        case "hitTestLetter": sameHit(p.hitTestLetter(float(s["x"]), float(s["y"]), options(s["options"])), value, tolerance, tag)
        case "hitTestLetterView": sameHit(p.hitTestLetterView(float(s["x"]), float(s["y"]), options(s["options"])), value, tolerance, tag)
        case let other: XCTFail("\(tag): no step \(other)")
        }
    }

    // ── one letter, end to end ──
    /// Attach the layer, colour the lam of word 0, and read it back: the lam's piece is red, and
    /// the joined outline draws as its two pieces.
    func testAttachColourOneLetterAndReadItBack() throws {
        let j = try scenario(), rgba = u32(j["red"])
        let fx = fixture(j, "letters")
        let p = try QvpPage(bytes: fixtureBytes(fx["page"] as! String)); defer { p.close() }
        XCTAssertFalse(p.hasLetters)
        XCTAssertEqual(p.nPaths, 8)
        XCTAssertEqual(p.buildPaths().count, 8)
        XCTAssertFalse(p.attachLetters(Data([0x51, 0x56, 0x4c, 0x31])), "a bad file is refused")
        XCTAssertEqual(p.nPaths, 8, "a refused attach changes nothing")
        XCTAssertTrue(p.attachLetters(try fixtureBytes(fx["letters"] as! String)))
        XCTAssertTrue(p.hasLetters)
        XCTAssertEqual(p.nPaths, 10)
        XCTAssertEqual(p.table.count, 80)
        XCTAssertEqual(p.buildPaths().count, 10, "the paths are built again with the pieces")
        let lam = try XCTUnwrap(p.letter(0, 1))
        XCTAssertEqual(lam.text, "ل")
        XCTAssertEqual(lam.codePoint, 0x644)
        XCTAssertEqual(lam.members.map(\.path), [9])
        XCTAssertEqual(p.letters(0).count, 2)
        p.layout(spec(j, "printed"))
        _ = p.tick(0)
        let before = p.layoutDrawRevision()
        XCTAssertTrue(drawList(p).contains([0, 0]), "one colour: the joined outline draws whole")
        XCTAssertNotEqual(p.style(.letter(0, 1), rgba), 0)
        _ = p.tick(0)
        XCTAssertEqual(p.colors()[9], rgba)
        XCTAssertEqual(p.colorOf(9), rgba)
        XCTAssertTrue(p.styledPaths().contains { $0.path == 9 && $0.color == rgba })
        XCTAssertNotEqual(p.layoutDrawRevision(), before, "the swap counts up the revision")
        let paths = drawList(p).map { $0[0] }
        XCTAssertTrue(paths.contains(8) && paths.contains(9), "the outline draws as its pieces")
        XCTAssertFalse(paths.contains(0), "and never with them")
        XCTAssertEqual(p.findLetters(QvpLetterQuery("ل")).map { [$0.word, $0.letter] }, [[0, 1], [1, 0]])
        XCTAssertEqual(p.findLetters(QvpLetterQuery(text: "ل")).map { [$0.word, $0.letter] }, [[0, 1], [1, 0]])
        XCTAssertEqual(QvpLetterQuery(text: "ل").codePoint, 0x644)
        XCTAssertEqual(QvpLetterQuery(codePoint: 0x644).text, "ل")
        XCTAssertEqual(QvpLetterQuery(text: "").codePoint, 0)
        XCTAssertEqual(QvpLetterQuery().text, "")
        let hit = try XCTUnwrap(p.hitTestLetter(45, 15))
        XCTAssertEqual([hit.word, hit.letter, hit.path], [0, 1, 9])
        XCTAssertTrue(hit.isExact)
    }

    /// The canvas keeps a draw list per band. With a letter layer it is used only at the same
    /// draw revision, so an attach and a swap give the new list.
    @MainActor func testCanvasDrawListFollowsTheDrawRevision() throws {
        guard #available(macOS 14.0, iOS 17.0, *) else { throw XCTSkip("QvpCanvasController needs macOS 14 / iOS 17") }
        let j = try scenario(), rgba = u32(j["red"])
        let fx = fixture(j, "letters")
        let p = try QvpPage(bytes: fixtureBytes(fx["page"] as! String)); defer { p.close() }
        let c = QvpCanvasController()
        c.page = p; c.setBounds(CGSize(width: 200, height: 200), fromCanvas: 1)
        let band = c.layoutBand(top: 0, height: 200)
        let drawn = { () throws -> [Int] in try XCTUnwrap(c.drawListNow(band: band)).draws.map(\.path) }
        XCTAssertTrue(try drawn().contains(0))
        XCTAssertTrue(p.attachLetters(try fixtureBytes(fx["letters"] as! String)))
        XCTAssertTrue(try drawn().contains(0), "the attach is read again; one colour draws the outline whole")
        p.style(.letter(0, 0), rgba); c.invalidate()
        let after = try drawn()
        XCTAssertTrue(after.contains(8) && after.contains(9), "the swap gives a new list for the same band")
        XCTAssertFalse(after.contains(0))
    }
}
