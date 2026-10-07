// A range of complete ayahs laid out on rows away from its page, over the C ABI in qvp.h.
// The engine measures and places every word; this file marshals the calls. See docs/API.md.
import Foundation
import CoreGraphics
import QvpFFI

/// Where the rows of a passage start.
public enum QvpPassageAlign: UInt8, Sendable {
    /// At the right edge, as Arabic text starts.
    case right = 0
    /// In the middle of the width.
    case center = 1
}

/// What a layout of a passage asks for. Lengths are layout points.
public struct QvpPassageSpec: Equatable, Sendable {
    /// The width of the rows. Zero or less puts the whole passage on one row, as wide as it is.
    public var width: Float
    /// Layout points per page unit. It shrinks only when one word and its signs are wider than
    /// the rows. To size a passage by its printed line, divide by `QvpPassage.lineSpacing`.
    public var scale: Float
    /// The distance between two rows' baselines, as a multiple of the printed one. At least 1.
    public var lineSpacing: Float
    /// The empty margin on every side.
    public var padding: Float
    /// Where each row starts.
    public var align: QvpPassageAlign
    /// The most rows to show. Zero shows every row.
    public var maxRows: Int
    /// After a cut, place the medallion of the passage's last ayah after the ellipsis.
    public var keepAyahMark: Bool
    /// The width that the host needs for its ellipsis after a cut.
    public var ellipsisWidth: Float

    /// Make a spec; the defaults put the passage on one row at the printed size.
    public init(width: Float = 0, scale: Float = 1, lineSpacing: Float = 1, padding: Float = 2, align: QvpPassageAlign = .right,
                maxRows: Int = 0, keepAyahMark: Bool = false, ellipsisWidth: Float = 0) {
        self.width = width; self.scale = scale; self.lineSpacing = lineSpacing; self.padding = padding; self.align = align
        self.maxRows = maxRows; self.keepAyahMark = keepAyahMark; self.ellipsisWidth = ellipsisWidth
    }
}

/// One row of a laid-out passage, in layout points.
public struct QvpPassageRow: Equatable {
    /// The box of the row's ink, and the height its words sit on.
    public let x0: Float, y0: Float, x1: Float, y1: Float, baseline: Float
}

/// One word of a laid-out passage with the signs that go with it, in layout points.
public struct QvpPassageWord: Equatable {
    /// The word's key and the page it is printed on.
    public let surah: Int, ayah: Int, word: Int, page: Int
    /// The row the word is on, from 0.
    public let row: Int
    /// The box of the word and its signs.
    public let x0: Float, y0: Float, x1: Float, y1: Float
}

/// The bounds of one ayah that a layout shows, in layout points.
public struct QvpPassageAyah: Equatable {
    /// The ayah, and the box of its shown words.
    public let surah: Int, ayah: Int, x0: Float, y0: Float, x1: Float, y1: Float
}

/// One path that a passage layout draws: its page, its index on that page, and which of
/// `QvpPassageLayout.placements` it goes under.
public struct QvpPassageDraw: Equatable {
    /// The page number. Draw the path with that page's geometry (`QvpPage.buildPaths()`).
    public let page: Int
    /// The path's index on its page.
    public let path: Int
    /// The index of its placement.
    public let placement: Int
}

/// A laid-out passage. A point `(x, y)` of a path draws at
/// `scale · (kx · x + dx), scale · (ky · y + dy)`, with the path's placement.
public struct QvpPassageLayout: Equatable {
    /// The size of the layout, padding included, and the scale it was laid out at.
    public let width: Float, height: Float, scale: Float
    /// The distance between two rows' baselines, page units, before a row moves down for its ink.
    public let lineSpacing: Float
    /// The rows, top to bottom.
    public let rows: [QvpPassageRow]
    /// The words shown, in reading order.
    public let words: [QvpPassageWord]
    /// The ayahs with at least one word shown.
    public let ayahs: [QvpPassageAyah]
    /// Every path to draw, in drawing order.
    public let draws: [QvpPassageDraw]
    /// What `QvpPassageDraw.placement` indexes.
    public let placements: [QvpPlacement]
    /// The layout shows fewer words than the passage has.
    public let isTruncated: Bool
    /// Where the host draws its ellipsis after a cut: from the last row's top to its baseline.
    public let ellipsis: CGRect?
}

/// One ayah of a passage, with its text from the page's own word records.
public struct QvpPassageText: Equatable {
    /// The ayah and its text.
    public let surah: Int, ayah: Int, text: String
}

/// A contiguous range of complete ayahs, ready to lay out at any width. It copies what it
/// needs from its pages, so the pages can be closed after it is made.
public final class QvpPassage {
    private var h: OpaquePointer?

    /// The ayahs in reading order.
    public let ayahs: [QvpPassageText]
    /// The printed line spacing of the passage's pages, in page units.
    public let lineSpacing: Float
    /// The last layout.
    public private(set) var currentLayout: QvpPassageLayout?

    /// Load the ayahs `from` to `to` of `surah` from the pages that print them, in any order.
    /// Throws when an ayah is missing or incomplete, a page is given twice, or the range has more
    /// than 4,096 words.
    public init(pages: [QvpPage], surah: Int, from: Int, to: Int? = nil) throws {
        let to = to ?? from
        // Every engine call on a closed page traps; the engine checks the range itself.
        guard pages.allSatisfy(\.isOpen), (1...Int(UInt16.max)).contains(surah), (1...Int(UInt16.max)).contains(from),
              (1...Int(UInt16.max)).contains(to) else { throw QvpError.badPassage }
        let handles: [OpaquePointer?] = pages.map { $0.p }
        h = handles.withUnsafeBufferPointer {
            qvp_passage_load($0.baseAddress, UInt32($0.count), UInt16(surah), UInt16(from), UInt16(to))
        }
        guard let h else { throw QvpError.badPassage }
        ayahs = (0..<Int(qvp_passage_ayah_count(h))).map { i in
            var s = QvpStr(); qvp_passage_ayah_text(h, UInt32(i), &s)
            return QvpPassageText(surah: surah, ayah: from + i, text: s.string)
        }
        lineSpacing = qvp_passage_line_spacing(h)
    }
    deinit { close() }
    /// Free the native passage. Safe to call more than once.
    public func close() { if let p = h { qvp_passage_free(p); h = nil } }
    /// False once `close()` has run.
    public var isOpen: Bool { h != nil }

    /// Lay the passage out. `nil` when the spec is out of range; the last layout then stays.
    @discardableResult
    public func layout(_ spec: QvpPassageSpec) -> QvpPassageLayout? {
        guard let p = h, spec.maxRows >= 0 else { return nil }
        var s = QvpFFI.QvpPassageSpec(width: spec.width, scale: spec.scale, line_spacing: spec.lineSpacing, padding: spec.padding,
                                       align: spec.align.rawValue, keep_ayah_mark: spec.keepAyahMark ? 1 : 0, _pad: (0, 0),
                                       max_rows: UInt32(spec.maxRows), ellipsis_width: spec.ellipsisWidth)
        var out = QvpFFI.QvpPassageLayout()
        guard qvp_passage_layout(p, &s, &out) != 0 else { return nil }
        let rows: [QvpFFI.QvpPassageRow] = collect(max(1, Int(out.n_rows))) { o, c in qvp_passage_rows(p, o, c) }
        let words: [QvpFFI.QvpPassageWord] = collect(max(1, Int(out.n_words))) { o, c in qvp_passage_words(p, o, c) }
        let ayahs: [QvpFFI.QvpPassageAyah] = collect(max(1, Int(out.n_ayahs))) { o, c in qvp_passage_ayahs(p, o, c) }
        let draws: [UInt32] = collect(max(3, Int(out.n_draws) * 3)) { o, c in qvp_passage_draw_list(p, o, c / 3) * 3 }
        let placements: [Float] = collect(64) { o, c in qvp_passage_placements(p, o, c / 4) * 4 }
        let layout = QvpPassageLayout(
            width: out.width, height: out.height, scale: out.scale, lineSpacing: out.line_spacing,
            rows: rows.map { QvpPassageRow(x0: $0.x0, y0: $0.y0, x1: $0.x1, y1: $0.y1, baseline: $0.baseline) },
            words: words.map { QvpPassageWord(surah: Int($0.surah), ayah: Int($0.ayah), word: Int($0.word), page: Int($0.page), row: Int($0.row),
                                              x0: $0.x0, y0: $0.y0, x1: $0.x1, y1: $0.y1) },
            ayahs: ayahs.map { QvpPassageAyah(surah: Int($0.surah), ayah: Int($0.ayah), x0: $0.x0, y0: $0.y0, x1: $0.x1, y1: $0.y1) },
            draws: stride(from: 0, to: draws.count, by: 3).map { QvpPassageDraw(page: Int(draws[$0]), path: Int(draws[$0 + 1]), placement: Int(draws[$0 + 2])) },
            placements: stride(from: 0, to: placements.count, by: 4).map {
                QvpPlacement(dx: placements[$0], dy: placements[$0 + 1], kx: placements[$0 + 2], ky: placements[$0 + 3])
            },
            isTruncated: out.is_truncated != 0,
            ellipsis: out.is_truncated != 0
                ? CGRect(x: CGFloat(out.ellipsis_x0), y: CGFloat(out.ellipsis_y0),
                         width: CGFloat(out.ellipsis_x1 - out.ellipsis_x0), height: CGFloat(out.ellipsis_y1 - out.ellipsis_y0))
                : nil)
        currentLayout = layout
        return layout
    }
}
