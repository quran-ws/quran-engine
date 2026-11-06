// Engine-wide helpers (names, Arabic text tools, layout maths). Mirrors `object QvpEngine` in Kotlin.
import Foundation
import QvpFFI

/// The engine-wide calls: names, the Arabic text tools, and the versions.
public enum QvpEngine {
    /// The engine version, e.g. `0.2.0`.
    public static func version() -> String { var s = QvpStr(); qvp_version(&s); return s.string }
    /// The page format version the engine reads.
    public static func formatVersion() -> Int { Int(qvp_format_version()) }
    /// The engine's name, `qvp`.
    public static func engineName() -> String { String(cString: qvp_engine_name()) }
    /// A mark's name by its id.
    public static func markName(_ m: Int) -> String { var s = QvpStr(); qvp_mark_name(UInt8(clamping: m), &s); return s.string }
    /// A mark family's name by its id.
    public static func familyName(_ f: Int) -> String { var s = QvpStr(); qvp_family_name(UInt8(clamping: f), &s); return s.string }
    /// A path kind's name by its id.
    public static func kindName(_ k: Int) -> String { var s = QvpStr(); qvp_kind_name(UInt8(clamping: k), &s); return s.string }
    /// A mark category's name by its id.
    public static func categoryName(_ c: Int) -> String { var s = QvpStr(); qvp_category_name(UInt8(clamping: c), &s); return s.string }
    /// A mark's id by its name; 255 when the engine has no such mark.
    public static func markFromName(_ name: String) -> Int { withBytes(name) { p, n in Int(qvp_mark_from_name(p, n)) } }
    /// The category a mark belongs to.
    public static func markCategory(_ m: Int) -> Int { Int(qvp_mark_category(UInt8(clamping: m))) }
    /// The engine's name tables (QVP_NAMES_*): no wrapper carries a table of its own.
    public enum Names {
        /// The table ids, by name.
        public static let mark = 0, kind = 1, family = 2, category = 3, decoration = 4, division = 5, place = 6
    }
    /// How many ids a name table has.
    public static func nameCount(_ table: Int) -> Int { Int(qvp_name_count(UInt8(clamping: table))) }
    /// The name of an id in a table; empty outside the table.
    public static func name(_ table: Int, _ id: Int) -> String { var s = QvpStr(); qvp_name(UInt8(clamping: table), UInt8(clamping: id), &s); return s.string }
    /// 255 when the table has no such name.
    public static func nameId(_ table: Int, _ name: String) -> Int { withBytes(name) { p, n in Int(qvp_name_id(UInt8(clamping: table), p, n)) } }
    /// Every name of a table, index = id.
    public static func names(_ table: Int) -> [String] { (0..<nameCount(table)).map { name(table, $0) } }
    /// A decoration kind's name by its id.
    public static func decorationName(_ k: Int) -> String { name(Names.decoration, k) }
    /// A division's name by its id.
    public static func divisionName(_ d: Int) -> String { name(Names.division, d) }
    /// A place of revelation's name by its id.
    public static func placeName(_ p: Int) -> String { name(Names.place, p) }
    /// Remove every Arabic mark (harakah, tatweel, waqf signs and the like) and keep the letters.
    public static func strip(_ s: String) -> String { arabic(0, s) }
    /// Fold the letter forms that search treats as one: أإآٱ to ا, ىئ to ي, ؤ to و, ة to ه,
    /// and Arabic-Indic digits to ASCII.
    public static func fold(_ s: String) -> String { arabic(1, s) }
    /// Strip, fold and collapse the spaces, as search does to both the query and the page.
    public static func normalize(_ s: String) -> String { arabic(2, s) }
    /// Normalise, then drop bare alif and hamzah: the key of the loose second search pass, so a
    /// typed `الرحمان` finds `الرحمن`.
    public static func looseKey(_ s: String) -> String { arabic(3, s) }
    private static func arabic(_ op: Int, _ s: String) -> String { withBytes(s) { p, n in var o = QvpStr(); qvp_arabic(UInt8(op), p, n, &o); return o.string } }
}
