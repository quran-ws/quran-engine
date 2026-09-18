// One loaded page and the cross-page atlas, over the C ABI in qvp.h.
// Geometry is copied out of the engine once; everything that decides (hit-testing, layout,
// styles, highlights, masks, search) stays inside the engine. Mirrors web/qvp.js and the
// Kotlin QvpPage — see docs/API.md.
import Foundation
import CoreGraphics
import QvpFFI

public enum QvpError: Error { case badPage, badAtlas }

public final class QvpPage {
    private var h: OpaquePointer?

    public let width: Float, height: Float, pageNo: Int
    public let nLines: Int, nAyahs: Int, nWords: Int, nPaths: Int, nDecorations: Int
    /// path ops: 0 MoveTo 1 LineTo 2 QuadTo 3 CubicTo 4 Close
    public let ops: [UInt8]
    /// x,y pairs consumed in order by the ops
    public let pts: [Float]
    /// stride 8 per path: opStart, opCount, ptStart, ptCount, flags, word, line, extra
    public let table: [UInt32]
    public let words: [QvpWord], ayahs: [QvpAyah], lines: [QvpLine], decorations: [QvpDecoration]
    public let lineSpacing: Float
    public private(set) var currentLayout: QvpLayout?
    public private(set) var defaultInk: UInt32 = QvpDefaults.INK
    private var paths: [CGPath]?

    /// Load a `NNN.qvp` page; bytes are copied by the engine.
    public init(bytes: Data) throws {
        h = bytes.withUnsafeBytes { qvp_page_load($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        guard let h else { throw QvpError.badPage }
        var info = QvpPageInfo(); qvp_page_info(h, &info)
        width = info.width; height = info.height; pageNo = Int(info.page)
        nLines = Int(info.n_lines); nAyahs = Int(info.n_ayahs); nWords = Int(info.n_words); nPaths = Int(info.n_paths); nDecorations = Int(info.n_decorations)
        var g = QvpGeometry(); qvp_geometry(h, &g)
        ops = Array(UnsafeBufferPointer(start: g.ops, count: Int(g.ops_len)))
        pts = Array(UnsafeBufferPointer(start: g.pts, count: Int(g.pts_len)))
        table = Array(UnsafeBufferPointer(start: g.table, count: Int(g.n_paths) * 8))
        words = (0..<Int(info.n_words)).map { k in
            var w = QvpWordInfo(); _ = qvp_word_info(h, UInt32(k), &w)
            return QvpWord(index: k, surah: Int(w.surah), ayah: Int(w.ayah), word: Int(w.word), line: Int(w.line_number), ayahIndex: Int(w.ayah_index), lineIndex: Int(w.line_index),
                           x0: w.x0, y0: w.y0, x1: w.x1, y1: w.y1, text: w.text.string, firstPath: Int(w.first_path), nPaths: Int(w.n_paths))
        }
        ayahs = (0..<Int(info.n_ayahs)).map { k in
            var a = QvpAyahInfo(); _ = qvp_ayah_info(h, UInt32(k), &a)
            return QvpAyah(index: k, surah: Int(a.surah), ayah: Int(a.ayah), fragment: Int(a.fragment), fragments: Int(a.fragments), flags: Int(a.flags), rubuAlHizb: Int(a.rubu_al_hizb), firstWord: Int(a.first_word), nWords: Int(a.n_words),
                           ayahMarkDecoration: index(a.ayah_mark_decoration), x0: a.x0, y0: a.y0, x1: a.x1, y1: a.y1)
        }
        lines = (0..<Int(info.n_lines)).map { k in
            var l = QvpLineInfo(); _ = qvp_line_info(h, UInt32(k), &l)
            return QvpLine(index: k, lineNumber: Int(l.line_number), isHeader: l.is_header != 0, firstWord: Int(l.first_word), nWords: Int(l.n_words),
                           x0: l.x0, y0: l.y0, x1: l.x1, y1: l.y1, bandY0: l.band_y0, bandY1: l.band_y1, centre: l.centre)
        }
        decorations = (0..<Int(info.n_decorations)).map { k in
            var d = QvpDecorationInfo(); _ = qvp_decoration_info(h, UInt32(k), &d)
            return QvpDecoration(index: k, decoration: Int(d.decoration), surah: Int(d.surah), ayah: Int(d.ayah), line: Int(d.line), x0: d.x0, y0: d.y0, x1: d.x1, y1: d.y1,
                                 text: d.text.string, firstPath: Int(d.first_path), nPaths: Int(d.n_paths))
        }
        lineSpacing = qvp_page_line_spacing(h)
    }
    deinit { close() }
    /// Free the native page. Safe to call more than once.
    public func close() { if let p = h { qvp_page_free(p); h = nil } }
    /// False once `close()` has run. A renderer racing a host's page teardown
    /// must check this before calling into the engine — every engine call on
    /// a closed page traps.
    public var isOpen: Bool { h != nil }
    /// The engine handle. Internal: the wrapper marshals through it, no app ever sees it.
    var p: OpaquePointer { h! }

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
    public func pathWord(_ i: Int) -> Int { index(table[i * 8 + 5]) }
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

    // ── words / text ──
    public func wordKey(_ i: Int) -> String { words[i].wordKey }
    public func wordForm(_ i: Int, _ form: Form = .rasmUthmani) -> String { var s = QvpStr(); return qvp_word_form(p, UInt32(i), UInt8(form.rawValue), &s) != 0 ? s.string : "" }
    public func findWord(_ surah: Int, _ ayah: Int, _ word: Int) -> Int { Int(qvp_find_word(p, UInt16(surah), UInt16(ayah), UInt16(word))) }
    public func target(_ s: String) -> Target { Target.parse(s, page: self) }
    public func targetWords(_ t: Target) -> [Int] { t.withC { tp in collect(512) { o, c in qvp_target_words(p, tp, o, c) } }.map { Int($0) } }
    public func targetWords(_ s: String) -> [Int] { targetWords(target(s)) }
    public func targetWords(_ ws: [Int]) -> [Int] { targetWords(Target.words(ws)) }
    public func text(_ t: Target, form: Form = .rasmUthmani, wordSep: String = " ", lineSep: String = "\n") -> String {
        withBytes(wordSep) { wp, wn in withBytes(lineSep) { lp, ln in t.withC { tp in var s = QvpStr(); qvp_text(p, tp, UInt8(form.rawValue), wp, wn, lp, ln, &s); return s.string } } }
    }
    public func text(_ s: String = "page", form: Form = .rasmUthmani, wordSep: String = " ", lineSep: String = "\n") -> String { text(target(s), form: form, wordSep: wordSep, lineSep: lineSep) }
    public func search(_ query: String, form: Form = .search, mode: SearchMode = .includes, normalize: Bool = true, looseMatch: Bool = true, limit: Int = 0) -> [QvpMatch] {
        let v: [QvpFFI.QvpMatch] = withBytes(query) { qp, qn in collect(256) { o, c in qvp_search(p, qp, qn, UInt8(form.rawValue), UInt8(mode.rawValue), normalize ? 1 : 0, looseMatch ? 1 : 0, UInt32(limit), o, c) } }
        return v.map { m in let w = Int(m.word); return QvpMatch(word: w, index: Int(m.index), isLooseMatch: m.is_loose_match != 0, wordKey: wordKey(w), text: words[w].text) }
    }
    public func citation(_ ws: [Int]) -> String { ws.map { UInt32($0) }.withUnsafeBufferPointer { b in var s = QvpStr(); qvp_citation(p, b.baseAddress, UInt32(b.count), &s); return s.string } }
    /// Attach a words sidecar ({"s:a:w": {"rasm_imlai","qpc","rasm","search"}}); returns words updated, -1 on bad JSON.
    public func attachWords(_ json: Data) -> Int { json.withUnsafeBytes { Int(qvp_attach_words(p, $0.bindMemory(to: UInt8.self).baseAddress, UInt32($0.count))) } }
    public func attachWords(_ json: String) -> Int { attachWords(Data(json.utf8)) }
    public func hasForm(_ form: Form) -> Bool { qvp_has_form(p, UInt8(form.rawValue)) != 0 }

    // ── metadata ──
    public func surahs() -> [QvpSurah] {
        (0..<Int(qvp_surah_count(p))).map { i in
            var s = QvpFFI.QvpSurah(); _ = qvp_surah_at(p, UInt32(i), &s)
            return QvpSurah(number: Int(s.number), ayahCount: Int(s.ayah_count), hasBanner: s.has_banner != 0, hasBasmalah: s.has_basmalah != 0, place: place(s.place), bannerDecoration: index(s.banner_decoration), arabic: s.arabic.string, latin: s.latin.string, english: s.english.string)
        }
    }
    public func divisions() -> [QvpDivision] { collect(64) { o, c in qvp_divisions(p, o, c) }.map { (d: QvpFFI.QvpDivision) in QvpDivision(division: Division(rawValue: Int(d.division)) ?? .juz, number: Int(d.number), surah: Int(d.surah), ayah: Int(d.ayah), line: Int(d.line), ayahIndex: Int(d.ayah_index)) } }
    public func ayahMarks() -> [QvpAyahMark] { collect(128) { o, c in qvp_ayah_marks(p, o, c) }.map { (m: QvpFFI.QvpAyahMark) in QvpAyahMark(decoration: index(m.decoration), surah: Int(m.surah), ayah: Int(m.ayah), line: Int(m.line), cx: m.cx, cy: m.cy, r: m.r, ornamentPath: index(m.ornament_path), numeralPath: index(m.numeral_path)) } }
    public func ayahMarkOf(_ surah: Int, _ ayah: Int) -> QvpAyahMark? { ayahMarks().first { $0.surah == surah && $0.ayah == ayah } }
    public func rosettes() -> [QvpRosette] { collect(32) { o, c in qvp_rosettes(p, o, c) }.map { (r: QvpFFI.QvpRosette) in QvpRosette(decoration: index(r.decoration), surah: Int(r.surah), ayah: Int(r.ayah), juz: Int(r.juz), hizb: Int(r.hizb), nisf: Int(r.nisf), rubuAlHizb: Int(r.rubu_al_hizb), rubuAlHizbInHizb: Int(r.rubu_al_hizb_in_hizb)) } }
    public func sajdahs() -> [QvpSajdah] { collect(16) { o, c in qvp_sajdahs(p, o, c) }.map { (s: QvpFFI.QvpSajdah) in QvpSajdah(decoration: index(s.decoration), surah: Int(s.surah), ayah: Int(s.ayah), signPath: index(s.sign_path)) } }
    public func ayahKeys() -> [(Int, Int)] { collect(256) { o, c in qvp_ayah_keys(p, o, c) }.map { (k: UInt32) in (Int(k >> 16), Int(k & 0xffff)) } }
    /// (count on this page, whole ayah is here)
    public func ayahWordCount(_ surah: Int, _ ayah: Int) -> (count: Int, isComplete: Bool) { var c: UInt8 = 0; let n = qvp_ayah_word_count(p, UInt16(surah), UInt16(ayah), &c); return (Int(n), c != 0) }
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
    private func hit(_ ok: Int32, _ v: QvpFFI.QvpHit) -> QvpHit? { ok != 0 ? QvpHit(word: index(v.word), path: index(v.path), decoration: index(v.decoration), line: index(v.line), distance: v.distance, isExact: v.is_exact != 0) : nil }
    /// Exact outline only, page units.
    public func hitTestExact(_ x: Float, _ y: Float) -> QvpHit? { var v = QvpFFI.QvpHit(); return hit(qvp_hit_test_exact(p, x, y, &v), v) }
    /// Exact outline only, viewport px through the current layout.
    public func hitTestExactView(_ viewX: Float, _ viewY: Float) -> QvpHit? { var v = QvpFFI.QvpHit(); return hit(qvp_hit_test_exact_view(p, viewX, viewY, &v), v) }
    /// Gap-aware: every point on a printed line resolves to the word the reader meant.
    public func hitTest(_ x: Float, _ y: Float, _ o: QvpHitOptions = QvpHitOptions()) -> QvpHit? {
        var opt = QvpFFI.QvpHitOptions(max_distance: o.maxDistance, gap_bias: o.gapBias, prefer_exact: o.preferExact ? 1 : 0); var v = QvpFFI.QvpHit()
        return hit(qvp_hit_test(p, x, y, &opt, &v), v)
    }
    public func hitTestView(_ viewX: Float, _ viewY: Float, _ o: QvpHitOptions = QvpHitOptions()) -> QvpHit? {
        var opt = QvpFFI.QvpHitOptions(max_distance: o.maxDistance, gap_bias: o.gapBias, prefer_exact: o.preferExact ? 1 : 0); var v = QvpFFI.QvpHit()
        return hit(qvp_hit_test_view(p, viewX, viewY, &opt, &v), v)
    }
    public func lineBands() -> [QvpLineBand] { collect(64) { o, c in qvp_line_bands(p, o, c) }.map { (b: QvpFFI.QvpLineBand) in QvpLineBand(line: Int(b.line), lineNumber: Int(b.line_number), y0: b.y0, y1: b.y1, mid: b.mid, inkY0: b.ink_y0, inkY1: b.ink_y1) } }
    public func hitAreas(gapBias: Float = QvpDefaults.GAP_BIAS) -> [QvpHitArea] { collect(512) { o, c in qvp_hit_areas(p, gapBias, o, c) }.map { (b: QvpFFI.QvpHitArea) in QvpHitArea(word: Int(b.word), line: Int(b.line), x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, inkX0: b.ink_x0, inkY0: b.ink_y0, inkX1: b.ink_x1, inkY1: b.ink_y1) } }

    // ── layout ──
    @discardableResult
    public func layout(_ spec: QvpLayoutSpec) -> QvpLayout {
        var s = spec.c
        var l = QvpFFI.QvpLayout(); qvp_layout(p, &s, &l)
        let n = Int(l.n_lines); let f = Array(UnsafeBufferPointer(start: l.lines, count: n * 3))
        let out = QvpLayout(scale: l.scale, offsetX: l.offset_x, offsetY: l.offset_y, contentW: l.content_w, contentH: l.content_h, lineSpacing: l.line_spacing,
                            lineDy: (0..<n).map { f[$0 * 3] }, slotTop: (0..<n).map { f[$0 * 3 + 1] }, slotBottom: (0..<n).map { f[$0 * 3 + 2] },
                            fitScale: l.fit_scale, fitX: l.fit_x, fitY: l.fit_y,
                            reflowed: l.reflowed != 0, rows: Int(l.n_rows))
        currentLayout = out; return out
    }

    /// Read back the layout the page already has, without computing one: what to call after the
    /// engine laid the page out itself, as the zoom control does.
    @discardableResult
    public func readLayout() -> QvpLayout? {
        var l = QvpFFI.QvpLayout()
        guard qvp_layout_current(p, &l) != 0 else { return nil }
        let n = Int(l.n_lines); let f = Array(UnsafeBufferPointer(start: l.lines, count: n * 3))
        let out = QvpLayout(scale: l.scale, offsetX: l.offset_x, offsetY: l.offset_y, contentW: l.content_w, contentH: l.content_h, lineSpacing: l.line_spacing,
                            lineDy: (0..<n).map { f[$0 * 3] }, slotTop: (0..<n).map { f[$0 * 3 + 1] }, slotBottom: (0..<n).map { f[$0 * 3 + 2] },
                            fitScale: l.fit_scale, fitX: l.fit_x, fitY: l.fit_y,
                            reflowed: l.reflowed != 0, rows: Int(l.n_rows))
        currentLayout = out; return out
    }

    // ── a reflowed page: where the ink went ──
    /// Where each group of paths is placed. Without reflow there is one group per printed line,
    /// holding that line's shift; with reflow the groups are the page's words and decorations.
    public func layoutGroups() -> [QvpPlacement] {
        let f: [Float] = collect(1024) { o, c in qvp_layout_groups(p, o, c / 4) * 4 }
        return stride(from: 0, to: f.count, by: 4).map { QvpPlacement(dx: f[$0], dy: f[$0 + 1], kx: f[$0 + 2], ky: f[$0 + 3]) }
    }
    /// The group of every path. Empty without reflow, where a path's group is its printed line.
    public func layoutPathGroups() -> [Int] { collect(4096) { (o: UnsafeMutablePointer<UInt32>, c) in qvp_layout_path_groups(p, o, c) }.map(Int.init) }
    /// Paths this layout does not draw: the sheet's furniture on a reflowed page.
    public func layoutOmittedPaths() -> [Int] { collect(64) { (o: UnsafeMutablePointer<UInt32>, c) in qvp_layout_omitted_paths(p, o, c) }.map(Int.init) }
    /// Everything the current layout draws, in drawing order: each path once under the
    /// placement it belongs to, and again for every row a decoration is repeated over. A path
    /// this layout leaves out never appears, so one loop draws any page, printed or reflowed.
    /// `band` holds it to a band of the laid-out page, in viewport px, as `QvpLayout.slotTop`
    /// and every `…View` answer are: what keeps a page taller than the screen smooth under a
    /// finger. `nil` draws the whole page.
    public func layoutDrawList(band: (top: Float, bottom: Float)? = nil) -> [QvpDraw] {
        let (t, b) = band ?? (0, 0)
        let f: [UInt32] = collect(4096) { o, c in qvp_layout_draw_list(p, t, b, o, c / 2) * 2 }
        return stride(from: 0, to: f.count, by: 2).map { QvpDraw(path: Int(f[$0]), placement: Int(f[$0 + 1])) }
    }
    /// What a draw list's `placement` indexes: every group, then every repeat.
    public func layoutPlacements() -> [QvpPlacement] {
        let f: [Float] = collect(1024) { o, c in qvp_layout_placements(p, o, c / 4) * 4 }
        return stride(from: 0, to: f.count, by: 4).map { QvpPlacement(dx: f[$0], dy: f[$0 + 1], kx: f[$0 + 2], ky: f[$0 + 3]) }
    }
    /// Paths drawn again elsewhere: a sajdah line over the two rows its words landed on.
    public func layoutRepeats() -> [QvpRepeat] {
        let f: [Float] = collect(64) { o, c in qvp_layout_repeats(p, o, c / 6) * 6 }
        return stride(from: 0, to: f.count, by: 6).map {
            QvpRepeat(firstPath: Int(f[$0]), nPaths: Int(f[$0 + 1]), placement: QvpPlacement(dx: f[$0 + 2], dy: f[$0 + 3], kx: f[$0 + 4], ky: f[$0 + 5]))
        }
    }
    /// The words of a reflowed row, in reading order.
    public func rowWords(_ row: Int) -> [Int] { collect(64) { (o: UnsafeMutablePointer<UInt32>, c) in qvp_layout_row_words(p, UInt32(row), o, c) }.map(Int.init) }
    /// The largest reflow zoom at which every word of this page still fits a row.
    public func reflowMaxZoom(_ spec: QvpLayoutSpec) -> Float { var s = spec.c; return qvp_reflow_max_zoom(p, &s) }
    /// The zoom steps this page ships with, lowest first. Zoom 1, the printed page, is the step
    /// before them, so a control has one more position than this has entries.
    public func zoomSteps(_ spec: QvpLayoutSpec) -> [Float] {
        var s = spec.c
        var out = [Float](repeating: 0, count: 8)
        let n = out.withUnsafeMutableBufferPointer { qvp_zoom_steps(p, &s, $0.baseAddress!, 8) }
        return Array(out.prefix(Int(n)))
    }
    /// The `lineSpacing` multiplier that makes this page fill the padded viewport of `spec`; `max` 0 = unlimited.
    public func layoutLineSpacingToFill(_ spec: QvpLayoutSpec, max: Float = 0) -> Float { var s = spec.c; return qvp_layout_line_spacing_to_fill(p, &s, max) }
    /// The share of the padded viewport of `spec` left empty when the page is fitted to width.
    public func layoutWastedFraction(_ spec: QvpLayoutSpec) -> Float { var s = spec.c; return qvp_layout_wasted_fraction(p, &s) }
    /// The grid this page is laid out inside: the mushaf's line count and the printed line spacing.
    public var grid: QvpGrid { var g = QvpFFI.QvpGrid(); qvp_page_grid(p, &g); return QvpGrid(lines: Int(g.lines), lineSpacing: g.line_spacing) }
    /// A word's box in viewport px through the current layout (x0, y0, x1, y1).
    public func wordBoundsView(_ i: Int) -> (x0: Float, y0: Float, x1: Float, y1: Float)? {
        var b: (Float, Float, Float, Float) = (0, 0, 0, 0)
        let ok = withUnsafeMutablePointer(to: &b) { $0.withMemoryRebound(to: Float.self, capacity: 4) { qvp_word_bounds_view(p, UInt32(i), $0) } }
        return ok != 0 ? (b.0, b.1, b.2, b.3) : nil
    }

    // ── styles (handles undo exactly) ──
    @discardableResult public func style(_ sel: Selector, _ rgba: UInt32, transitionMs: Int = 0, layer: Int = QvpLayer.BASE) -> Int { var s = sel.c; return Int(qvp_style_add(p, Int32(layer), &s, rgba, UInt32(transitionMs))) }
    @discardableResult public func styleTarget(_ t: Target, _ rgba: UInt32, transitionMs: Int = 0, layer: Int = QvpLayer.BASE) -> Int { t.withC { Int(qvp_style_add_target(p, Int32(layer), $0, rgba, UInt32(transitionMs))) } }
    @discardableResult public func styleTarget(_ s: String, _ rgba: UInt32, transitionMs: Int = 0, layer: Int = QvpLayer.BASE) -> Int { styleTarget(target(s), rgba, transitionMs: transitionMs, layer: layer) }
    @discardableResult public func removeStyle(_ handle: Int) -> Int { Int(qvp_style_remove(p, UInt32(handle))) }
    @discardableResult public func recolorStyle(_ handle: Int, _ rgba: UInt32, transitionMs: Int = 0) -> Int { Int(qvp_style_recolor(p, UInt32(handle), rgba, UInt32(transitionMs))) }
    @discardableResult public func hide(_ sel: Selector) -> Int { var s = sel.c; return Int(qvp_style_hide(p, &s)) }
    public func clearStyles() { qvp_style_clear(p) }
    public func clearLayer(_ layer: Int) { qvp_style_clear_layer(p, Int32(layer)) }
    public func setDefaultColor(_ rgba: UInt32) { defaultInk = rgba; qvp_style_default_color(p, rgba) }
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
    public func colors() -> [UInt32] { Array(UnsafeBufferPointer(start: qvp_colors(p), count: nPaths)) }
    /// (path, colour) pairs for paths ≠ default ink.
    public func styledPaths() -> [(path: Int, color: UInt32)] {
        let v: [UInt32] = collect(512) { o, c in qvp_styled_paths(p, o, c / 2) * 2 }
        return stride(from: 0, to: v.count - 1, by: 2).map { (Int(v[$0]), v[$0 + 1]) }
    }
    public func colorOf(_ path: Int) -> UInt32 { qvp_color_of(p, UInt32(path)) }

    // ── highlights ──
    @discardableResult public func highlight(_ t: Target, _ style: QvpHighlightStyle = QvpHighlightStyle()) -> Int { var s = style.c; return t.withC { Int(qvp_highlight_add(p, $0, &s)) } }
    @discardableResult public func highlight(_ s: String, _ style: QvpHighlightStyle = QvpHighlightStyle()) -> Int { highlight(target(s), style) }
    @discardableResult public func moveHighlight(_ handle: Int, _ t: Target) -> Bool { t.withC { qvp_highlight_move(p, UInt32(handle), $0) != 0 } }
    @discardableResult public func moveHighlight(_ handle: Int, _ s: String) -> Bool { moveHighlight(handle, target(s)) }
    @discardableResult public func restyleHighlight(_ handle: Int, _ style: QvpHighlightStyle) -> Bool { var s = style.c; return qvp_highlight_restyle(p, UInt32(handle), &s) != 0 }
    @discardableResult public func removeHighlight(_ handle: Int) -> Bool { qvp_highlight_remove(p, UInt32(handle)) != 0 }
    public func clearHighlights() { qvp_highlight_clear(p) }
    public func highlightHandles() -> [Int] { collect(64) { o, c in qvp_highlight_handles(p, o, c) }.map { (h: UInt32) in Int(h) } }
    public func highlightWords(_ handle: Int) -> [Int] { collect(512) { o, c in qvp_highlight_words(p, UInt32(handle), o, c) }.map { (w: UInt32) in Int(w) } }
    private func boxes(_ v: [QvpFFI.QvpBox]) -> [QvpBox] { v.map { QvpBox(id: Int($0.id), line: Int($0.line), x0: $0.x0, y0: $0.y0, x1: $0.x1, y1: $0.y1, color: $0.color, radius: $0.radius) } }
    /// Animated band boxes in viewport px; draw each id as one nonzero path behind the ink.
    public func highlightBoxesView() -> [QvpBox] { boxes(collect(128) { o, c in qvp_highlight_boxes_view(p, o, c) }) }
    public func wordBands(_ ws: [Int], height: BandHeight = .lineSpacing, padX: Float = QvpDefaults.HIGHLIGHT_PAD_X, padY: Float = QvpDefaults.HIGHLIGHT_PAD_Y) -> [QvpBox] {
        ws.map { UInt32($0) }.withUnsafeBufferPointer { wb in boxes(collect(64) { o, c in qvp_word_bands(p, wb.baseAddress, UInt32(wb.count), UInt8(height.rawValue), padX, padY, o, c) }) }
    }

    // ── selection ──
    public func select(_ anchor: Int, _ focus: Int? = nil) { qvp_select(p, anchor < 0 ? QVP_NONE : UInt32(anchor), (focus ?? anchor) < 0 ? QVP_NONE : UInt32(focus ?? anchor)) }
    public func clearSelection() { qvp_select(p, QVP_NONE, QVP_NONE) }
    public func selection() -> [Int] { collect(512) { o, c in qvp_selection(p, o, c) }.map { (w: UInt32) in Int(w) } }
    public func selectionText(_ form: Form = .rasmUthmani, includeCitation: Bool = false) -> String { var s = QvpStr(); qvp_selection_text(p, UInt8(form.rawValue), includeCitation ? 1 : 0, &s); return s.string }

    // ── memorisation ──
    public func mask(_ t: Target, _ mode: MaskMode = .hide) { t.withC { qvp_mask(p, $0, UInt8(mode.rawValue)) } }
    public func mask(_ s: String, _ mode: MaskMode = .hide) { mask(target(s), mode) }
    public func maskFrom(_ wordIndex: Int, _ mode: MaskMode = .hide) { qvp_mask_from(p, UInt32(wordIndex), UInt8(mode.rawValue)) }
    public func maskOptions(blockColor: UInt32 = QvpDefaults.MASK_BLOCK, padX: Float = QvpDefaults.MASK_PAD, padY: Float = QvpDefaults.MASK_PAD, radius: Float = QvpDefaults.MASK_RADIUS, reverse: Bool = false) { qvp_mask_options(p, blockColor, padX, padY, radius, reverse ? 1 : 0) }
    /// Fade `.hide` words in and out over `ms` on the engine clock (0 = instant). Reset by `unmask()`.
    public func maskTransition(_ ms: Int) { qvp_mask_transition(p, UInt32(max(ms, 0))) }
    @discardableResult public func unmaskNext(_ n: Int = 1) -> Int { Int(qvp_unmask_next(p, UInt32(n))) }
    @discardableResult public func maskBack(_ n: Int = 1) -> Int { Int(qvp_mask_back(p, UInt32(n))) }
    @discardableResult public func unmaskWord(_ wordIndex: Int) -> Bool { qvp_unmask_word(p, UInt32(wordIndex)) != 0 }
    @discardableResult public func maskWord(_ wordIndex: Int) -> Bool { qvp_mask_word(p, UInt32(wordIndex)) != 0 }
    public func unmaskAll() { qvp_unmask_all(p) }
    public func maskAll() { qvp_mask_all(p) }
    public func unmask() { qvp_unmask(p) }
    public func maskHidden() -> [Int] { collect(512) { o, c in qvp_mask_hidden(p, o, c) }.map { (w: UInt32) in Int(w) } }
    public func maskWords() -> [Int] { collect(512) { o, c in qvp_mask_words(p, o, c) }.map { (w: UInt32) in Int(w) } }
    /// Block/blur boxes in viewport px, drawn over the ink.
    public func maskBoxesView() -> [QvpBox] { boxes(collect(128) { o, c in qvp_mask_boxes_view(p, o, c) }) }
    /// Greyed page with a lit window; returns steps.
    @discardableResult public func revealStart(lit: Int = QvpDefaults.REVEAL_LIT, byAyah: Bool = false, grey: UInt32 = QvpDefaults.REVEAL_GREY, ink: UInt32 = QvpDefaults.INK, ayahMarks: Bool = true, transitionMs: Int = 0) -> Int {
        Int(qvp_reveal_start(p, UInt32(lit), byAyah ? 1 : 0, grey, ink, ayahMarks ? 1 : 0, UInt32(transitionMs)))
    }
    @discardableResult public func revealGoto(_ at: Int) -> Bool { qvp_reveal_goto(p, Int64(at)) != 0 }
    /// Current step, -1 when nothing is lit yet, nil when no reveal is running.
    public func revealPosition() -> Int? { let v = qvp_reveal_position(p); return v == -2 ? nil : Int(v) }
    public func revealStepCount() -> Int { Int(qvp_reveal_step_count(p)) }
    public func revealStepOf(_ wordIndex: Int) -> Int { Int(qvp_reveal_step_of(p, UInt32(wordIndex))) }
    public func revealStop() { qvp_reveal_stop(p) }

    // ── crop ──
    public func cropBounds(_ t: Target, pad: Float = QvpDefaults.CROP_PAD, keepAyahMarks: Bool = true) -> QvpCropBounds? {
        var b = QvpFFI.QvpCropBounds()
        guard t.withC({ qvp_crop_bounds(p, $0, pad, keepAyahMarks ? 1 : 0, &b) }) != 0 else { return nil }
        return QvpCropBounds(x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, nWords: Int(b.n_words), ayahMarkDecoration: index(b.ayah_mark_decoration))
    }
    public func cropBounds(_ s: String, pad: Float = QvpDefaults.CROP_PAD, keepAyahMarks: Bool = true) -> QvpCropBounds? { cropBounds(target(s), pad: pad, keepAyahMarks: keepAyahMarks) }
    /// Standalone SVG with the current colours; background alpha 0 = transparent.
    public func cropSvg(_ t: Target, pad: Float = QvpDefaults.CROP_PAD, keepAyahMarks: Bool = true, background: UInt32 = 0) -> String? {
        var s = QvpStr()
        return t.withC({ qvp_crop_svg(p, $0, pad, keepAyahMarks ? 1 : 0, background, &s) }) != 0 ? s.string : nil
    }
    public func cropSvg(_ key: String, pad: Float = QvpDefaults.CROP_PAD, keepAyahMarks: Bool = true, background: UInt32 = 0) -> String? { cropSvg(target(key), pad: pad, keepAyahMarks: keepAyahMarks, background: background) }
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
    /// The engine handle. Internal: the wrapper marshals through it, no app ever sees it.
    var p: OpaquePointer { h! }
    private func surah(_ ok: Int32, _ s: QvpFFI.QvpAtlasSurah) -> QvpAtlasSurah? {
        ok != 0 ? QvpAtlasSurah(number: Int(s.number), page: Int(s.first_page), ayahCount: Int(s.ayah_count), place: place(s.place), arabic: s.arabic.string, latin: s.latin.string, english: s.english.string) : nil
    }
    public func pageOf(_ surah: Int, _ ayah: Int) -> Int? { let v = qvp_atlas_page_of(p, UInt16(surah), UInt16(ayah)); return v < 0 ? nil : Int(v) }
    /// ((first surah, first ayah), (last surah, last ayah)) printed on the page.
    public func pageRange(_ page: Int) -> (first: (Int, Int), last: (Int, Int))? {
        var r: (UInt16, UInt16, UInt16, UInt16) = (0, 0, 0, 0)
        let ok = withUnsafeMutablePointer(to: &r) { $0.withMemoryRebound(to: UInt16.self, capacity: 4) { qvp_atlas_page_range(p, UInt16(page), $0) } }
        return ok != 0 ? ((Int(r.0), Int(r.1)), (Int(r.2), Int(r.3))) : nil
    }
    public func pageCount() -> Int { Int(qvp_atlas_page_count(p)) }
    public func surah(_ n: Int) -> QvpAtlasSurah? { var s = QvpFFI.QvpAtlasSurah(); return surah(qvp_atlas_surah(p, UInt16(n), &s), s) }
    public func surahs() -> [QvpAtlasSurah] { (0..<Int(qvp_atlas_surah_count(p))).compactMap { var s = QvpFFI.QvpAtlasSurah(); return surah(qvp_atlas_surah_at(p, UInt32($0), &s), s) } }
    public func pageOfSurah(_ n: Int) -> Int? { surah(n)?.page }
    public func division(_ division: Division, _ n: Int) -> QvpAtlasRubuAlHizb? {
        var r = QvpFFI.QvpAtlasRubuAlHizb()
        return qvp_atlas_division(p, UInt8(division.rawValue), UInt16(n), &r) != 0 ? QvpAtlasRubuAlHizb(rubuAlHizb: Int(r.rubu_al_hizb), surah: Int(r.surah), ayah: Int(r.ayah), page: Int(r.page)) : nil
    }
    public func juz(_ n: Int) -> QvpAtlasRubuAlHizb? { division(.juz, n) }
    public func hizb(_ n: Int) -> QvpAtlasRubuAlHizb? { division(.hizb, n) }
    public func rubuAlHizb(_ n: Int) -> QvpAtlasRubuAlHizb? { division(.rubuAlHizb, n) }
    public func divisionOf(_ division: Division, _ surah: Int, _ ayah: Int) -> Int? { let v = qvp_atlas_division_of(p, UInt8(division.rawValue), UInt16(surah), UInt16(ayah)); return v < 0 ? nil : Int(v) }
    public func juzOf(_ surah: Int, _ ayah: Int) -> Int? { divisionOf(.juz, surah, ayah) }
    public func pagesOfJuz(_ n: Int) -> (Int, Int)? {
        var r: (UInt16, UInt16) = (0, 0)
        let ok = withUnsafeMutablePointer(to: &r) { $0.withMemoryRebound(to: UInt16.self, capacity: 2) { qvp_atlas_pages_of_juz(p, UInt16(n), $0) } }
        return ok != 0 ? (Int(r.0), Int(r.1)) : nil
    }
    /// 'cow' | 'البقرة' | '2' → matching surahs.
    public func searchSurahs(_ text: String) -> [QvpAtlasSurah] {
        let ns: [UInt16] = withBytes(text) { tp, tn in collect(128) { o, c in qvp_atlas_search_surahs(p, tp, tn, o, c) } }
        return ns.compactMap { surah(Int($0)) }
    }
    public func json() -> String { var s = QvpStr(); qvp_atlas_json(p, &s); return s.string }
}
