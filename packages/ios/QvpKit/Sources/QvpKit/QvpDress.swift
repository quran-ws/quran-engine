// Dress: another mushaf's ornaments on a page, over the C ABI in qvp.h. The engine places
// every medallion, band and border and hands back a display list. This file only marshals
// it. Mirrors `QvpOrnaments` and `QvpPage.dress` in web/qvp.js. See docs/API.md, "Dress".
//
// The binding calls qvp_ornaments_load, qvp_ornaments_free, qvp_ornament_count,
// qvp_ornament_info, qvp_ornament_find, qvp_ornament_part, qvp_dress, qvp_undress,
// qvp_dress_info, qvp_dress_geometry, qvp_dress_clamp and qvp_dress_view.
import Foundation
import CoreGraphics
import QvpFFI

/// What an ornament replaces (QVP_ORNAMENT_*). `name` is the web wrapper's string for it.
public enum OrnamentKind: Int, CaseIterable, Hashable {
    case ayahMark = 0, surahHeader, pageFrame
    public var name: String { ["ayah_mark", "surah_header", "page_frame"][rawValue] }
    /// The QVP_LINE_ART_* bit of this kind.
    var lineArtBit: UInt32 { 1 << UInt32(rawValue) }
}

/// Where an ornament's artwork came from (QVP_LINEAGE_*).
public enum OrnamentLineage: Int, Hashable {
    case scan = 0, font
    public var name: String { self == .scan ? "scan" : "font" }
}

/// What an ornament may be published under. Read it before you publish a page wearing it.
public struct OrnamentLicence: Equatable {
    public let id: String, status: String, redistributable: Bool, attribution: String
}

/// One printed colour of an ornament. A part with alpha 0 is a window the design leaves open.
public struct OrnamentPart: Equatable {
    public let index: Int, color: UInt32, stroke: Bool, name: String
}

/// One ornament of a set: a medallion, a band or a border.
public struct Ornament: Equatable {
    public let index: Int, kind: OrnamentKind, lineage: OrnamentLineage
    public let style: String, riwayah: String, hasSlices: Bool
    public let licence: OrnamentLicence
    public let parts: [OrnamentPart]
}

/// Every style in a set, with the ornament it has of each kind, or nil.
public struct OrnamentStyle: Equatable {
    public let style: String, riwayah: String, lineage: OrnamentLineage
    public var ayahMark: Ornament?, surahHeader: Ornament?, pageFrame: Ornament?
}

/// The ornaments of other mushafs and fonts, from `ornaments.qvo`. None of these outlines is
/// part of a QVP page. Each ornament says what it may be redistributed under.
public final class QvpOrnaments {
    private var h: OpaquePointer?
    /// How many ornaments the set holds.
    public let count: Int
    /// Every ornament, in the set's order. `list[i].index == i`.
    public let list: [Ornament]

    /// Load an ornament set. The engine copies the bytes.
    public init(bytes: Data) throws {
        h = bytes.withUnsafeBytes { qvp_ornaments_load($0.bindMemory(to: UInt8.self).baseAddress, $0.count) }
        guard let h else { throw QvpError.badOrnaments }
        count = Int(qvp_ornament_count(h))
        var out: [Ornament] = []
        for i in 0..<count {
            var o = QvpOrnament()
            guard qvp_ornament_info(h, UInt32(i), &o) != 0 else { continue }
            // the strings point into the set and live as long as it: copy them now
            let parts: [OrnamentPart] = (0..<Int(o.n_parts)).compactMap { k in
                var q = QvpOrnamentPart()
                guard qvp_ornament_part(h, UInt32(i), UInt32(k), &q) != 0 else { return nil }
                return OrnamentPart(index: k, color: q.color, stroke: q.stroke != 0, name: q.name.string)
            }
            out.append(Ornament(index: i, kind: OrnamentKind(rawValue: Int(o.kind)) ?? .ayahMark, lineage: OrnamentLineage(rawValue: Int(o.lineage)) ?? .scan,
                                style: o.style.string, riwayah: o.riwayah.string, hasSlices: o.has_slices != 0,
                                licence: OrnamentLicence(id: o.license_id.string, status: o.license_status.string, redistributable: o.redistributable != 0, attribution: o.attribution.string),
                                parts: parts))
        }
        list = out
    }
    /// Load an ornament set from a file.
    public convenience init(path: String) throws { try self.init(bytes: try Data(contentsOf: URL(fileURLWithPath: path))) }
    deinit { close() }
    /// Free the native set. Safe to call more than once. A page dressed from it keeps its dress.
    public func close() { if let s = h { qvp_ornaments_free(s); h = nil } }
    public var isOpen: Bool { h != nil }
    /// The engine handle. Internal: the wrapper marshals through it, no app ever sees it.
    var p: OpaquePointer { h! }

    /// The ornament of `kind` in `style`, or nil.
    public func find(_ kind: OrnamentKind, _ style: String) -> Ornament? {
        guard let s = h else { return nil }
        let i = withBytes(style) { sp, sn in qvp_ornament_find(s, UInt32(kind.rawValue), sp, sn) }
        return i < 0 ? nil : list.first { $0.index == Int(i) }
    }
    /// The ornaments of one kind.
    public func ofKind(_ kind: OrnamentKind) -> [Ornament] { list.filter { $0.kind == kind } }
    /// Every style in the set, in the set's order, with the ornament it has of each kind.
    public func styles() -> [OrnamentStyle] {
        var out: [OrnamentStyle] = []
        for o in list {
            var k = out.firstIndex { $0.style == o.style }
            if k == nil { out.append(OrnamentStyle(style: o.style, riwayah: o.riwayah, lineage: o.lineage)); k = out.count - 1 }
            switch o.kind {
            case .ayahMark: out[k!].ayahMark = o
            case .surahHeader: out[k!].surahHeader = o
            case .pageFrame: out[k!].pageFrame = o
            }
        }
        return out
    }
}

/// Which ornament a dress puts on one kind: an ornament of the set, its index, a style name
/// (the ornament of that kind in that style), or `.absent` for none.
public enum OrnamentChoice: Equatable, ExpressibleByStringLiteral, ExpressibleByIntegerLiteral {
    case ornament(Ornament), index(Int), style(String), absent
    public init(stringLiteral value: String) { self = .style(value) }
    public init(integerLiteral value: Int) { self = .index(value) }
}

/// A repaint of one part of one ornament. `color` is 0xRRGGBBAA.
public struct OrnamentColor: Equatable {
    public var ornament: Int, part: Int, color: UInt32
    public init(ornament: Int, part: Int, color: UInt32) { self.ornament = ornament; self.part = part; self.color = color }
    public init(_ ornament: Ornament, part: Int, color: UInt32) { self.init(ornament: ornament.index, part: part, color: color) }
}

/// What to dress a page in. Mirrors the web wrapper's dress options.
public struct DressSpec: Equatable {
    /// A mushaf name: its medallion, band and border together. A kind set below overrides it.
    public var style: String?
    /// One ornament per kind. nil takes the kind from `style`; `.absent` leaves it out.
    public var ayahMark: OrnamentChoice?, surahHeader: OrnamentChoice?, pageFrame: OrnamentChoice?
    /// The air between the text and the border, and between a band and the border, in page units.
    public var gap: Float, bandGap: Float
    /// The medallion's size against the printed ring. 0 takes the design's own:
    /// `QvpDefaults.AYAH_MARK_SCALE_SCAN` for a scanned mushaf, 1 for a font marker.
    public var ayahMarkScale: Float
    /// The border's size against the design's own. The pattern scales as a whole.
    public var pageFrameScale: Float
    /// The kinds drawn as line art, their constant-width strokes alone. `Set(OrnamentKind.allCases)` for all.
    public var lineArt: Set<OrnamentKind>
    /// Repaints. Every part left out keeps the design's own colour.
    public var colors: [OrnamentColor]
    public init(style: String? = nil, ayahMark: OrnamentChoice? = nil, surahHeader: OrnamentChoice? = nil, pageFrame: OrnamentChoice? = nil,
                gap: Float = 5, bandGap: Float = 5, ayahMarkScale: Float = 0, pageFrameScale: Float = 1,
                lineArt: Set<OrnamentKind> = [], colors: [OrnamentColor] = []) {
        self.style = style; self.ayahMark = ayahMark; self.surahHeader = surahHeader; self.pageFrame = pageFrame
        self.gap = gap; self.bandGap = bandGap; self.ayahMarkScale = ayahMarkScale; self.pageFrameScale = pageFrameScale
        self.lineArt = lineArt; self.colors = colors
    }
}

/// The readout of a dressed page.
public struct DressInfo: Equatable {
    /// The ornament dressed on each kind, or nil.
    public let ayahMark: Int?, surahHeader: Int?, pageFrame: Int?
    /// How many medallions and bands were placed, and how the border was built.
    public let ayahMarks: Int, surahHeaders: Int, frameRepeats: Int, frameStretched: Bool
    /// How many draws the display list holds.
    public let draws: Int
    /// Counts up on every rebuild: a new dress or a new layout. Cache the ornament raster on it.
    public let revision: Int
    /// x, y, w, h in page units. A border grows it past the page.
    public let viewBox: [Float]
    /// left, top, right, bottom in viewport px: how far the border reaches past the laid-out
    /// content. Fit content plus overflow, or the border is cropped.
    public let overflow: [Float]
}

/// One draw of the ornament display list, in page units. Paint it behind the page ink.
public struct OrnamentDraw {
    public let path: CGPath
    public let color: UInt32
    public let kind: OrnamentKind
    /// Fill with the even-odd rule.
    public let evenOdd: Bool
    /// Stroke with `strokeWidth` (page units), round caps and joins, instead of filling.
    public let stroke: Bool
    public let strokeWidth: Float
    public let part: Int
    /// The index into `QvpPage.layoutPlacements()` the draw goes under, as a page draw does.
    /// nil for the border, which is drawn in layout space with the identity placement.
    public let placement: Int?
}

extension QvpPage {
    /// Put other ornaments on the page: up to one per kind. Returns the readout, or nil when
    /// the engine refused the spec (an index out of range, an ornament of another kind, or
    /// nothing to dress). Replaces any previous dress.
    @discardableResult
    public func dress(_ set: QvpOrnaments, _ spec: DressSpec = DressSpec()) -> DressInfo? {
        guard set.isOpen else { return nil }
        // Resolve every name before the C spec is filled.
        func index(_ c: OrnamentChoice?, _ kind: OrnamentKind) -> UInt32 {
            let choice = c ?? spec.style.map { OrnamentChoice.style($0) } ?? .absent
            switch choice {
            case .ornament(let o): return UInt32(o.index)
            case .index(let i): return i < 0 ? QVP_NONE : UInt32(i)
            case .style(let s): return set.find(kind, s).map { UInt32($0.index) } ?? QVP_NONE
            case .absent: return QVP_NONE
            }
        }
        let ayah = index(spec.ayahMark, .ayahMark), header = index(spec.surahHeader, .surahHeader), frame = index(spec.pageFrame, .pageFrame)
        let lineArt = spec.lineArt.reduce(UInt32(0)) { $0 | $1.lineArtBit }
        let triples: [UInt32] = spec.colors.flatMap { [UInt32($0.ornament), UInt32($0.part), $0.color] }
        dressDraws = nil
        // the triples stay alive across the call
        let ok = triples.withUnsafeBufferPointer { tp in
            var s = QvpDressSpec(ayah_mark: ayah, surah_header: header, page_frame: frame, gap: spec.gap, band_gap: spec.bandGap,
                                 ayah_mark_scale: spec.ayahMarkScale, page_frame_scale: spec.pageFrameScale, line_art: lineArt,
                                 colors: triples.isEmpty ? nil : tp.baseAddress, n_colors: UInt32(triples.count / 3))
            return qvp_dress(p, set.p, &s)
        }
        // the dress changed the layout's fit: read it again
        readLayout()
        return ok != 0 ? dressInfo() : nil
    }
    /// Take the ornaments off: the printed rings and bands come back.
    public func undress() { qvp_undress(p); dressDraws = nil; readLayout() }
    /// The readout, or nil when the page is not dressed.
    public func dressInfo() -> DressInfo? {
        var d = QvpDressInfo()
        guard qvp_dress_info(p, &d) != 0 else { return nil }
        let id = { (v: UInt32) -> Int? in v == QVP_NONE ? nil : Int(v) }
        return DressInfo(ayahMark: id(d.ayah_mark), surahHeader: id(d.surah_header), pageFrame: id(d.page_frame),
                         ayahMarks: Int(d.n_ayah_marks), surahHeaders: Int(d.n_surah_headers), frameRepeats: Int(d.n_frame_repeats),
                         frameStretched: d.frame_stretched != 0, draws: Int(d.n_draws), revision: Int(d.revision),
                         viewBox: [d.view_box.0, d.view_box.1, d.view_box.2, d.view_box.3],
                         overflow: [d.overflow.0, d.overflow.1, d.overflow.2, d.overflow.3])
    }
    /// Counts up whenever the ornament layer is rebuilt. 0 when undressed.
    public func dressRevision() -> Int { dressInfo()?.revision ?? 0 }
    /// The ornament display list, in page units, in drawing order. Built once per revision.
    public func dressGeometry() -> [OrnamentDraw] {
        let rev = dressRevision()
        if let c = dressDraws, c.revision == rev { return c.draws }
        var g = QvpDressGeometry()
        qvp_dress_geometry(p, &g)
        let n = Int(g.n_draws)
        guard n > 0, let ops = g.ops, let pts = g.pts, let table = g.table else { dressDraws = (rev, []); return [] }
        // the engine's buffers last until the next dress, undress or layout: read them now
        let t = UnsafeBufferPointer(start: table, count: n * 9)
        var out: [OrnamentDraw] = []; out.reserveCapacity(n)
        for i in 0..<n {
            let m = CGMutablePath()
            var o = Int(t[i * 9]); let oe = o + Int(t[i * 9 + 1]); var k = Int(t[i * 9 + 2])
            func pt(_ j: Int) -> CGPoint { CGPoint(x: CGFloat(pts[k + j]), y: CGFloat(pts[k + j + 1])) }
            while o < oe {
                switch ops[o] {
                case 0: m.move(to: pt(0)); k += 2
                case 1: m.addLine(to: pt(0)); k += 2
                case 2: m.addQuadCurve(to: pt(2), control: pt(0)); k += 4
                case 3: m.addCurve(to: pt(4), control1: pt(0), control2: pt(2)); k += 6
                case 4: m.closeSubpath()
                default: break
                }
                o += 1
            }
            let flags = t[i * 9 + 5], placement = t[i * 9 + 8]
            out.append(OrnamentDraw(path: m, color: t[i * 9 + 4], kind: OrnamentKind(rawValue: Int(flags & 0xff)) ?? .ayahMark,
                                    evenOdd: flags & UInt32(QVP_DRAW_EVENODD) != 0, stroke: flags & UInt32(QVP_DRAW_STROKE) != 0,
                                    strokeWidth: Float(bitPattern: t[i * 9 + 6]), part: Int(t[i * 9 + 7]),
                                    placement: placement == QVP_NONE ? nil : Int(placement)))
        }
        dressDraws = (rev, out)
        return out
    }
}

/// `QvpPage.dressView`: the view for a host that scrolls the page, and the dressed box's size
/// at that view's scale, in layout px.
public struct QvpDressView: Equatable {
    public let view: QvpView
    public let width: Float, height: Float
}

// ── what both renderers share: the raster and the overflow of a dressed page ──

/// How far a dress's border reaches past the laid-out content, in layout px.
struct DressOverflow: Equatable {
    let left: CGFloat, top: CGFloat, right: CGFloat, bottom: CGFloat
}

extension QvpPage {
    /// The dress overflow of the layout in hand, or nil when the page is not dressed.
    var dressOverflow: DressOverflow? {
        guard isOpen, let o = dressInfo()?.overflow, o.count == 4 else { return nil }
        return DressOverflow(left: CGFloat(o[0]), top: CGFloat(o[1]), right: CGFloat(o[2]), bottom: CGFloat(o[3]))
    }

    /// `viewClamp` over the content plus the dress overflow: a drag reaches the border, never
    /// past it. An undressed page gets the plain clamp. The engine answers, so a scroll never
    /// pushes the border's top or bottom off the screen.
    public func dressClamp(_ view: QvpView, viewportW: Float, viewportH: Float) -> QvpView {
        var v = view.c, out = QvpFFI.QvpView()
        qvp_dress_clamp(p, &v, viewportW, viewportH, &out)
        return QvpView(out)
    }

    /// For a host that scrolls the page itself: the view that shows the dressed box at `scale`,
    /// centred across `viewportW` (at x = 0 when wider) with its top at 0, and the box's size at
    /// that scale, which is the size to give the canvas. An undressed page gets its content
    /// centred at the top and the content's size.
    public func dressView(_ scale: Float, viewportW: Float) -> QvpDressView {
        var out = QvpFFI.QvpView(), size: (Float, Float) = (0, 0)
        withUnsafeMutablePointer(to: &size) { s in
            s.withMemoryRebound(to: Float.self, capacity: 2) { qvp_dress_view(p, scale, viewportW, &out, $0) }
        }
        return QvpDressView(view: QvpView(out), width: size.0, height: size.1)
    }

    /// The paper behind the page: the content grown by the dress overflow, so the border sits
    /// on paper. `ox` and `oy` are where the content's origin stands on screen. Sizing only:
    /// the fit and the clamp are the engine's.
    func paperRect(_ l: QvpLayout, scale s: CGFloat, ox: CGFloat, oy: CGFloat) -> CGRect {
        let o = dressOverflow ?? DressOverflow(left: 0, top: 0, right: 0, bottom: 0)
        return CGRect(x: ox - o.left * s, y: oy - o.top * s,
                      width: (CGFloat(l.contentW) + o.left + o.right) * s, height: (CGFloat(l.contentH) + o.top + o.bottom) * s)
    }

    /// The ornament layer as a bitmap of `width` × `height` pixels, y-down like the ink bitmap.
    /// `transform` is the renderer's own page-to-band transform for one placement: each draw
    /// goes under the placement its page ink goes under, and the border goes under the
    /// identity placement. Strokes take their width with round caps and joins. Fills take the
    /// even-odd rule when flagged.
    func rasterizeOrnaments(width W: Int, height H: Int, scale: CGFloat,
                            transform: (QvpPlacement) -> CGAffineTransform) -> CGImage? {
        let draws = dressGeometry()
        guard W > 0, H > 0, !draws.isEmpty,
              let oc = CGContext(data: nil, width: W, height: H, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(),
                                 bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue)
        else { return nil }
        // flip to y-down page space so `makeImage()` comes out upright
        oc.translateBy(x: 0, y: CGFloat(H)); oc.scaleBy(x: scale, y: -scale)
        oc.setAllowsAntialiasing(true); oc.setShouldAntialias(true)
        oc.setLineCap(.round); oc.setLineJoin(.round)
        let places = layoutPlacements()
        var cur: Int?? = .none
        for d in draws {
            if cur != .some(d.placement) {
                if cur != nil { oc.restoreGState() }
                oc.saveGState()
                oc.concatenate(transform(d.placement.map { $0 < places.count ? places[$0] : .identity } ?? .identity))
                cur = .some(d.placement)
            }
            oc.addPath(d.path)
            if d.stroke {
                oc.setStrokeColor(QvpColor.cgColor(d.color)); oc.setLineWidth(CGFloat(d.strokeWidth)); oc.strokePath()
            } else {
                oc.setFillColor(QvpColor.cgColor(d.color)); oc.fillPath(using: d.evenOdd ? .evenOdd : .winding)
            }
        }
        if cur != nil { oc.restoreGState() }
        return oc.makeImage()
    }
}

/// The ornament layer of one page, rasterised once per dress revision, view transform and
/// band. A tiled border is hundreds of outlines and nothing in it moves while a highlight
/// animates, so a frame that draws the ink again takes this image as it is. One image per
/// slot: a page drawn as a stack of canvases keeps one per canvas.
final class QvpOrnamentRaster {
    private var slots: [Int: (key: String, image: CGImage)] = [:]
    /// The last image this cache handed out, for tests and a HUD.
    private(set) var last: CGImage?
    func clear() { slots.removeAll(); last = nil }
    /// The layer of `p` for one band, or nil when the page is not dressed. `key` says where the
    /// band is and at what view transform. The page, its dress revision and the pixel size
    /// are added here.
    func image(_ p: QvpPage, slot: Int = 0, key: String, width W: Int, height H: Int, scale: CGFloat,
               transform: (QvpPlacement) -> CGAffineTransform) -> CGImage? {
        guard p.isOpen, let dress = p.dressInfo() else { slots[slot] = nil; last = nil; return nil }
        let k = "\(ObjectIdentifier(p))|\(dress.revision)|\(key)|\(W)x\(H)|\(scale)"
        if let e = slots[slot], e.key == k { last = e.image; return e.image }
        guard let image = p.rasterizeOrnaments(width: W, height: H, scale: scale, transform: transform) else { slots[slot] = nil; return nil }
        slots[slot] = (k, image); last = image
        return image
    }
}
