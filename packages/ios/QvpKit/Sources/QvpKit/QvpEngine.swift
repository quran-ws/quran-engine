// Engine-wide helpers (names, Arabic text tools, layout maths). Mirrors `object QvpEngine` in Kotlin.
import Foundation
import QvpFFI

public enum QvpEngine {
    /// The engine version, e.g. `0.2.0`.
    public static func version() -> String { var s = QvpStr(); qvp_version(&s); return s.string }
    /// The page format version the engine reads.
    public static func formatVersion() -> Int { Int(qvp_format_version()) }
    public static func engineName() -> String { String(cString: qvp_engine_name()) }
    public static func markName(_ m: Int) -> String { var s = QvpStr(); qvp_mark_name(UInt8(clamping: m), &s); return s.string }
    public static func familyName(_ f: Int) -> String { var s = QvpStr(); qvp_family_name(UInt8(clamping: f), &s); return s.string }
    public static func kindName(_ k: Int) -> String { var s = QvpStr(); qvp_kind_name(UInt8(clamping: k), &s); return s.string }
    public static func categoryName(_ c: Int) -> String { var s = QvpStr(); qvp_category_name(UInt8(clamping: c), &s); return s.string }
    public static func markFromName(_ name: String) -> Int { withBytes(name) { p, n in Int(qvp_mark_from_name(p, n)) } }
    public static func markCategory(_ m: Int) -> Int { Int(qvp_mark_category(UInt8(clamping: m))) }
    /// The engine's name tables (QVP_NAMES_*): no wrapper carries a table of its own.
    public enum Names { public static let mark = 0, kind = 1, family = 2, category = 3, decoration = 4, division = 5, place = 6 }
    public static func nameCount(_ table: Int) -> Int { Int(qvp_name_count(UInt8(clamping: table))) }
    public static func name(_ table: Int, _ id: Int) -> String { var s = QvpStr(); qvp_name(UInt8(clamping: table), UInt8(clamping: id), &s); return s.string }
    /// 255 when the table has no such name.
    public static func nameId(_ table: Int, _ name: String) -> Int { withBytes(name) { p, n in Int(qvp_name_id(UInt8(clamping: table), p, n)) } }
    /// Every name of a table, index = id.
    public static func names(_ table: Int) -> [String] { (0..<nameCount(table)).map { name(table, $0) } }
    public static func decorationName(_ k: Int) -> String { name(Names.decoration, k) }
    public static func divisionName(_ d: Int) -> String { name(Names.division, d) }
    public static func placeName(_ p: Int) -> String { name(Names.place, p) }
    public static func strip(_ s: String) -> String { arabic(0, s) }
    public static func fold(_ s: String) -> String { arabic(1, s) }
    public static func normalize(_ s: String) -> String { arabic(2, s) }
    public static func looseKey(_ s: String) -> String { arabic(3, s) }
    private static func arabic(_ op: Int, _ s: String) -> String { withBytes(s) { p, n in var o = QvpStr(); qvp_arabic(UInt8(op), p, n, &o); return o.string } }
}
