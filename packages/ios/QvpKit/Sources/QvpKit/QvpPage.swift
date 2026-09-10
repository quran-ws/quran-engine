// One loaded page and the cross-page atlas, over the C ABI in qvp.h.
// Geometry is copied out of the engine once; everything that decides (hit-testing, layout,
// styles, highlights, masks, search) stays inside the engine. Mirrors web/qvp.js and the
// Kotlin QvpPage — see docs/API.md.
import Foundation
import CoreGraphics
import QvpFFI

public enum QvpError: Error { case badPage, badAtlas, badOrnaments }

public final class QvpPage {
    private var h: OpaquePointer?

    public let width: Float, height: Float, pageNo: Int
    public let nLines: Int, nAyahs: Int, nWords: Int, nPaths: Int, nDecos: Int
    /// path ops: 0 MoveTo 1 LineTo 2 QuadTo 3 CubicTo 4 Close
    public let ops: [UInt8]
    /// x,y pairs consumed in order by the ops
    public let pts: [Float]
    /// stride 8 per path: opStart, opCount, ptStart, ptCount, flags, word, line, extra
    public let table: [UInt32]
    public let words: [QvpWord], ayahs: [QvpAyah], lines: [QvpLine], decos: [QvpDecoration]
    public let naturalPitch: Float
    public private(set) var currentLayout: QvpLayout?
    public private(set) var defaultInk: UInt32 = 0x231f20ff
    private var paths: [CGPath]?
    private var dressed: [QvpOrnamentDraw]?
    private var dressedRevision: UInt32 = 0

    /// Load a `NNN.qvp` page; bytes are copied by the engine.
    public init(bytes: Data) throws {
        h = bytes.withUnsafeBytes { qvp_page_load($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        guard let h else { throw QvpError.badPage }
        var info = QvpPageInfo(); qvp_page_info(h, &info)
        width = info.width; height = info.height; pageNo = Int(info.page)
        nLines = Int(info.n_lines); nAyahs = Int(info.n_ayahs); nWords = Int(info.n_words); nPaths = Int(info.n_paths); nDecos = Int(info.n_decos)
        var g = QvpGeometry(); qvp_geometry(h, &g)
        ops = Array(UnsafeBufferPointer(start: g.ops, count: Int(g.ops_len)))
        pts = Array(UnsafeBufferPointer(start: g.pts, count: Int(g.pts_len)))
        table = Array(UnsafeBufferPointer(start: g.table, count: Int(g.n_paths) * 8))
        words = (0..<Int(info.n_words)).map { k in
            var w = QvpWordInfo(); _ = qvp_word_info(h, UInt32(k), &w)
            return QvpWord(idx: k, surah: Int(w.surah), ayah: Int(w.ayah), word: Int(w.word), line: Int(w.line_no), ayahIdx: Int(w.ayah_idx), lineIdx: Int(w.line_idx),
                           x0: w.x0, y0: w.y0, x1: w.x1, y1: w.y1, text: w.text.string, firstPath: Int(w.first_path), nPaths: Int(w.n_paths))
        }
        ayahs = (0..<Int(info.n_ayahs)).map { k in
            var a = QvpAyahInfo(); _ = qvp_ayah_info(h, UInt32(k), &a)
            return QvpAyah(idx: k, surah: Int(a.surah), ayah: Int(a.ayah), fragment: Int(a.fragment), fragments: Int(a.fragments), flags: Int(a.flags), rubuAlHizb: Int(a.rubu_al_hizb), firstWord: Int(a.first_word), nWords: Int(a.n_words),
                           ayahMarkDeco: idx(a.ayah_mark_deco), x0: a.x0, y0: a.y0, x1: a.x1, y1: a.y1)
        }
        lines = (0..<Int(info.n_lines)).map { k in
            var l = QvpLineInfo(); _ = qvp_line_info(h, UInt32(k), &l)
            return QvpLine(idx: k, lineNo: Int(l.line_no), isHeader: l.is_header != 0, firstWord: Int(l.first_word), nWords: Int(l.n_words),
                           x0: l.x0, y0: l.y0, x1: l.x1, y1: l.y1, bandY0: l.band_y0, bandY1: l.band_y1, centre: l.centre)
        }
        decos = (0..<Int(info.n_decos)).map { k in
            var d = QvpDecoInfo(); _ = qvp_deco_info(h, UInt32(k), &d)
            return QvpDecoration(idx: k, kind: Int(d.kind), surah: Int(d.surah), ayah: Int(d.ayah), line: Int(d.line), x0: d.x0, y0: d.y0, x1: d.x1, y1: d.y1,
                                 text: d.text.string, firstPath: Int(d.first_path), nPaths: Int(d.n_paths))
        }
        naturalPitch = qvp_natural_pitch(h)
    }
    deinit { close() }
    /// Free the native page. Safe to call more than once.
    public func close() { if let p = h { qvp_page_free(p); h = nil } }
    private var p: OpaquePointer { h! }

    // ── geometry ──
    public func pathOpStart(_ i: Int) -> Int { Int(table[i * 8]) }
    public func pathOpCount(_ i: Int) -> Int { Int(table[i * 8 + 1]) }
    public func pathPtStart(_ i: Int) -> Int { Int(table[i * 8 + 2]) }
    public func pathPtCount(_ i: Int) -> Int { Int(table[i * 8 + 3]) }
    public func pathFlags(_ i: Int) -> UInt32 { table[i * 8 + 4] }
    public func pathKind(_ i: Int) -> Int { Int(pathFlags(i) & 0xff) }
    public func pathMark(_ i: Int) -> Int { Int((pathFlags(i) >> 8) & 0xff) }
    public func pathFamily(_ i: Int) -> Int { Int((pathFlags(i) >> 16) & 0xff) }
    public func pathEvenOdd(_ i: Int) -> Bool { (pathFlags(i) >> 24) & 1 == 1 }
    public func pathWord(_ i: Int) -> Int { idx(table[i * 8 + 5]) }
    public func pathLine(_ i: Int) -> Int { Int(table[i * 8 + 6]) }
    public func pathCategory(_ i: Int) -> Int { Int(table[i * 8 + 7] & 0xff) }
    public func pathNthInWord(_ i: Int) -> Int { Int((table[i * 8 + 7] >> 8) & 0xff) }
    public func pathNthMark(_ i: Int) -> Int { let n = Int((table[i * 8 + 7] >> 16) & 0xff); return n == 0xff ? -1 : n }
    /// One CGPath per engine path, in page units, built once. Fill with `pathEvenOdd(i)` as the rule.
    public func buildPaths() -> [CGPath] {
        if let paths { return paths }
        var out: [CGPath] = []; out.reserveCapacity(nPaths)
        for i in 0..<nPaths {
            let m = CGMutablePath()
            var o = pathOpStart(i); let oe = o + pathOpCount(i); var k = pathPtStart(i)
            while o < oe {
                switch ops[o] {
                case 0: m.move(to: CGPoint(x: CGFloat(pts[k]), y: CGFloat(pts[k + 1]))); k += 2
                case 1: m.addLine(to: CGPoint(x: CGFloat(pts[k]), y: CGFloat(pts[k + 1]))); k += 2
                case 2: m.addQuadCurve(to: CGPoint(x: CGFloat(pts[k + 2]), y: CGFloat(pts[k + 3])), control: CGPoint(x: CGFloat(pts[k]), y: CGFloat(pts[k + 1]))); k += 4
                case 3: m.addCurve(to: CGPoint(x: CGFloat(pts[k + 4]), y: CGFloat(pts[k + 5])), control1: CGPoint(x: CGFloat(pts[k]), y: CGFloat(pts[k + 1])), control2: CGPoint(x: CGFloat(pts[k + 2]), y: CGFloat(pts[k + 3]))); k += 6
                case 4: m.closeSubpath()
                default: break
                }
                o += 1
            }
            out.append(m)
        }
        paths = out; return out
    }

    // ── dress: another mushaf's ornaments ──

    /// Put a mushaf's ornaments on this page. Returns the readout, or nil when
    /// the set has no such style. Replaces any previous dress.
    ///
    /// `colors` is by part NAME; the design's own printed colour is kept for
    /// every part left out.
    @discardableResult
    public func dress(_ ornaments: QvpOrnaments, style: Int = 0, gap: Float = 5,
                      lineArt: Bool = false, ayahMarks: Bool = true, surahHeaders: Bool = true,
                      pageFrame: Bool = true, colors: [String: UInt32] = [:]) -> QvpDress? {
        guard style >= 0, style < ornaments.styles.count else { return nil }
        let parts = ornaments.styles[style].parts
        var pairs: [UInt32] = []
        for (name, rgba) in colors {
            if let k = parts.firstIndex(where: { $0.name == name }) { pairs.append(UInt32(k)); pairs.append(rgba) }
        }
        let ok: UInt32 = pairs.withUnsafeBufferPointer { cp in
            var spec = QvpDressSpec(style: UInt32(style), gap: gap,
                                    line_art: lineArt ? 1 : 0, ayah_marks: ayahMarks ? 1 : 0,
                                    surah_headers: surahHeaders ? 1 : 0, page_frame: pageFrame ? 1 : 0,
                                    colors: pairs.isEmpty ? nil : cp.baseAddress, n_colors: UInt32(pairs.count / 2))
            return qvp_dress(p, ornaments.handle, &spec)
        }
        guard ok != 0 else { return nil }
        dressed = nil
        return dressInfo()
    }

    /// Take the ornaments off: the printed rings come back and the viewBox
    /// returns to the page's own.
    public func undress() { qvp_undress(p); dressed = nil }

    /// Bumped whenever the ornament layer is rebuilt — a new dress, or a layout
    /// that respaced the page under the border. 0 when the page is undressed.
    public var dressRevision: UInt32 {
        var d = QvpDressInfo()
        return qvp_dress_info(p, &d) != 0 ? d.revision : 0
    }

    /// The readout, or nil when the page is not dressed.
    public func dressInfo() -> QvpDress? {
        var d = QvpDressInfo()
        guard qvp_dress_info(p, &d) != 0 else { return nil }
        return QvpDress(style: Int(d.style), ayahMarks: Int(d.n_ayah_marks), surahHeaders: Int(d.n_surah_headers),
                        frameRepeats: Int(d.n_frame_repeats), frameStretched: d.frame_stretched != 0, nDraws: Int(d.n_draws),
                        revision: d.revision, viewBox: (d.view_box.0, d.view_box.1, d.view_box.2, d.view_box.3))
    }

    /// The ornament display list, in page units, as CGPath + paint. DRAW IT
    /// BEHIND THE PAGE INK: that is what keeps the print's own ayah numerals on
    /// top of whatever replaced the rings around them.
    public func dressGeometry() -> [QvpOrnamentDraw] {
        // A LAYOUT REBUILDS THIS LAYER: the border is drawn around the laid-out
        // page, so a cache kept only until the next dress() would go stale under
        // a resize. The engine's revision is what says so.
        let rev = dressRevision
        if let dressed, dressedRevision == rev { return dressed }
        dressedRevision = rev
        var g = QvpDressGeometry(); qvp_dress_geometry(p, &g)
        guard g.n_draws > 0, let tab = g.table, let opp = g.ops, let ptp = g.pts else { dressed = []; return [] }
        let t = UnsafeBufferPointer(start: tab, count: Int(g.n_draws) * 9)
        let o = UnsafeBufferPointer(start: opp, count: Int(g.ops_len))
        let v = UnsafeBufferPointer(start: ptp, count: Int(g.pts_len))
        var out: [QvpOrnamentDraw] = []; out.reserveCapacity(Int(g.n_draws))
        for i in 0..<Int(g.n_draws) {
            let m = CGMutablePath()
            var k = Int(t[i * 9]); let ke = k + Int(t[i * 9 + 1]); var j = Int(t[i * 9 + 2])
            while k < ke {
                switch o[k] {
                case 0: m.move(to: CGPoint(x: CGFloat(v[j]), y: CGFloat(v[j + 1]))); j += 2
                case 1: m.addLine(to: CGPoint(x: CGFloat(v[j]), y: CGFloat(v[j + 1]))); j += 2
                case 2: m.addQuadCurve(to: CGPoint(x: CGFloat(v[j + 2]), y: CGFloat(v[j + 3])), control: CGPoint(x: CGFloat(v[j]), y: CGFloat(v[j + 1]))); j += 4
                case 3: m.addCurve(to: CGPoint(x: CGFloat(v[j + 4]), y: CGFloat(v[j + 5])), control1: CGPoint(x: CGFloat(v[j]), y: CGFloat(v[j + 1])), control2: CGPoint(x: CGFloat(v[j + 2]), y: CGFloat(v[j + 3]))); j += 6
                case 4: m.closeSubpath()
                default: break
                }
                k += 1
            }
            let flags = t[i * 9 + 5]
            let line = t[i * 9 + 8]
            out.append(QvpOrnamentDraw(path: m, color: t[i * 9 + 4], kind: OrnamentKind(rawValue: Int(flags & 0xff)) ?? .pageFrame,
                                       evenOdd: flags & 0x100 != 0, stroke: flags & 0x200 != 0,
                                       strokeWidth: Float(bitPattern: t[i * 9 + 6]), part: Int(t[i * 9 + 7]),
                                       line: line == 0xffff_ffff ? -1 : Int(line)))
        }
        dressed = out; return out
    }

    /// How much bigger the dressed page is than the laid-out content, on each
    /// side, in viewport px through the current layout: (left, top, right, bottom).
    /// Fit `content + overflow` or a border is cropped off; an undressed page
    /// answers zeroes.
    public func dressOverflow() -> (Float, Float, Float, Float) {
        var v = [Float](repeating: 0, count: 4)
        v.withUnsafeMutableBufferPointer { qvp_dress_overflow(p, $0.baseAddress) }
        return (v[0], v[1], v[2], v[3])
    }

    /// The page's viewBox (x, y, w, h). A dressed page's border grows it.
    public func viewBox() -> (Float, Float, Float, Float) {
        var v = [Float](repeating: 0, count: 4)
        v.withUnsafeMutableBufferPointer { qvp_page_view_box(p, $0.baseAddress) }
        return (v[0], v[1], v[2], v[3])
    }

    /// The box the page's text occupies (x0, y0, x1, y1) in page units.
    public func contentBox() -> (Float, Float, Float, Float) {
        var v = [Float](repeating: 0, count: 4)
        v.withUnsafeMutableBufferPointer { qvp_content_box(p, $0.baseAddress) }
        return (v[0], v[1], v[2], v[3])
    }

    // ── words / text ──
    public func wordKey(_ i: Int) -> String { words[i].wordKey }
    public func wordForm(_ i: Int, _ form: Form = .rasmUthmani) -> String { var s = QvpStr(); return qvp_word_form(p, UInt32(i), UInt8(form.rawValue), &s) != 0 ? s.string : "" }
    public func findWord(_ surah: Int, _ ayah: Int, _ word: Int) -> Int { Int(qvp_find_word(p, UInt16(surah), UInt16(ayah), UInt16(word))) }
    public func target(_ s: String) -> Target { Target.parse(s, page: self) }
    public func resolve(_ t: Target) -> [Int] { t.withC { tp in collect(512) { o, c in qvp_resolve(p, tp, o, c) } }.map { Int($0) } }
    public func resolve(_ s: String) -> [Int] { resolve(target(s)) }
    public func resolve(_ ws: [Int]) -> [Int] { resolve(Target.words(ws)) }
    public func text(_ t: Target, form: Form = .rasmUthmani, wordSep: String = " ", lineSep: String = "\n") -> String {
        withBytes(wordSep) { wp, wn in withBytes(lineSep) { lp, ln in t.withC { tp in var s = QvpStr(); qvp_text_target(p, tp, UInt8(form.rawValue), wp, wn, lp, ln, &s); return s.string } } }
    }
    public func text(_ s: String = "page", form: Form = .rasmUthmani, wordSep: String = " ", lineSep: String = "\n") -> String { text(target(s), form: form, wordSep: wordSep, lineSep: lineSep) }
    public func search(_ query: String, form: Form = .search, mode: SearchMode = .includes, normalize: Bool = true, loose: Bool = true, limit: Int = 0) -> [QvpMatch] {
        let v: [QvpFFI.QvpMatch] = withBytes(query) { qp, qn in collect(256) { o, c in qvp_search(p, qp, qn, UInt8(form.rawValue), UInt8(mode.rawValue), normalize ? 1 : 0, loose ? 1 : 0, UInt32(limit), o, c) } }
        return v.map { m in let w = Int(m.word); return QvpMatch(word: w, index: Int(m.index), loose: m.loose != 0, wordKey: wordKey(w), text: words[w].text) }
    }
    public func citation(_ ws: [Int]) -> String { ws.map { UInt32($0) }.withUnsafeBufferPointer { b in var s = QvpStr(); qvp_citation(p, b.baseAddress, UInt32(b.count), &s); return s.string } }
    /// Attach a words sidecar ({"s:a:w": {"rasm_imlai","qpc","rasm","search"}}); returns words updated, -1 on bad JSON.
    public func attachWords(_ json: Data) -> Int { json.withUnsafeBytes { Int(qvp_attach_words(p, $0.bindMemory(to: UInt8.self).baseAddress, UInt32($0.count))) } }
    public func attachWords(_ json: String) -> Int { attachWords(Data(json.utf8)) }
    public func hasForm(_ form: Form) -> Bool { qvp_has_form(p, UInt8(form.rawValue)) != 0 }

    // ── metadata ──
    public func surahs() -> [QvpSurah] {
        (0..<Int(qvp_surahs_count(p))).map { i in
            var s = QvpSurahInfo(); _ = qvp_surah_at(p, UInt32(i), &s)
            return QvpSurah(number: Int(s.number), ayahCount: Int(s.ayah_count), hasBanner: s.has_banner != 0, hasBasmalah: s.has_basmalah != 0, place: place(s.place), bannerDeco: idx(s.banner_deco), arabic: s.arabic.string, latin: s.latin.string, english: s.english.string)
        }
    }
    public func divisions() -> [QvpDivision] { collect(64) { o, c in qvp_divisions(p, o, c) }.map { (d: QvpFFI.QvpDivision) in QvpDivision(kind: Division(rawValue: Int(d.kind)) ?? .juz, n: Int(d.n), surah: Int(d.surah), ayah: Int(d.ayah), line: Int(d.line), ayahIdx: Int(d.ayah_idx)) } }
    public func ayahMarks() -> [QvpAyahMark] { collect(128) { o, c in qvp_ayah_marks(p, o, c) }.map { (m: QvpFFI.QvpAyahMark) in QvpAyahMark(deco: idx(m.deco), surah: Int(m.surah), ayah: Int(m.ayah), line: Int(m.line), cx: m.cx, cy: m.cy, r: m.r, ornamentPath: idx(m.ornament_path), numeralPath: idx(m.numeral_path)) } }
    public func ayahMarkOf(_ surah: Int, _ ayah: Int) -> QvpAyahMark? { ayahMarks().first { $0.surah == surah && $0.ayah == ayah } }
    public func rosettes() -> [QvpRosette] { collect(32) { o, c in qvp_rosettes(p, o, c) }.map { (r: QvpFFI.QvpRosette) in QvpRosette(deco: idx(r.deco), surah: Int(r.surah), ayah: Int(r.ayah), juz: Int(r.juz), hizb: Int(r.hizb), nisf: Int(r.nisf), rubuAlHizb: Int(r.rubu_al_hizb), rubuAlHizbInHizb: Int(r.rubu_al_hizb_in_hizb)) } }
    public func sajdahs() -> [QvpSajdah] { collect(16) { o, c in qvp_sajdahs(p, o, c) }.map { (s: QvpFFI.QvpSajdah) in QvpSajdah(deco: idx(s.deco), surah: Int(s.surah), ayah: Int(s.ayah), signPath: idx(s.sign_path)) } }
    public func ayahKeys() -> [(Int, Int)] { collect(256) { o, c in qvp_ayah_keys(p, o, c) }.map { (k: UInt32) in (Int(k >> 16), Int(k & 0xffff)) } }
    /// (count on this page, whole ayah is here)
    public func ayahWordCount(_ surah: Int, _ ayah: Int) -> (count: Int, complete: Bool) { var c: UInt32 = 0; let n = qvp_ayah_word_count(p, UInt16(surah), UInt16(ayah), &c); return (Int(n), c != 0) }
    /// Words for n recitation segments, or nil when the counts disagree (follow the ayah whole).
    public func reciteMap(_ surah: Int, _ ayah: Int, nSegments: Int) -> [Int]? {
        var buf = [UInt32](repeating: 0, count: Swift.max(nSegments, 1) * 4 + 16)
        let n = buf.withUnsafeMutableBufferPointer { qvp_recite_map(p, UInt16(surah), UInt16(ayah), UInt32(nSegments), $0.baseAddress, UInt32($0.count)) }
        if n < 0 { return nil }
        if Int(n) > buf.count { buf = [UInt32](repeating: 0, count: Int(n)); _ = buf.withUnsafeMutableBufferPointer { qvp_recite_map(p, UInt16(surah), UInt16(ayah), UInt32(nSegments), $0.baseAddress, UInt32($0.count)) } }
        return buf.prefix(Int(n)).map { Int($0) }
    }
    public func wordLabel(_ i: Int) -> String { var s = QvpStr(); qvp_word_label(p, UInt32(i), &s); return s.string }
    public func ayahLabel(_ i: Int) -> String { var s = QvpStr(); qvp_ayah_label(p, UInt32(i), &s); return s.string }

    // ── hit testing ──
    private func hit(_ ok: Int32, _ v: QvpFFI.QvpHit) -> QvpHit? { ok != 0 ? QvpHit(word: idx(v.word), path: idx(v.path), deco: idx(v.deco)) : nil }
    private func hitEx(_ ok: Int32, _ v: QvpFFI.QvpHitEx) -> QvpHitEx? { ok != 0 ? QvpHitEx(word: idx(v.word), path: idx(v.path), deco: idx(v.deco), line: idx(v.line), distance: v.distance, exact: v.exact != 0) : nil }
    public func hitTest(_ x: Float, _ y: Float) -> QvpHit? { var v = QvpFFI.QvpHit(); return hit(qvp_hit_test(p, x, y, &v), v) }
    public func hitTestView(_ vx: Float, _ vy: Float) -> QvpHit? { var v = QvpFFI.QvpHit(); return hit(qvp_hit_test_view(p, vx, vy, &v), v) }
    /// Gap-aware: every point on a printed line resolves to the word the reader meant.
    public func hitTestEx(_ x: Float, _ y: Float, _ o: QvpHitOptions = QvpHitOptions()) -> QvpHitEx? {
        var opt = QvpFFI.QvpHitOptions(max_distance: o.maxDistance, gap_bias: o.gapBias, exact_first: o.exactFirst ? 1 : 0); var v = QvpFFI.QvpHitEx()
        return hitEx(qvp_hit_test_ex(p, x, y, &opt, &v), v)
    }
    public func hitTestViewEx(_ vx: Float, _ vy: Float, _ o: QvpHitOptions = QvpHitOptions()) -> QvpHitEx? {
        var opt = QvpFFI.QvpHitOptions(max_distance: o.maxDistance, gap_bias: o.gapBias, exact_first: o.exactFirst ? 1 : 0); var v = QvpFFI.QvpHitEx()
        return hitEx(qvp_hit_test_view_ex(p, vx, vy, &opt, &v), v)
    }
    public func lineBands() -> [QvpLineBand] { collect(64) { o, c in qvp_line_bands(p, o, c) }.map { (b: QvpFFI.QvpLineBand) in QvpLineBand(line: Int(b.line), lineNo: Int(b.line_no), y0: b.y0, y1: b.y1, mid: b.mid, inkY0: b.ink_y0, inkY1: b.ink_y1) } }
    public func hitBoxes(gapBias: Float = 0.6) -> [QvpHitBox] { collect(512) { o, c in qvp_hit_boxes(p, gapBias, o, c) }.map { (b: QvpFFI.QvpHitBox) in QvpHitBox(word: Int(b.word), line: Int(b.line), x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, inkX0: b.ink_x0, inkY0: b.ink_y0, inkX1: b.ink_x1, inkY1: b.ink_y1) } }

    // ── layout ──
    @discardableResult
    public func layout(_ spec: QvpLayoutSpec) -> QvpLayout {
        var s = QvpFFI.QvpLayoutSpec(viewport_w: spec.viewportW, viewport_h: spec.viewportH, pad_top: spec.padTop, pad_bottom: spec.padBottom, pad_left: spec.padLeft, pad_right: spec.padRight, line_spacing: spec.lineSpacing, line_gap: spec.lineGap, fill_height: spec.fillHeight ? 1 : 0, nominal_lines: UInt32(spec.nominalLines))
        var l = QvpFFI.QvpLayout(); qvp_layout(p, &s, &l)
        let n = Int(l.n_lines); let f = Array(UnsafeBufferPointer(start: l.lines, count: n * 3))
        let out = QvpLayout(scale: l.scale, ox: l.ox, oy: l.oy, contentW: l.content_w, contentH: l.content_h, pitch: l.pitch,
                            lineDy: (0..<n).map { f[$0 * 3] }, slotTop: (0..<n).map { f[$0 * 3 + 1] }, slotBottom: (0..<n).map { f[$0 * 3 + 2] })
        currentLayout = out; return out
    }
    /// A word's box in viewport px through the current layout (x0, y0, x1, y1).
    public func wordBoxView(_ i: Int) -> (x0: Float, y0: Float, x1: Float, y1: Float)? {
        var b: (Float, Float, Float, Float) = (0, 0, 0, 0)
        let ok = withUnsafeMutablePointer(to: &b) { $0.withMemoryRebound(to: Float.self, capacity: 4) { qvp_word_box_view(p, UInt32(i), $0) } }
        return ok != 0 ? (b.0, b.1, b.2, b.3) : nil
    }

    // ── styles (handles undo exactly) ──
    @discardableResult public func style(_ sel: Selector, _ rgba: UInt32, transitionMs: Int = 0, layer: Int = QvpLayer.BASE) -> Int { var s = sel.c; return Int(qvp_style_add(p, Int32(layer), &s, rgba, UInt32(transitionMs))) }
    @discardableResult public func styleTarget(_ t: Target, _ rgba: UInt32, transitionMs: Int = 0, layer: Int = QvpLayer.BASE) -> Int { t.withC { Int(qvp_style_add_target(p, Int32(layer), $0, rgba, UInt32(transitionMs))) } }
    @discardableResult public func styleTarget(_ s: String, _ rgba: UInt32, transitionMs: Int = 0, layer: Int = QvpLayer.BASE) -> Int { styleTarget(target(s), rgba, transitionMs: transitionMs, layer: layer) }
    @discardableResult public func unstyle(_ handle: Int) -> Int { Int(qvp_style_remove(p, UInt32(handle))) }
    @discardableResult public func restyle(_ handle: Int, _ rgba: UInt32, transitionMs: Int = 0) -> Int { Int(qvp_style_repaint(p, UInt32(handle), rgba, UInt32(transitionMs))) }
    @discardableResult public func hide(_ sel: Selector) -> Int { var s = sel.c; return Int(qvp_hide(p, &s)) }
    public func clearStyles() { qvp_style_clear(p) }
    public func clearLayer(_ layer: Int) { qvp_style_clear_layer(p, Int32(layer)) }
    public func setDefaultInk(_ rgba: UInt32) { defaultInk = rgba; qvp_style_default(p, rgba) }
    @discardableResult public func theme(_ t: QvpTheme) -> Int {
        let pairs: [UInt32] = t.marks.flatMap { [UInt32(markId($0.key)), $0.value] }
        return pairs.withUnsafeBufferPointer { mp in
            var c = QvpFFI.QvpTheme(ink: t.ink ?? 0, diacritics: t.diacritics ?? 0, dots: t.dots ?? 0, waqf: t.waqf ?? 0, sifr: t.sifr ?? 0, ayah_mark: t.ayahMark ?? 0, numeral: t.numeral ?? 0, headers: t.headers ?? 0, transition_ms: UInt32(t.transitionMs), marks: pairs.isEmpty ? nil : mp.baseAddress, n_marks: UInt32(pairs.count / 2))
            return Int(qvp_theme(p, &c))
        }
    }
    public func styleHandles() -> [Int] { collect(256) { o, c in qvp_style_handles(p, o, c) }.map { (h: UInt32) in Int(h) } }

    // ── clock & display list ──
    /// Advance animations; true while something is still moving (keep drawing frames).
    public func tick(_ nowMs: Double) -> Bool { qvp_tick(p, nowMs) != 0 }
    /// A colour per path (current, mid-transition).
    public func paint() -> [UInt32] { Array(UnsafeBufferPointer(start: qvp_paint(p), count: nPaths)) }
    /// (path, colour) pairs for paths ≠ default ink.
    public func styled() -> [(path: Int, color: UInt32)] {
        let v: [UInt32] = collect(512) { o, c in qvp_styled(p, o, c / 2) * 2 }
        return stride(from: 0, to: v.count - 1, by: 2).map { (Int(v[$0]), v[$0 + 1]) }
    }
    public func colorOf(_ path: Int) -> UInt32 { qvp_color_of(p, UInt32(path)) }

    // ── highlights ──
    @discardableResult public func highlight(_ t: Target, _ style: QvpHighlightStyle = QvpHighlightStyle()) -> Int { var s = style.c; return t.withC { Int(qvp_highlight(p, $0, &s)) } }
    @discardableResult public func highlight(_ s: String, _ style: QvpHighlightStyle = QvpHighlightStyle()) -> Int { highlight(target(s), style) }
    @discardableResult public func rehighlight(_ handle: Int, _ t: Target) -> Bool { t.withC { qvp_rehighlight(p, UInt32(handle), $0) != 0 } }
    @discardableResult public func rehighlight(_ handle: Int, _ s: String) -> Bool { rehighlight(handle, target(s)) }
    @discardableResult public func restyleHighlight(_ handle: Int, _ style: QvpHighlightStyle) -> Bool { var s = style.c; return qvp_restyle_highlight(p, UInt32(handle), &s) != 0 }
    @discardableResult public func unhighlight(_ handle: Int) -> Bool { qvp_unhighlight(p, UInt32(handle)) != 0 }
    public func clearHighlights() { qvp_clear_highlights(p) }
    public func highlightHandles() -> [Int] { collect(64) { o, c in qvp_highlight_handles(p, o, c) }.map { (h: UInt32) in Int(h) } }
    public func highlightWords(_ handle: Int) -> [Int] { collect(512) { o, c in qvp_highlight_words(p, UInt32(handle), o, c) }.map { (w: UInt32) in Int(w) } }
    private func boxes(_ v: [QvpFFI.QvpBox]) -> [QvpBox] { v.map { QvpBox(id: Int($0.id), line: Int($0.line), x0: $0.x0, y0: $0.y0, x1: $0.x1, y1: $0.y1, color: $0.color, radius: $0.radius) } }
    /// Animated band boxes in viewport px; draw each id as one nonzero path behind the ink.
    public func highlightBoxes() -> [QvpBox] { boxes(collect(128) { o, c in qvp_highlight_boxes(p, o, c) }) }
    public func bandBoxes(_ ws: [Int], height: BandHeight = .pitch, padX: Float = 1.2, padY: Float = 0) -> [QvpBox] {
        ws.map { UInt32($0) }.withUnsafeBufferPointer { wb in boxes(collect(64) { o, c in qvp_band_boxes(p, wb.baseAddress, UInt32(wb.count), UInt8(height.rawValue), padX, padY, o, c) }) }
    }

    // ── selection ──
    public func select(_ anchor: Int, _ focus: Int? = nil) { qvp_select(p, anchor < 0 ? QVP_NONE : UInt32(anchor), (focus ?? anchor) < 0 ? QVP_NONE : UInt32(focus ?? anchor)) }
    public func clearSelection() { qvp_select(p, QVP_NONE, QVP_NONE) }
    public func selection() -> [Int] { collect(512) { o, c in qvp_selection(p, o, c) }.map { (w: UInt32) in Int(w) } }
    public func selectionText(_ form: Form = .rasmUthmani, citation: Bool = false) -> String { var s = QvpStr(); qvp_selection_text(p, UInt8(form.rawValue), citation ? 1 : 0, &s); return s.string }

    // ── memorisation ──
    public func mask(_ t: Target, _ mode: MaskMode = .hide) { t.withC { qvp_mask(p, $0, UInt8(mode.rawValue)) } }
    public func mask(_ s: String, _ mode: MaskMode = .hide) { mask(target(s), mode) }
    public func maskFrom(_ wi: Int, _ mode: MaskMode = .hide) { qvp_mask_from(p, UInt32(wi), UInt8(mode.rawValue)) }
    public func maskOptions(blockColor: UInt32 = 0xd9d4c8ff, padX: Float = 0.6, padY: Float = 0.6, radius: Float = 0.8, reverse: Bool = false) { qvp_mask_options(p, blockColor, padX, padY, radius, reverse ? 1 : 0) }
    @discardableResult public func revealNext(_ n: Int = 1) -> Int { Int(qvp_reveal_next(p, UInt32(n))) }
    @discardableResult public func hideBack(_ n: Int = 1) -> Int { Int(qvp_hide_back(p, UInt32(n))) }
    @discardableResult public func revealWord(_ wi: Int) -> Bool { qvp_reveal_word(p, UInt32(wi)) != 0 }
    @discardableResult public func hideWord(_ wi: Int) -> Bool { qvp_hide_word(p, UInt32(wi)) != 0 }
    public func revealAll() { qvp_reveal_all(p) }
    public func hideAll() { qvp_hide_all(p) }
    public func unmask() { qvp_unmask(p) }
    public func maskHidden() -> [Int] { collect(512) { o, c in qvp_mask_hidden(p, o, c) }.map { (w: UInt32) in Int(w) } }
    public func maskWords() -> [Int] { collect(512) { o, c in qvp_mask_words(p, o, c) }.map { (w: UInt32) in Int(w) } }
    /// Block/blur boxes in viewport px, drawn over the ink.
    public func maskBoxes() -> [QvpBox] { boxes(collect(128) { o, c in qvp_mask_boxes(p, o, c) }) }
    /// Greyed page with a lit window; returns steps.
    @discardableResult public func revealStart(lit: Int = 1, byAyah: Bool = false, grey: UInt32 = 0xc9c4b8ff, ink: UInt32 = 0x231f20ff, ayahMarks: Bool = true, transitionMs: Int = 0) -> Int {
        Int(qvp_reveal_start(p, UInt32(lit), byAyah ? 1 : 0, grey, ink, ayahMarks ? 1 : 0, UInt32(transitionMs)))
    }
    @discardableResult public func revealGoto(_ at: Int) -> Bool { qvp_reveal_goto(p, Int64(at)) != 0 }
    /// Current step, -1 when nothing is lit yet, nil when no reveal is running.
    public func revealAt() -> Int? { let v = qvp_reveal_at(p); return v == -2 ? nil : Int(v) }
    public func revealSteps() -> Int { Int(qvp_reveal_steps(p)) }
    public func revealStepOf(_ wi: Int) -> Int { Int(qvp_reveal_step_of(p, UInt32(wi))) }
    public func revealStop() { qvp_reveal_stop(p) }

    // ── crop ──
    public func cropBox(_ t: Target, pad: Float = 2, keepAyahMarks: Bool = true) -> QvpCropBox? {
        var b = QvpFFI.QvpCropBox()
        guard t.withC({ qvp_crop_box(p, $0, pad, keepAyahMarks ? 1 : 0, &b) }) != 0 else { return nil }
        return QvpCropBox(x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, nWords: Int(b.n_words), ayahMarkDeco: idx(b.ayah_mark_deco))
    }
    public func cropBox(_ s: String, pad: Float = 2, keepAyahMarks: Bool = true) -> QvpCropBox? { cropBox(target(s), pad: pad, keepAyahMarks: keepAyahMarks) }
    /// Standalone SVG with the current colours; background alpha 0 = transparent.
    public func cropSvg(_ t: Target, pad: Float = 2, keepAyahMarks: Bool = true, background: UInt32 = 0) -> String? {
        var s = QvpStr()
        return t.withC({ qvp_crop_svg(p, $0, pad, keepAyahMarks ? 1 : 0, background, &s) }) != 0 ? s.string : nil
    }
    public func cropSvg(_ key: String, pad: Float = 2, keepAyahMarks: Bool = true, background: UInt32 = 0) -> String? { cropSvg(target(key), pad: pad, keepAyahMarks: keepAyahMarks, background: background) }
}

/// Cross-page lookup from atlas.qva.
public final class QvpAtlas {
    private var h: OpaquePointer?
    public init(bytes: Data) throws {
        h = bytes.withUnsafeBytes { qvp_atlas_load($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        if h == nil { throw QvpError.badAtlas }
    }
    deinit { close() }
    public func close() { if let a = h { qvp_atlas_free(a); h = nil } }
    private var p: OpaquePointer { h! }
    private func surah(_ ok: Int32, _ s: QvpFFI.QvpAtlasSurah) -> QvpAtlasSurah? {
        ok != 0 ? QvpAtlasSurah(n: Int(s.n), page: Int(s.first_page), ayahCount: Int(s.ayah_count), place: place(s.place), arabic: s.arabic.string, latin: s.latin.string, english: s.english.string) : nil
    }
    public func pageOf(_ surah: Int, _ ayah: Int) -> Int? { let v = qvp_atlas_page_of(p, UInt16(surah), UInt16(ayah)); return v < 0 ? nil : Int(v) }
    /// ((first surah, first ayah), (last surah, last ayah)) printed on the page.
    public func pageRange(_ page: Int) -> (first: (Int, Int), last: (Int, Int))? {
        var r: (UInt16, UInt16, UInt16, UInt16) = (0, 0, 0, 0)
        let ok = withUnsafeMutablePointer(to: &r) { $0.withMemoryRebound(to: UInt16.self, capacity: 4) { qvp_atlas_page_range(p, UInt16(page), $0) } }
        return ok != 0 ? ((Int(r.0), Int(r.1)), (Int(r.2), Int(r.3))) : nil
    }
    public func pages() -> Int { Int(qvp_atlas_pages(p)) }
    public func surah(_ n: Int) -> QvpAtlasSurah? { var s = QvpFFI.QvpAtlasSurah(); return surah(qvp_atlas_surah(p, UInt16(n), &s), s) }
    public func surahs() -> [QvpAtlasSurah] { (0..<Int(qvp_atlas_surahs(p))).compactMap { var s = QvpFFI.QvpAtlasSurah(); return surah(qvp_atlas_surah_at(p, UInt32($0), &s), s) } }
    public func pageOfSurah(_ n: Int) -> Int? { surah(n)?.page }
    public func division(_ kind: Division, _ n: Int) -> QvpAtlasRubuAlHizb? {
        var r = QvpFFI.QvpAtlasRubuAlHizb()
        return qvp_atlas_division(p, UInt8(kind.rawValue), UInt16(n), &r) != 0 ? QvpAtlasRubuAlHizb(rubuAlHizb: Int(r.rubu_al_hizb), surah: Int(r.surah), ayah: Int(r.ayah), page: Int(r.page)) : nil
    }
    public func juz(_ n: Int) -> QvpAtlasRubuAlHizb? { division(.juz, n) }
    public func hizb(_ n: Int) -> QvpAtlasRubuAlHizb? { division(.hizb, n) }
    public func rubuAlHizb(_ n: Int) -> QvpAtlasRubuAlHizb? { division(.rubuAlHizb, n) }
    public func divisionAt(_ kind: Division, _ surah: Int, _ ayah: Int) -> Int? { let v = qvp_atlas_division_at(p, UInt8(kind.rawValue), UInt16(surah), UInt16(ayah)); return v < 0 ? nil : Int(v) }
    public func juzAt(_ surah: Int, _ ayah: Int) -> Int? { divisionAt(.juz, surah, ayah) }
    public func pagesOfJuz(_ n: Int) -> (Int, Int)? {
        var r: (UInt16, UInt16) = (0, 0)
        let ok = withUnsafeMutablePointer(to: &r) { $0.withMemoryRebound(to: UInt16.self, capacity: 2) { qvp_atlas_pages_of_juz(p, UInt16(n), $0) } }
        return ok != 0 ? (Int(r.0), Int(r.1)) : nil
    }
    /// 'cow' | 'البقرة' | '2' → matching surahs.
    public func findSurah(_ text: String) -> [QvpAtlasSurah] {
        let ns: [UInt16] = withBytes(text) { tp, tn in collect(128) { o, c in qvp_atlas_find_surah(p, tp, tn, o, c) } }
        return ns.compactMap { surah(Int($0)) }
    }
    public func json() -> String { var s = QvpStr(); qvp_atlas_json(p, &s); return s.string }
}

/// The ornaments of other printed mushafs, from `ornaments.qvo`.
///
/// NONE OF THESE OUTLINES IS PART OF A QVP PAGE. They are traced from scans of
/// other prints, and each style says what it may be redistributed under — read
/// `licence` before you publish a page wearing them.
public final class QvpOrnaments {
    private var h: OpaquePointer?
    public private(set) var styles: [QvpOrnamentStyle] = []

    /// Load an `ornaments.qvo` set; bytes are copied by the engine.
    public init(bytes: Data) throws {
        h = bytes.withUnsafeBytes { qvp_ornaments_load($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        guard let h else { throw QvpError.badOrnaments }
        styles = (0..<Int(qvp_ornament_styles(h))).map { i in
            var s = QvpFFI.QvpOrnamentStyle()
            _ = qvp_ornament_style(h, UInt32(i), &s)
            let parts = (0..<Int(s.n_parts)).map { k -> QvpOrnamentPart in
                var pt = QvpFFI.QvpOrnamentPart()
                _ = qvp_ornament_part(h, UInt32(i), UInt32(k), &pt)
                return QvpOrnamentPart(index: k, name: pt.name.string, color: pt.color, stroke: pt.stroke != 0)
            }
            return QvpOrnamentStyle(index: i, name: s.name.string, riwayah: s.riwayah.string,
                                    hasAyahMark: s.assets & 1 != 0, hasSurahHeader: s.assets & 2 != 0,
                                    hasPageFrame: s.assets & 4 != 0, tiles: s.assets & 8 != 0,
                                    licence: QvpOrnamentLicence(id: s.license_id.string, status: s.license_status.string,
                                                                redistributable: s.redistributable != 0,
                                                                attribution: s.attribution.string),
                                    parts: parts)
        }
    }
    deinit { close() }
    public func close() { if let o = h { qvp_ornaments_free(o); h = nil } }
    internal var handle: OpaquePointer { h! }
    public func find(_ name: String) -> QvpOrnamentStyle? {
        let i = name.withCString { cs -> Int32 in
            let n = strlen(cs)
            return cs.withMemoryRebound(to: UInt8.self, capacity: n) { qvp_ornament_find_style(handle, $0, UInt32(n)) }
        }
        return i < 0 ? nil : styles[Int(i)]
    }
}
