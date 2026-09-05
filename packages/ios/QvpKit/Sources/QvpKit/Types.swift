// Constants, selectors, targets and record types mirroring qvp.h / the Kotlin wrapper
// (packages/android/qvp/.../Types.kt). Colours everywhere are 0xRRGGBBAA (see QvpColor).
import Foundation
import CoreGraphics
import QvpFFI

/// Absent index in the C ABI.
public let QVP_NONE: UInt32 = 0xFFFF_FFFF

public enum QvpKind { public static let BODY = 0, MARK = 1, AYAH_NUMBER = 2, AYAH_ORNAMENT = 3, HEADER_INK = 4, OTHER = 255 }
public enum QvpFamily { public static let NONE = 0, DIACRITIC = 1, TANWEEN = 2, DOTS = 3, WAQF = 4, SIFR = 5, SAJDAH = 6, READING_SIGN = 7 }
public enum QvpCategory { public static let NONE = 0, HARAKA = 1, TANWEEN = 2, LETTER_DOT = 3, ORTHOGRAPHIC = 4, DABT = 5, WAQF = 6, READING_SIGN = 7, STANDALONE = 8 }
public enum QvpDeco { public static let AYAH_MARKER = 0, SURAH_NAME = 1, BASMALAH = 2, HIZB_MARK = 3, SAJDAH_MARK = 4 }
public enum QvpLayer { public static let BASE = 0, THEME = 10, HIGHLIGHT = 50, SELECTION = 60, TOP = 100 }

public enum Form: Int { case uthmani = 0, imlaei, qpc, rasm, search }
public enum SearchMode: Int { case includes = 0, exact, prefix }
public enum HighlightMode: Int { case ink = 0, band, both }
public enum BandHeight: Int { case pitch = 0, ink }
public enum MaskMode: Int { case hide = 0, block, blur }
public enum Division: Int, CaseIterable { case juz = 0, hizb, nisf, rub }

/// Mark ids by name (index = id; 255 = unknown).
public let QVP_MARKS: [String] = ["", "fatha", "kasra", "damma", "fathatan", "kasratan", "dammatan", "shadda", "sukun", "maddah", "hamza", "wasla", "small-alef", "small-waw", "small-ya", "small-noon", "dot", "two-dots", "three-dots", "sifr-mustadir", "sifr-mustatil", "waqf-jaiz", "waqf-awla", "wasl-awla", "waqf-lazim", "muanaqah", "saktah", "meem-iqlab", "hizb", "sajdah", "sajdah-sign", "sajdah-line", "seen-reading", "tashil", "ishmam", "imalah"]
public func markId(_ name: String) -> Int { QVP_MARKS.firstIndex(of: name) ?? 255 }

/// 0xRRGGBBAA helpers.
public enum QvpColor {
    /// "#rgb", "#rrggbb" or "#rrggbbaa" → 0xRRGGBBAA (alpha applies to 3/6-digit forms).
    public static func parse(_ hex: String, alpha: Float = 1) -> UInt32 {
        var h = hex.trimmingCharacters(in: .whitespaces)
        if h.hasPrefix("#") { h.removeFirst() }
        if h.count == 3 || h.count == 4 { h = h.map { "\($0)\($0)" }.joined() }
        if h.count == 6 { h += String(format: "%02x", Int((alpha * 255).rounded()).clamped(0, 255)) }
        return UInt32(h, radix: 16) ?? 0
    }
    public static func withAlpha(_ rgba: UInt32, _ alpha: Float) -> UInt32 {
        (rgba & 0xFFFF_FF00) | UInt32(Int((alpha * 255).rounded()).clamped(0, 255))
    }
    public static func components(_ rgba: UInt32) -> (r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat) {
        (CGFloat((rgba >> 24) & 255) / 255, CGFloat((rgba >> 16) & 255) / 255, CGFloat((rgba >> 8) & 255) / 255, CGFloat(rgba & 255) / 255)
    }
    public static func cgColor(_ rgba: UInt32) -> CGColor {
        let c = components(rgba)
        return CGColor(srgbRed: c.r, green: c.g, blue: c.b, alpha: c.a)
    }
    /// CGColor (sRGB / RGB / grey) → 0xRRGGBBAA.
    public static func rgba(_ color: CGColor) -> UInt32 {
        let c = color.converted(to: CGColorSpace(name: CGColorSpace.sRGB)!, intent: .defaultIntent, options: nil) ?? color
        let k = c.components ?? [0, 0, 0, 1]
        let (r, g, b, a) = k.count >= 4 ? (k[0], k[1], k[2], k[3]) : (k[0], k[0], k[0], k.count > 1 ? k[1] : 1)
        func u(_ v: CGFloat) -> UInt32 { UInt32(Int((v * 255).rounded()).clamped(0, 255)) }
        return (u(r) << 24) | (u(g) << 16) | (u(b) << 8) | u(a)
    }
}

extension Int { func clamped(_ lo: Int, _ hi: Int) -> Int { Swift.min(Swift.max(self, lo), hi) } }

/// What a style rule applies to (mirrors Sel in web/qvp.js and Selector in Kotlin).
public struct Selector {
    let c: QvpSelector
    init(_ kind: Int, _ a: Int = 0, _ b: Int = 0, _ cc: Int = 0) { c = QvpSelector(kind: UInt8(kind), a: UInt32(a), b: UInt32(b), c: UInt32(cc)) }
    public static func page() -> Selector { Selector(0) }
    public static func path(_ i: Int) -> Selector { Selector(1, i) }
    public static func wordPath(_ w: Int, _ nth: Int) -> Selector { Selector(2, w, nth) }
    /// nth mark of the word (0-based, marks only)
    public static func wordMark(_ w: Int, _ nth: Int) -> Selector { Selector(3, w, nth) }
    public static func wordMarkNamed(_ w: Int, _ mark: String, _ nth: Int = 0) -> Selector { Selector(4, w, markId(mark), nth) }
    public static func wordBody(_ w: Int) -> Selector { Selector(5, w) }
    public static func wordMarks(_ w: Int) -> Selector { Selector(6, w) }
    public static func word(_ w: Int) -> Selector { Selector(7, w) }
    public static func ayah(_ s: Int, _ a: Int) -> Selector { Selector(8, s, a) }
    public static func line(_ n: Int) -> Selector { Selector(9, n) }
    public static func mark(_ name: String) -> Selector { Selector(10, markId(name)) }
    public static func mark(_ id: Int) -> Selector { Selector(10, id) }
    public static func category(_ c: Int) -> Selector { Selector(11, c) }
    public static func family(_ f: Int) -> Selector { Selector(12, f) }
    public static func kind(_ k: Int) -> Selector { Selector(13, k) }
    public static func deco(_ k: Int) -> Selector { Selector(14, k) }
    public static func decoIdx(_ i: Int) -> Selector { Selector(15, i) }
}

/// What resolves to a word list (mirrors T in web/qvp.js and Target in Kotlin).
/// Strings: "page", "2:255", "2:255:3", "2:255-257", "line:7", "surah:2".
public struct Target {
    let kind: Int; let a: Int; let b: Int; let c: Int; let words: [UInt32]
    init(_ kind: Int, _ a: Int = 0, _ b: Int = 0, _ c: Int = 0, _ words: [UInt32] = []) { self.kind = kind; self.a = a; self.b = b; self.c = c; self.words = words }
    public static func page() -> Target { Target(0) }
    public static func word(_ i: Int) -> Target { Target(1, i) }
    public static func words(_ ws: [Int]) -> Target { Target(2, 0, 0, 0, ws.map { UInt32($0) }) }
    public static func ayah(_ s: Int, _ a: Int) -> Target { Target(3, s, a) }
    public static func ayahRange(_ s: Int, _ a: Int, _ b: Int) -> Target { Target(4, s, a, b) }
    public static func line(_ n: Int) -> Target { Target(5, n) }
    public static func surah(_ s: Int) -> Target { Target(6, s) }
    public static func range(_ a: Int, _ b: Int) -> Target { Target(7, a, b) }
    /// Parse a string key; "s:a:w" word keys need the page to resolve (see QvpPage.target).
    public static func parse(_ s: String, page: QvpPage? = nil) -> Target {
        if s == "page" { return Target.page() }
        func nums(_ pattern: String) -> [Int]? {
            guard let re = try? NSRegularExpression(pattern: pattern), let m = re.firstMatch(in: s, range: NSRange(s.startIndex..., in: s)) else { return nil }
            return (1..<m.numberOfRanges).compactMap { Range(m.range(at: $0), in: s).flatMap { Int(s[$0]) } }
        }
        if let g = nums("^line:(\\d+)$") { return line(g[0]) }
        if let g = nums("^surah:(\\d+)$") { return surah(g[0]) }
        if let g = nums("^(\\d+):(\\d+)-(\\d+)$") { return ayahRange(g[0], g[1], g[2]) }
        if let g = nums("^(\\d+):(\\d+):(\\d+)$") { let i = page?.findWord(g[0], g[1], g[2]) ?? -1; return i >= 0 ? word(i) : words([]) }
        if let g = nums("^(\\d+):(\\d+)$") { return ayah(g[0], g[1]) }
        preconditionFailure("bad target \(s)")
    }
    /// Run `body` with a C QvpTarget whose `words` pointer is valid for the call.
    func withC<R>(_ body: (UnsafePointer<QvpTarget>) -> R) -> R {
        words.withUnsafeBufferPointer { wp in
            var t = QvpTarget(kind: UInt8(kind), a: UInt32(a), b: UInt32(b), c: UInt32(c), words: wp.baseAddress, n_words: UInt32(wp.count))
            return withUnsafePointer(to: &t, body)
        }
    }
}

public struct QvpWord: Equatable {
    public let idx: Int, sura: Int, ayah: Int, word: Int, line: Int, ayahIdx: Int, lineIdx: Int
    public let x0: Float, y0: Float, x1: Float, y1: Float
    public let text: String, firstPath: Int, nPaths: Int
    public var wid: String { "\(sura):\(ayah):\(word)" }
    public var aid: String { "\(sura):\(ayah)" }
}
public struct QvpAyah: Equatable {
    public let idx: Int, sura: Int, ayah: Int, part: Int, parts: Int, flags: Int, rub: Int, firstWord: Int, nWords: Int, markerDeco: Int
    public let x0: Float, y0: Float, x1: Float, y1: Float
}
public struct QvpLine: Equatable {
    public let idx: Int, lineNo: Int, isHeader: Bool, firstWord: Int, nWords: Int
    public let x0: Float, y0: Float, x1: Float, y1: Float, bandY0: Float, bandY1: Float, centre: Float
}
public struct QvpDecoration: Equatable {
    public let idx: Int, kind: Int, sura: Int, ayah: Int, line: Int
    public let x0: Float, y0: Float, x1: Float, y1: Float
    public let text: String, firstPath: Int, nPaths: Int
}
/// Indices are -1 when absent.
public struct QvpHit: Equatable { public let word: Int, path: Int, deco: Int }
public struct QvpHitEx: Equatable { public let word: Int, path: Int, deco: Int, line: Int, distance: Float, exact: Bool }
public struct QvpHitOptions: Equatable {
    public var maxDistance: Float, gapBias: Float, exactFirst: Bool
    public init(maxDistance: Float = 0, gapBias: Float = 0.6, exactFirst: Bool = true) { self.maxDistance = maxDistance; self.gapBias = gapBias; self.exactFirst = exactFirst }
}
public struct QvpBox: Equatable { public let id: Int, line: Int, x0: Float, y0: Float, x1: Float, y1: Float, color: UInt32, radius: Float }
public struct QvpHitBox: Equatable { public let word: Int, line: Int, x0: Float, y0: Float, x1: Float, y1: Float, inkX0: Float, inkY0: Float, inkX1: Float, inkY1: Float }
public struct QvpLineBand: Equatable { public let line: Int, lineNo: Int, y0: Float, y1: Float, mid: Float, inkY0: Float, inkY1: Float }
public struct QvpLayoutSpec: Equatable {
    public var viewportW: Float, viewportH: Float, padTop: Float, padBottom: Float, padLeft: Float, padRight: Float, lineSpacing: Float, lineGap: Float, fillHeight: Bool, nominalLines: Int
    public init(viewportW: Float, viewportH: Float, padTop: Float = 0, padBottom: Float = 0, padLeft: Float = 0, padRight: Float = 0, lineSpacing: Float = 1, lineGap: Float = 0, fillHeight: Bool = false, nominalLines: Int = 15) {
        self.viewportW = viewportW; self.viewportH = viewportH; self.padTop = padTop; self.padBottom = padBottom; self.padLeft = padLeft; self.padRight = padRight
        self.lineSpacing = lineSpacing; self.lineGap = lineGap; self.fillHeight = fillHeight; self.nominalLines = nominalLines
    }
}
/// Page → viewport: vx = ox + x*scale ; vy = oy + (y + lineDy[line])*scale.
public final class QvpLayout {
    public let scale: Float, ox: Float, oy: Float, contentW: Float, contentH: Float, pitch: Float
    public let lineDy: [Float], slotTop: [Float], slotBottom: [Float]
    init(scale: Float, ox: Float, oy: Float, contentW: Float, contentH: Float, pitch: Float, lineDy: [Float], slotTop: [Float], slotBottom: [Float]) {
        self.scale = scale; self.ox = ox; self.oy = oy; self.contentW = contentW; self.contentH = contentH; self.pitch = pitch; self.lineDy = lineDy; self.slotTop = slotTop; self.slotBottom = slotBottom
    }
}
public struct QvpHighlightStyle: Equatable {
    public var mode: HighlightMode, ink: UInt32, band: UInt32, height: BandHeight, padX: Float, padY: Float, radius: Float, seam: Float, transitionMs: Int, layer: Int
    public init(mode: HighlightMode = .band, ink: UInt32 = 0x1a73e8ff, band: UInt32 = 0xd6a3264d, height: BandHeight = .pitch, padX: Float = 1.2, padY: Float = 0, radius: Float = 0, seam: Float = 0.25, transitionMs: Int = 0, layer: Int = QvpLayer.HIGHLIGHT) {
        self.mode = mode; self.ink = ink; self.band = band; self.height = height; self.padX = padX; self.padY = padY; self.radius = radius; self.seam = seam; self.transitionMs = transitionMs; self.layer = layer
    }
    var c: QvpFFI.QvpHighlightStyle {
        QvpFFI.QvpHighlightStyle(mode: UInt8(mode.rawValue), height: UInt8(height.rawValue), ink: ink, band: band, pad_x: padX, pad_y: padY, radius: radius, seam: seam, transition_ms: UInt32(transitionMs), layer: Int32(layer))
    }
}
/// Colours that are nil (or alpha 0) leave that part alone.
public struct QvpTheme: Equatable {
    public var ink: UInt32?, diacritics: UInt32?, dots: UInt32?, waqf: UInt32?, sifr: UInt32?, marker: UInt32?, numeral: UInt32?, headers: UInt32?
    public var marks: [String: UInt32], transitionMs: Int
    public init(ink: UInt32? = nil, diacritics: UInt32? = nil, dots: UInt32? = nil, waqf: UInt32? = nil, sifr: UInt32? = nil, marker: UInt32? = nil, numeral: UInt32? = nil, headers: UInt32? = nil, marks: [String: UInt32] = [:], transitionMs: Int = 0) {
        self.ink = ink; self.diacritics = diacritics; self.dots = dots; self.waqf = waqf; self.sifr = sifr; self.marker = marker; self.numeral = numeral; self.headers = headers; self.marks = marks; self.transitionMs = transitionMs
    }
}
public struct QvpSurah: Equatable { public let number: Int, ayahCount: Int, hasBanner: Bool, hasBasmalah: Bool, place: String, bannerDeco: Int, arabic: String, latin: String, english: String }
public struct QvpDivision: Equatable { public let kind: Division, n: Int, sura: Int, ayah: Int, line: Int, ayahIdx: Int }
public struct QvpMarker: Equatable { public let deco: Int, sura: Int, ayah: Int, line: Int, cx: Float, cy: Float, r: Float, ornamentPath: Int, numeralPath: Int }
public struct QvpRosette: Equatable { public let deco: Int, sura: Int, ayah: Int, juz: Int, hizb: Int, nisf: Int, rub: Int, rubInHizb: Int }
public struct QvpSajdah: Equatable { public let deco: Int, sura: Int, ayah: Int, signPath: Int }
public struct QvpMatch: Equatable { public let word: Int, index: Int, loose: Bool, wid: String, text: String }
public struct QvpCropBox: Equatable { public let x0: Float, y0: Float, x1: Float, y1: Float, nWords: Int, markerDeco: Int }
public struct QvpAtlasSurah: Equatable { public let n: Int, page: Int, ayahCount: Int, place: String, arabic: String, latin: String, english: String }
public struct QvpAtlasRub: Equatable { public let rub: Int, sura: Int, ayah: Int, page: Int; public var aid: String { "\(sura):\(ayah)" } }

// ── C interop helpers ──
extension QvpStr {
    var string: String {
        guard let p = ptr, len > 0 else { return "" }
        return String(decoding: UnsafeBufferPointer(start: p, count: Int(len)), as: UTF8.self)
    }
}
func idx(_ v: UInt32) -> Int { v == QVP_NONE ? -1 : Int(v) }
func place(_ p: UInt8) -> String { p == 0 ? "makkah" : p == 1 ? "madinah" : "" }
/// Call an `(out, cap) → total` C function, growing the buffer until everything fits.
func collect<T>(_ initial: Int = 256, _ f: (UnsafeMutablePointer<T>, UInt32) -> UInt32) -> [T] {
    var cap = initial
    while true {
        var total: UInt32 = 0
        let buf = [T](unsafeUninitializedCapacity: cap) { p, n in total = f(p.baseAddress!, UInt32(cap)); n = Swift.min(Int(total), cap) }
        if Int(total) <= cap { return buf }
        cap = Int(total)
    }
}
func withBytes<R>(_ s: String, _ body: (UnsafePointer<UInt8>, UInt32) -> R) -> R {
    var b = Array(s.utf8); if b.isEmpty { b = [0] }
    let n = UInt32(s.utf8.count)
    return b.withUnsafeBufferPointer { body($0.baseAddress!, n) }
}
