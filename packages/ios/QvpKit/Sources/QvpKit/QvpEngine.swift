// Engine-wide helpers (names, Arabic text tools, layout maths). Mirrors `object QvpEngine` in Kotlin.
import Foundation
import QvpFFI

public enum QvpEngine {
    public static func version() -> Int { Int(qvp_version()) }
    public static func engineName() -> String { String(cString: qvp_engine_name()) }
    public static func markName(_ m: Int) -> String { var s = QvpStr(); qvp_mark_name(UInt8(clamping: m), &s); return s.string }
    public static func familyName(_ f: Int) -> String { var s = QvpStr(); qvp_family_name(UInt8(clamping: f), &s); return s.string }
    public static func kindName(_ k: Int) -> String { var s = QvpStr(); qvp_kind_name(UInt8(clamping: k), &s); return s.string }
    public static func categoryName(_ c: Int) -> String { var s = QvpStr(); qvp_category_name(UInt8(clamping: c), &s); return s.string }
    public static func markFromName(_ name: String) -> Int { withBytes(name) { p, n in Int(qvp_mark_from_name(p, n)) } }
    public static func markCategory(_ m: Int) -> Int { Int(qvp_mark_category(UInt8(clamping: m))) }
    public static func strip(_ s: String) -> String { arabic(0, s) }
    public static func fold(_ s: String) -> String { arabic(1, s) }
    public static func normalize(_ s: String) -> String { arabic(2, s) }
    public static func looseKey(_ s: String) -> String { arabic(3, s) }
    private static func arabic(_ kind: Int, _ s: String) -> String { withBytes(s) { p, n in var o = QvpStr(); qvp_arabic(UInt8(kind), p, n, &o); return o.string } }
    public static func gapToFill(pageW: Float, pageH: Float, lines: Int, viewW: Float, viewH: Float, max: Float = 0) -> Float { qvp_gap_to_fill(pageW, pageH, UInt32(lines), viewW, viewH, max) }
    public static func wastedFraction(pageW: Float, pageH: Float, viewW: Float, viewH: Float) -> Float { qvp_wasted_fraction(pageW, pageH, viewW, viewH) }
}
