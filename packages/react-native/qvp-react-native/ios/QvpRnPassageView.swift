// @quran.ws/qvp-react-native, iOS. The React Native host of a passage.
import UIKit
import React

/// The React Native host of a passage: a range of complete ayahs, or a surah's basmalah, laid out
/// on rows of this view's width and drawn here. The engine loads, measures and places every word.
/// This class reads the pages the props name, hands the engine a spec in points, draws the layout
/// and reports it to JS. Nothing is decided here, as in android/.../QvpRnPassageView.kt.
@objc(QvpRnPassageView)
final class QvpRnPassageView: UIView {
    // ── props (set by the manager) ──
    @objc var pageUris: [String] = [] { didSet { changed() } }
    @objc var surah = 0 { didSet { changed() } }
    @objc var from = 0 { didSet { changed() } }
    @objc var to = 0 { didSet { changed() } }
    @objc var basmalah = false { didSet { changed() } }
    /// Points per page unit; `lineHeight` wins when it is set.
    @objc var scale: CGFloat = 1 { didSet { changed() } }
    /// Points per printed line: the scale is this over the passage's printed line spacing.
    @objc var lineHeight: CGFloat = 0 { didSet { changed() } }
    @objc var lineSpacing: CGFloat = 1 { didSet { changed() } }
    @objc var padding: CGFloat = 2 { didSet { changed() } }
    @objc var align: String? { didSet { changed() } }
    @objc var maxRows = 0 { didSet { changed() } }
    @objc var keepAyahMark = false { didSet { changed() } }
    @objc var ellipsisWidth: CGFloat = 0 { didSet { changed() } }
    @objc var maxStretch: CGFloat = 0 { didSet { changed() } }
    @objc var ink: String? { didSet { setNeedsDisplay() } }
    @objc var paperColor: String? { didSet { setNeedsDisplay() } }
    @objc var onPassageLayout: RCTDirectEventBlock?
    @objc var onError: RCTDirectEventBlock?

    // ── applied state ──
    private var loadedKey: String?
    private var pages: [QvpPage] = []
    private var pagesByNumber: [Int: QvpPage] = [:]
    private var passage: QvpPassage?
    private var layoutKey: String?
    private var passageLayout: QvpPassageLayout?

    override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        backgroundColor = .clear
        contentMode = .redraw
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }
    deinit { destroy() }

    /// React Native dropped the view: it removes the view from its parent when it unmounts it,
    /// and never moves it to another. Close the pages now, as Android's onDropViewInstance does.
    private var mounted = false
    override func didMoveToSuperview() {
        super.didMoveToSuperview()
        if superview != nil { mounted = true } else if mounted { destroy() }
    }

    private func destroy() {
        passage?.close(); passage = nil
        pages.forEach { $0.close() }; pages = []; pagesByNumber = [:]
        loadedKey = nil; layoutKey = nil; passageLayout = nil
    }

    private var commitQueued = false
    private func changed() {
        if commitQueued { return }
        commitQueued = true
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.commitQueued = false
            self.commit()
        }
    }

    /// Apply the props set since the last commit.
    func commit() {
        do {
            try ensurePassage()
            relayout()
            setNeedsDisplay()
        } catch {
            NSLog("QvpRn: passage commit failed: %@", "\(error)")
            emit(onError, ["message": error.localizedDescription])
        }
    }

    private var lastWidth: CGFloat = 0
    override func layoutSubviews() {
        super.layoutSubviews()
        if bounds.width != lastWidth { lastWidth = bounds.width; commit() }
    }

    private func ensurePassage() throws {
        let key = "\(pageUris.joined(separator: "|"))#\(surah):\(from)-\(to):\(basmalah)"
        if key == loadedKey { return }
        destroy()
        loadedKey = key
        guard !pageUris.isEmpty, surah > 0 else { return }
        // The passage copies what it needs, but a draw names a path by its page, so the pages
        // stay open for as long as the passage is drawn. Each page goes into `pages` as it opens,
        // so a URI that fails after it leaves nothing for `destroy` to miss.
        for uri in pageUris {
            let bytes = try QvpRnBytes.read(uri)
            let p: QvpPage
            do { p = try QvpPage(bytes: bytes) } catch { throw QvpRnError("not a QVP page: \(uri) (QvpError.\(error))") }
            pages.append(p)
        }
        pagesByNumber = Dictionary(pages.map { ($0.pageNo, $0) }) { a, _ in a }
        do {
            passage = basmalah ? try QvpPassage(basmalahOf: surah, pages: pages)
                               : try QvpPassage(pages: pages, surah: surah, from: from, to: to > 0 ? to : from)
        } catch {
            throw QvpRnError((basmalah ? "no basmalah of surah \(surah) on the pages given" : "the pages given do not hold \(surah):\(from)-\(to > 0 ? to : from)") + " (QvpError.\(error))")
        }
    }

    private func relayout() {
        guard let p = passage, bounds.width > 0 else { return }
        // points per page unit: the host's scale, or the points it wants one printed line to take
        let s = lineHeight > 0 && p.lineSpacing > 0 ? Float(lineHeight) / p.lineSpacing : Float(scale)
        let a: QvpPassageAlign = align == "center" || align == "centre" ? .center : align == "justified" ? .justified : .right
        let rows = max(maxRows, 0)
        // The view draws the ellipsis after a cut, so it knows how wide one is: three dots as
        // tall as a row, unless the host reserved a width of its own.
        let ew: Float = ellipsisWidth > 0 || rows == 0 ? Float(ellipsisWidth)
            : Float((Self.ellipsis as NSString).size(withAttributes: [.font: UIFont.systemFont(ofSize: CGFloat(s * p.lineSpacing))]).width)
        let spec = QvpPassageSpec(width: Float(bounds.width), scale: s, lineSpacing: Float(lineSpacing), padding: Float(padding),
                                  align: a, maxRows: rows, keepAyahMark: keepAyahMark, ellipsisWidth: ew, maxStretch: Float(maxStretch))
        let key = "\(spec)"
        if key == layoutKey { return }
        let l = p.layout(spec)
        layoutKey = key
        passageLayout = l
        guard let l else { emit(onError, ["message": "the passage spec is out of range: \(spec)"]); return }
        emit(onPassageLayout, QvpRnMarshal.passageLayout(l, p))
    }

    override func draw(_ rect: CGRect) {
        guard let l = passageLayout, let ctx = UIGraphicsGetCurrentContext() else { return }
        let paper = QvpRnMarshal.color(paperColor, 0)
        if paper != 0 { ctx.setFillColor(QvpColor.cgColor(paper)); ctx.fill(bounds) }
        let inkColor = QvpColor.cgColor(QvpRnMarshal.color(ink, QvpDefaults.INK))
        ctx.setFillColor(inkColor)
        // the layout is in points, and so is the context
        l.draw(in: ctx, pages: pagesByNumber)
        // The engine leaves the ellipsis to the host: three dots in the ink, as tall as the row
        // the cut left room on.
        if let e = l.ellipsis {
            let font = UIFont.systemFont(ofSize: e.height)
            let text = Self.ellipsis as NSString
            let size = text.size(withAttributes: [.font: font])
            text.draw(at: CGPoint(x: e.midX - size.width / 2, y: e.maxY - font.ascender),
                      withAttributes: [.font: font, .foregroundColor: UIColor(cgColor: inkColor)])
        }
    }

    private func emit(_ block: RCTDirectEventBlock?, _ payload: [String: Any?]) { block?(QvpRnMarshal.toJsMap(payload)) }

    private static let ellipsis = "\u{2026}"
}
