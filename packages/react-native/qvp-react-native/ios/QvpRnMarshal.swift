// @quran.ws/qvp-react-native, iOS. Marshalling only: JS shapes (docs/API.md) to QvpKit types and
// back. No hit test, layout or style decision lives here. Every call goes to QvpPage, QvpAtlas or
// QvpOrnaments, which go to the engine. This file mirrors android/.../Marshal.kt key for key, so
// the two platforms hand JavaScript the same shapes.
//
// Targets from JS: "page" | "2:255" | "2:255:3" | "2:255-257" | "line:7" | "surah:2" | word index |
// [word indices] | {target, a, b, c, words} (target = QVP_TARGET_* or its name in QvpRnCodes).
// Selectors from JS: {selector, a, b, c} (selector = QVP_SELECTOR_* or its name in QvpRnCodes).
// Colours: '#rgb' | '#rrggbb' | '#rrggbbaa' | 0xRRGGBBAA.
//
// QvpKit's sources compile into this pod's module, so its types need no import.
import Foundation

enum QvpRnMarshal {
    // ── JS values in ──
    /// The value, or nil for a missing value and for JS null.
    static func some(_ v: Any?) -> Any? { v is NSNull ? nil : v }
    static func num(_ v: Any?) -> Int? {
        switch some(v) {
        case let n as NSNumber: return n.intValue
        case let s as String: return Int(s)
        default: return nil
        }
    }
    static func flt(_ v: Any?, _ d: Float) -> Float {
        switch some(v) {
        case let n as NSNumber: return n.floatValue
        case let s as String: return Float(s) ?? d
        default: return d
        }
    }
    static func bool(_ v: Any?, _ d: Bool) -> Bool { (some(v) as? NSNumber)?.boolValue ?? d }
    /// True only for JS `true`, as Kotlin's `v == true`.
    static func isTrue(_ v: Any?) -> Bool { (some(v) as? NSNumber)?.boolValue == true }
    /// False only for JS `false`, as Kotlin's `v == false`: a missing key is not false.
    static func isFalse(_ v: Any?) -> Bool { (some(v) as? NSNumber).map { !$0.boolValue } ?? false }
    static func ints(_ v: Any?) -> [Int] {
        switch some(v) {
        case let a as [Any]: return a.compactMap { num($0) }
        case let n as NSNumber: return [n.intValue]
        default: return []
        }
    }
    static func dict(_ v: Any?) -> [String: Any]? { some(v) as? [String: Any] }
    static func str(_ v: Any?) -> String? { some(v) as? String }

    // ── colours ──
    /// A colour as 0xRRGGBBAA, or `d` when the value is missing or does not parse.
    static func color(_ v: Any?, _ d: UInt32) -> UInt32 {
        switch some(v) {
        case let s as String:
            let h = s.trimmingCharacters(in: .whitespaces)
            let digits = h.hasPrefix("#") ? String(h.dropFirst()) : h
            // QvpColor.parse reads a bad string as 0; Kotlin's throws and the default stands.
            guard [3, 4, 6, 8].contains(digits.count), digits.allSatisfy({ $0.isASCII && $0.isHexDigit }) else { return d }
            return QvpColor.parse(h)
        case let n as NSNumber: return UInt32(truncatingIfNeeded: n.int64Value)
        default: return d
        }
    }
    /// A colour, or nil when it is missing or transparent black.
    static func colorOrNil(_ v: Any?) -> UInt32? {
        guard some(v) != nil else { return nil }
        let c = color(v, 0)
        return c == 0 ? nil : c
    }

    /// A code given as a number, or by its name in `names` (or as a numeral); -1 when neither.
    private static func code(_ v: Any?, _ names: [String: Int]) -> Int {
        switch some(v) {
        case let n as NSNumber: return n.intValue
        case let s as String: return names[s] ?? Int(s) ?? -1
        default: return -1
        }
    }

    /// A JS target as a QvpKit Target. An unknown one is the empty word list.
    static func target(_ v: Any?, _ page: QvpPage?) -> Target {
        switch some(v) {
        case nil: return .page()
        case let s as String: return Target(parsing: s, page: page) ?? .words([])
        case let n as NSNumber: return .word(n.intValue)
        case let a as [Any]: return .words(ints(a))
        case let m as [String: Any]:
            let a = num(m["a"]) ?? 0, b = num(m["b"]) ?? 0, c = num(m["c"]) ?? 0
            switch code(m["target"], QvpRnCodes.TARGETS) {
            case QvpRnCodes.TARGET_PAGE: return .page()
            case QvpRnCodes.TARGET_WORD: return .word(a)
            case QvpRnCodes.TARGET_WORDS: return .words(ints(some(m["words"]) ?? m["a"]))
            case QvpRnCodes.TARGET_AYAH: return .ayah(a, b)
            case QvpRnCodes.TARGET_AYAH_RANGE: return .ayahRange(a, b, c)
            case QvpRnCodes.TARGET_LINE: return .line(a)
            case QvpRnCodes.TARGET_SURAH: return .surah(a)
            case QvpRnCodes.TARGET_RANGE: return .range(a, b)
            default: return .words([])
            }
        default: return .words([])
        }
    }

    /// A JS selector {selector, a, b, c} as a QvpKit Selector, or nil when it is malformed.
    static func selector(_ v: Any?) -> Selector? {
        guard let m = dict(v) else { return nil }
        let a = num(m["a"]) ?? 0, b = num(m["b"]) ?? 0, c = num(m["c"]) ?? 0
        switch code(m["selector"], QvpRnCodes.SELECTORS) {
        case QvpRnCodes.SELECTOR_PAGE: return .page()
        case QvpRnCodes.SELECTOR_PATH: return .path(a)
        case QvpRnCodes.SELECTOR_WORD_PATH: return .wordPath(a, b)
        case QvpRnCodes.SELECTOR_WORD_MARK: return .wordMark(a, b)
        case QvpRnCodes.SELECTOR_WORD_MARK_NAMED: return .wordMarkNamed(a, str(m["mark"]) ?? QvpEngine.markName(b), c)
        case QvpRnCodes.SELECTOR_WORD_BODY: return .wordBody(a)
        case QvpRnCodes.SELECTOR_WORD_MARKS: return .wordMarks(a)
        case QvpRnCodes.SELECTOR_WORD: return .word(a)
        case QvpRnCodes.SELECTOR_AYAH: return .ayah(a, b)
        case QvpRnCodes.SELECTOR_LINE: return .line(a)
        case QvpRnCodes.SELECTOR_MARK: return str(m["mark"]).map { Selector.mark($0) } ?? .mark(a)
        case QvpRnCodes.SELECTOR_CATEGORY: return .category(a)
        case QvpRnCodes.SELECTOR_FAMILY: return .family(a)
        case QvpRnCodes.SELECTOR_KIND: return .kind(a)
        case QvpRnCodes.SELECTOR_DECORATION: return .decoration(a)
        case QvpRnCodes.SELECTOR_DECORATION_INDEX: return .decorationIndex(a)
        default: return nil
        }
    }

    /// The case of an Int-backed enum that `v` names, in any of its spellings: the Kotlin
    /// constant's (RASM_IMLAI), snake case (rasm_imlai) or camelCase (rasmImlai). A number is the
    /// raw value.
    static func named<E: RawRepresentable>(_ v: Any?, _ d: E) -> E where E.RawValue == Int {
        switch some(v) {
        case let n as NSNumber: return E(rawValue: n.intValue) ?? d
        case let s as String:
            let key = s.replacingOccurrences(of: "_", with: "").lowercased()
            var i = 0
            while let e = E(rawValue: i) {
                if String(describing: e).lowercased() == key { return e }
                i += 1
            }
            return d
        default: return d
        }
    }
    /// A case's name as the TypeScript types spell it: `rasmImlai`.
    static func jsName<E>(_ e: E) -> String { String(describing: e) }
    /// A case's name as Kotlin's lower-cased constant spells it: `rubuAlHizb` is `rubu_al_hizb`.
    static func snakeName<E>(_ e: E) -> String {
        String(describing: e).reduce(into: "") { out, ch in
            if ch.isUppercase { out += "_" + ch.lowercased() } else { out.append(ch) }
        }
    }

    static func form(_ v: Any?, _ d: Form = .rasmUthmani) -> Form { named(v, d) }
    static func searchMode(_ v: Any?) -> SearchMode { named(v, .includes) }
    static func maskMode(_ v: Any?) -> MaskMode { named(v, .hide) }
    static func division(_ v: Any?) -> Division { named(v, .juz) }

    /// {mode, ink, band, height, padX, padY, radius, seam, ms, layer}. A missing key keeps the library default.
    static func highlightStyle(_ v: Any?) -> QvpHighlightStyle {
        let d = QvpHighlightStyle()
        guard let m = dict(v) else { return d }
        return QvpHighlightStyle(mode: named(m["mode"], d.mode), ink: color(m["ink"], d.ink), band: color(m["band"], d.band),
                                 height: named(m["height"], d.height), padX: flt(m["padX"], d.padX), padY: flt(m["padY"], d.padY),
                                 radius: flt(m["radius"], d.radius), seam: flt(m["seam"], d.seam),
                                 transitionMs: num(some(m["ms"]) ?? m["transitionMs"]) ?? d.transitionMs, layer: num(m["layer"]) ?? d.layer)
    }
    /// {ink, diacritics, dots, waqf, sifr, ayahMark, numeral, headers, marks: {name: colour}, ms}.
    static func theme(_ v: Any?) -> QvpTheme {
        guard let m = dict(v) else { return QvpTheme() }
        var marks: [String: UInt32] = [:]
        for (k, c) in dict(m["marks"]) ?? [:] { if let c = colorOrNil(c) { marks[k] = c } }
        return QvpTheme(ink: colorOrNil(m["ink"]), diacritics: colorOrNil(m["diacritics"]), dots: colorOrNil(m["dots"]), waqf: colorOrNil(m["waqf"]),
                        sifr: colorOrNil(m["sifr"]), ayahMark: colorOrNil(m["ayahMark"]), numeral: colorOrNil(m["numeral"]), headers: colorOrNil(m["headers"]),
                        marks: marks, transitionMs: num(some(m["ms"]) ?? m["transitionMs"]) ?? 0)
    }

    // ── dress ──
    /// 'ayah_mark' | 'surah_header' | 'page_frame' (or QVP_ORNAMENT_*), or nil.
    static func ornamentKind(_ v: Any?) -> OrnamentKind? {
        switch some(v) {
        case let s as String: return OrnamentKind.allCases.first { $0.name == s.lowercased() }
        case let n as NSNumber: return OrnamentKind(rawValue: n.intValue)
        default: return nil
        }
    }
    /// One kind of a JS spec: an index, a style name, or null for none. JS has already put the
    /// style in each kind, so a missing kind is none and not "take the style".
    private static func ornamentRef(_ v: Any?) -> OrnamentChoice {
        switch some(v) {
        case let n as NSNumber: return .index(n.intValue)
        case let s as String: return .style(s)
        default: return .absent
        }
    }
    /// {ayahMark, surahHeader, pageFrame, gap, bandGap, ayahMarkScale, pageFrameScale, lineArt: [kind],
    /// colors: [[ornament, part, colour]]}. A missing number keeps the library's default.
    static func dressSpec(_ v: Any?) -> DressSpec {
        let d = DressSpec()
        guard let m = dict(v) else { return d }
        let lineArt = Set(((some(m["lineArt"]) as? [Any]) ?? []).compactMap { ornamentKind($0) })
        let colors: [OrnamentColor] = ((some(m["colors"]) as? [Any]) ?? []).compactMap { t in
            guard let l = t as? [Any], l.count >= 2, let o = num(l[0]), let part = num(l[1]) else { return nil }
            return OrnamentColor(ornament: o, part: part, color: color(l.count > 2 ? l[2] : nil, 0))
        }
        return DressSpec(style: nil, ayahMark: ornamentRef(m["ayahMark"]), surahHeader: ornamentRef(m["surahHeader"]), pageFrame: ornamentRef(m["pageFrame"]),
                         gap: flt(m["gap"], d.gap), bandGap: flt(m["bandGap"], d.bandGap), ayahMarkScale: flt(m["ayahMarkScale"], d.ayahMarkScale),
                         pageFrameScale: flt(m["pageFrameScale"], d.pageFrameScale), lineArt: lineArt, colors: colors)
    }
    static func ornamentPart(_ p: OrnamentPart) -> [String: Any?] {
        ["index": p.index, "color": p.color, "stroke": p.stroke, "name": p.name]
    }
    static func ornament(_ o: Ornament) -> [String: Any?] {
        ["index": o.index, "kind": o.kind.name, "lineage": o.lineage.name, "style": o.style, "riwayah": o.riwayah, "hasSlices": o.hasSlices,
         "licence": ["id": o.licence.id, "status": o.licence.status, "redistributable": o.licence.redistributable, "attribution": o.licence.attribution] as [String: Any?],
         "parts": o.parts.map { ornamentPart($0) }]
    }
    static func dressInfo(_ i: DressInfo) -> [String: Any?] {
        ["ayahMark": i.ayahMark, "surahHeader": i.surahHeader, "pageFrame": i.pageFrame,
         "ayahMarks": i.ayahMarks, "surahHeaders": i.surahHeaders, "frameRepeats": i.frameRepeats, "frameStretched": i.frameStretched,
         "draws": i.draws, "revision": i.revision, "viewBox": i.viewBox, "overflow": i.overflow]
    }

    // ── QvpKit to JS ──
    /// The engine's name of every id of the path and decoration tables, read once. A page has
    /// hundreds of paths, and each names its kind, mark, category and family: a lookup here is an
    /// array read, not four calls into the engine. Ids run 0 to 255, so the table covers every id
    /// a path can hold.
    private struct Names {
        let kind = (0..<256).map { QvpEngine.kindName($0) }
        let mark = (0..<256).map { QvpEngine.markName($0) }
        let category = (0..<256).map { QvpEngine.categoryName($0) }
        let family = (0..<256).map { QvpEngine.familyName($0) }
        let decoration = (0..<256).map { (k: Int) -> String in let n = QvpEngine.decorationName(k); return n.isEmpty ? "other" : n }
    }
    private static let names = Names()

    /// The forms a page has: rasmUthmani always, and each other form the words sidecar gave.
    static func forms(_ p: QvpPage) -> [Form] {
        var out: [Form] = []
        var i = 0
        while let f = Form(rawValue: i) { if f == .rasmUthmani || p.hasForm(f) { out.append(f) }; i += 1 }
        return out
    }

    /// One word and its paths. `forms` is `forms(p)`, read once by the caller.
    static func word(_ p: QvpPage, _ w: QvpWord, _ forms: [Form]) -> [String: Any?] {
        let n = names
        var text: [String: Any?] = [:]
        for f in forms { text[jsName(f)] = p.wordForm(w.index, f) }
        let paths: [[String: Any?]] = (w.firstPath..<(w.firstPath + w.nPaths)).map { i in
            let kind = p.pathKind(i), mark = p.pathMark(i), cat = p.pathCategory(i), fam = p.pathFamily(i)
            return ["index": i, "kind": kind, "kindName": n.kind[kind], "mark": mark, "markName": n.mark[mark], "nthMark": p.pathNthMark(i),
                    "nthInWord": p.pathNthInWord(i), "category": cat, "categoryName": n.category[cat], "family": fam, "familyName": n.family[fam], "line": p.pathLine(i)]
        }
        return ["index": w.index, "surah": w.surah, "ayah": w.ayah, "word": w.word, "line": w.line, "lineIndex": w.lineIndex, "ayahIndex": w.ayahIndex,
                "x0": w.x0, "y0": w.y0, "x1": w.x1, "y1": w.y1, "text": w.text, "firstPath": w.firstPath, "nPaths": w.nPaths,
                "wordKey": w.wordKey, "ayahKey": w.ayahKey, "forms": text, "label": p.wordLabel(w.index), "paths": paths]
    }
    static func decoration(_ d: QvpDecoration) -> [String: Any?] {
        ["index": d.index, "decoration": d.decoration, "decorationName": names.decoration[d.decoration & 0xff],
         "surah": d.surah, "ayah": d.ayah, "line": d.line, "x0": d.x0, "y0": d.y0, "x1": d.x1, "y1": d.y1, "text": d.text, "firstPath": d.firstPath, "nPaths": d.nPaths]
    }
    static func hit(_ p: QvpPage, _ h: QvpHit) -> [String: Any?] {
        let w = h.word >= 0 && h.word < p.words.count ? p.words[h.word] : nil
        return ["word": h.word, "path": h.path, "decoration": h.decoration, "line": h.line, "distance": h.distance, "isExact": h.isExact,
                "wordKey": w?.wordKey, "ayahKey": w?.ayahKey]
    }
    static func ayah(_ a: QvpAyah) -> [String: Any?] {
        ["index": a.index, "surah": a.surah, "ayah": a.ayah, "fragment": a.fragment, "fragments": a.fragments, "flags": a.flags, "rubuAlHizb": a.rubuAlHizb,
         "firstWord": a.firstWord, "nWords": a.nWords, "ayahMarkDecoration": a.ayahMarkDecoration, "bbox": [a.x0, a.y0, a.x1, a.y1]]
    }
    static func line(_ l: QvpLine) -> [String: Any?] {
        ["index": l.index, "lineNumber": l.lineNumber, "isHeader": l.isHeader, "firstWord": l.firstWord, "nWords": l.nWords, "bbox": [l.x0, l.y0, l.x1, l.y1],
         "bandY0": l.bandY0, "bandY1": l.bandY1, "centre": l.centre]
    }
    static func surah(_ s: QvpSurah) -> [String: Any?] {
        ["number": s.number, "ayahCount": s.ayahCount, "hasBanner": s.hasBanner, "hasBasmalah": s.hasBasmalah, "place": s.place,
         "bannerDecoration": s.bannerDecoration, "arabic": s.arabic, "latin": s.latin, "english": s.english]
    }
    static func division(_ d: QvpDivision) -> [String: Any?] {
        ["division": snakeName(d.division), "number": d.number, "surah": d.surah, "ayah": d.ayah, "line": d.line, "ayahIndex": d.ayahIndex]
    }
    static func surahHeader(_ h: QvpSurahHeader) -> [String: Any?] {
        ["decoration": h.decoration, "surah": h.surah, "line": h.line, "x0": h.x0, "y0": h.y0, "x1": h.x1, "y1": h.y1,
         "titleX0": h.titleX0, "titleY0": h.titleY0, "titleX1": h.titleX1, "titleY1": h.titleY1]
    }
    static func ayahMark(_ m: QvpAyahMark) -> [String: Any?] {
        ["decoration": m.decoration, "surah": m.surah, "ayah": m.ayah, "line": m.line, "cx": m.cx, "cy": m.cy, "r": m.r,
         "ornamentPath": m.ornamentPath, "numeralPath": m.numeralPath]
    }
    static func rosette(_ r: QvpRosette) -> [String: Any?] {
        ["decoration": r.decoration, "surah": r.surah, "ayah": r.ayah, "juz": r.juz, "hizb": r.hizb, "nisf": r.nisf,
         "rubuAlHizb": r.rubuAlHizb, "rubuAlHizbInHizb": r.rubuAlHizbInHizb]
    }
    static func sajdah(_ s: QvpSajdah) -> [String: Any?] { ["decoration": s.decoration, "surah": s.surah, "ayah": s.ayah, "signPath": s.signPath] }
    static func match(_ m: QvpMatch) -> [String: Any?] { ["word": m.word, "index": m.index, "isLooseMatch": m.isLooseMatch, "wordKey": m.wordKey, "text": m.text] }
    static func passageLayout(_ l: QvpPassageLayout, _ p: QvpPassage) -> [String: Any?] {
        let e = l.ellipsis
        return ["width": l.width, "height": l.height, "scale": l.scale, "lineSpacing": l.lineSpacing, "printedLineSpacing": p.lineSpacing,
                "isTruncated": l.isTruncated, "ellipsis": e.map { [$0.minX, $0.minY, $0.maxX, $0.maxY] },
                "rows": l.rows.map { ["x0": $0.x0, "y0": $0.y0, "x1": $0.x1, "y1": $0.y1, "baseline": $0.baseline] as [String: Any?] },
                "words": l.words.map { ["surah": $0.surah, "ayah": $0.ayah, "word": $0.word, "wordKey": $0.wordKey, "page": $0.page, "row": $0.row,
                                        "x0": $0.x0, "y0": $0.y0, "x1": $0.x1, "y1": $0.y1,
                                        "wordX0": $0.wordX0, "wordY0": $0.wordY0, "wordX1": $0.wordX1, "wordY1": $0.wordY1] as [String: Any?] },
                "ayahs": l.ayahs.map { ["surah": $0.surah, "ayah": $0.ayah, "x0": $0.x0, "y0": $0.y0, "x1": $0.x1, "y1": $0.y1] as [String: Any?] },
                "texts": p.ayahs.map { ["surah": $0.surah, "ayah": $0.ayah, "text": $0.text] as [String: Any?] }]
    }
    static func cropBounds(_ c: QvpCropBounds) -> [String: Any?] {
        ["x0": c.x0, "y0": c.y0, "x1": c.x1, "y1": c.y1, "nWords": c.nWords, "ayahMarkDecoration": c.ayahMarkDecoration]
    }
    static func atlasSurah(_ s: QvpAtlasSurah) -> [String: Any?] {
        ["number": s.number, "page": s.page, "ayahCount": s.ayahCount, "place": s.place, "arabic": s.arabic, "latin": s.latin, "english": s.english]
    }
    static func atlasRubuAlHizb(_ r: QvpAtlasRubuAlHizb) -> [String: Any?] {
        ["rubuAlHizb": r.rubuAlHizb, "surah": r.surah, "ayah": r.ayah, "page": r.page, "ayahKey": r.ayahKey]
    }
    static func layout(_ l: QvpLayout) -> [String: Any?] {
        ["scale": l.scale, "offsetX": l.offsetX, "offsetY": l.offsetY, "contentW": l.contentW, "contentH": l.contentH, "lineSpacing": l.lineSpacing,
         "fitScale": l.fitScale, "fitX": l.fitX, "fitY": l.fitY, "lineDy": l.lineDy,
         "slots": l.slotTop.indices.map { [l.slotTop[$0], l.slotBottom[$0]] }]
    }
    static func pageInfo(_ p: QvpPage, _ forms: [Form]) -> [String: Any?] {
        ["page": p.pageNo, "width": p.width, "height": p.height, "nLines": p.nLines, "nAyahs": p.nAyahs, "nWords": p.nWords, "nPaths": p.nPaths,
         "nDecorations": p.nDecorations, "lineSpacing": p.lineSpacing, "forms": forms.map { jsName($0) }]
    }
    static func selection(_ p: QvpPage) -> [String: Any?] {
        let ws = p.selection()
        return ["words": ws, "text": ws.isEmpty ? "" : p.text(.words(ws)), "citation": ws.isEmpty ? "" : p.citation(ws),
                "textWithCitation": ws.isEmpty ? "" : p.selectionText(.rasmUthmani, includeCitation: true)]
    }

    /// Any Swift value (dictionaries, arrays, numbers, strings, optionals) as a bridge value:
    /// nil is NSNull, a Float is a Double, an enum is its lower-case name.
    static func toJs(_ v: Any?) -> Any {
        guard let v = flatten(v) else { return NSNull() }
        switch v {
        case let d as [String: Any?]: return d.mapValues { toJs($0) }
        case let d as [String: Any]: return d.mapValues { toJs($0) }
        case let a as [Any?]: return a.map { toJs($0) }
        case let a as [Any]: return a.map { toJs($0) }
        case let b as Bool: return b
        case let i as Int: return i
        case let u as UInt32: return NSNumber(value: u)
        case let f as Float: return Double(f)
        case let d as Double: return d
        case let c as CGFloat: return Double(c)
        case let s as String: return s
        case let n as NSNumber: return n
        case is NSNull: return NSNull()
        default: return String(describing: v).lowercased()
        }
    }
    /// The value inside any depth of Optional, or nil when one of them is empty.
    private static func flatten(_ v: Any?) -> Any? {
        guard let v else { return nil }
        let m = Mirror(reflecting: v)
        guard m.displayStyle == .optional else { return v }
        return m.children.first.flatMap { flatten($0.value) }
    }
    static func toJsMap(_ m: [String: Any?]) -> [String: Any] { toJs(m) as? [String: Any] ?? [:] }
}
