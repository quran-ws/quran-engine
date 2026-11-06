// @quran.ws/qvp-react-native, iOS. The native module over QvpKit.
import Foundation
import UIKit
import React

/// Promise-based queries on the page behind a `<QvpPageView />` (looked up by react tag), plus
/// engine-wide helpers, the atlas and the ornament sets. The method names, arguments and result
/// shapes are those of android/.../QvpModule.kt, so src/index.tsx has no platform branch. Names
/// follow docs/API.md. QvpModule.m declares the methods to React Native.
///
/// Two queues run the calls. The query queue, which this module owns, runs every call that reads
/// only what a page fixed when it loaded: its words, ayahs, lines, text, search, metadata and crop
/// box. It also runs the engine-wide helpers and owns the atlases. A long query on it does not
/// hold up a frame. The view keeps the page open while a query reads it (`QvpRnPageView.read`).
/// Every call that reads or changes the layout, the view, styles, colours, the selection, the
/// mask, the reveal or the dress runs on the main thread, where the view draws.
///
/// The view methods take and return points, the iOS form of the dp the Android module uses.
@objc(QvpModule)
final class QvpModule: NSObject {
    /// The query queue. The atlases below are touched only on it.
    private let queries = DispatchQueue(label: "ws.quran.qvp.rn.query", qos: .userInitiated)
    private var atlases: [Int: QvpAtlas] = [:]
    private var atlasByUri: [String: Int] = [:]
    private var nextAtlas = 1

    @objc static func requiresMainQueueSetup() -> Bool { false }

    // ── plumbing ──
    private func settle(_ resolve: RCTPromiseResolveBlock, _ reject: RCTPromiseRejectBlock, _ block: () throws -> Any?) {
        do { resolve(QvpRnMarshal.toJs(try block())) } catch { reject("qvp", error.localizedDescription, error) }
    }
    private func ui(_ resolve: @escaping RCTPromiseResolveBlock, _ reject: @escaping RCTPromiseRejectBlock, _ block: @escaping () throws -> Any?) {
        DispatchQueue.main.async { self.settle(resolve, reject, block) }
    }
    /// An engine-wide call, or an atlas call, on the query queue.
    private func off(_ resolve: @escaping RCTPromiseResolveBlock, _ reject: @escaping RCTPromiseRejectBlock, _ block: @escaping () throws -> Any?) {
        queries.async { self.settle(resolve, reject, block) }
    }
    private func view(_ tag: NSNumber) throws -> QvpRnPageView {
        guard let v = QvpRnRegistry.get(tag.intValue) else { throw QvpRnError("no QvpPageView with tag \(tag)") }
        return v
    }
    private func loaded(_ v: QvpRnPageView, _ tag: NSNumber) throws -> QvpPage {
        guard let p = v.page, p.isOpen else { throw QvpRnError("QvpPageView \(tag) has no page loaded") }
        return p
    }
    /// A read-only page query on the query queue, with the page held open until it returns. The
    /// queue sees the view's state alone, never the view. A view the registry does not hold
    /// yet is looked up again on the main thread, after the prop batches queued there, as
    /// Android looks it up through the UIManager, and the query runs there.
    private func readPage(_ tag: NSNumber, _ resolve: @escaping RCTPromiseResolveBlock, _ reject: @escaping RCTPromiseRejectBlock,
                          _ block: @escaping (QvpRnPageState, QvpPage, _ loadMs: Double, _ bytes: Int) throws -> Any?) {
        let run = { (s: QvpRnPageState) in
            self.settle(resolve, reject) {
                try s.read { p, ms, bytes in
                    guard let p else { throw QvpRnError("QvpPageView \(tag) has no page loaded") }
                    return try block(s, p, ms, bytes)
                }
            }
        }
        queries.async {
            if let s = QvpRnRegistry.state(tag.intValue) { run(s); return }
            DispatchQueue.main.async {
                guard let s = QvpRnRegistry.state(tag.intValue) else { reject("qvp", "no QvpPageView with tag \(tag)", nil); return }
                run(s)
            }
        }
    }
    /// A page call on the main thread that the queries queued before it see done, and that is
    /// done before the queries queued after it start: the query queue waits for it. It holds no
    /// page open while it waits, so the main thread never waits for it in turn.
    private func inOrder(_ tag: NSNumber, _ resolve: @escaping RCTPromiseResolveBlock, _ reject: @escaping RCTPromiseRejectBlock,
                         _ block: @escaping (QvpRnPageView, QvpPage) throws -> Any?) {
        queries.async {
            DispatchQueue.main.sync {
                self.settle(resolve, reject) { let v = try self.view(tag); return try block(v, try self.loaded(v, tag)) }
            }
        }
    }
    /// A page call on the main thread.
    private func withPage(_ tag: NSNumber, _ resolve: @escaping RCTPromiseResolveBlock, _ reject: @escaping RCTPromiseRejectBlock,
                          _ block: @escaping (QvpRnPageView, QvpPage) throws -> Any?) {
        ui(resolve, reject) { let v = try self.view(tag); return try block(v, try self.loaded(v, tag)) }
    }
    private func withAtlas(_ id: NSNumber, _ resolve: @escaping RCTPromiseResolveBlock, _ reject: @escaping RCTPromiseRejectBlock, _ block: @escaping (QvpAtlas) throws -> Any?) {
        off(resolve, reject) {
            guard let a = self.atlases[id.intValue] else { throw QvpRnError("no atlas \(id)") }
            return try block(a)
        }
    }
    private func withOrnaments(_ id: NSNumber, _ resolve: @escaping RCTPromiseResolveBlock, _ reject: @escaping RCTPromiseRejectBlock, _ block: @escaping (QvpOrnaments) throws -> Any?) {
        ui(resolve, reject) {
            guard let s = QvpOrnamentSets.get(id.intValue) else { throw QvpRnError("no ornament set \(id)") }
            return try block(s)
        }
    }
    private func opt(_ m: NSDictionary?) -> [String: Any] { (m as? [String: Any]) ?? [:] }
    private func ints(_ a: NSArray?) -> [Int] { QvpRnMarshal.ints(a as? [Any]) }
    private func hitOptions(_ o: [String: Any]) -> QvpHitOptions {
        QvpHitOptions(maxDistance: QvpRnMarshal.flt(o["maxDistance"], QvpDefaults.TAP_DISTANCE), gapBias: QvpRnMarshal.flt(o["gapBias"], QvpDefaults.GAP_BIAS),
                      preferExact: !QvpRnMarshal.isFalse(o["preferExact"]))
    }
    private func viewOf(_ v: QvpRnPageView) -> QvpView {
        QvpView(scale: Float(v.inner.viewScale), offsetX: Float(v.inner.viewOx), offsetY: Float(v.inner.viewOy))
    }

    // ── page facts ──
    @objc func info(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { s, p, ms, bytes in QvpRnMarshal.pageInfo(p, s.words(p).forms).merging(["loadMs": ms, "bytes": bytes]) { a, _ in a } }
    }
    @objc func words(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { s, p, _, _ in s.words(p).all() }
    }
    @objc func word(_ tag: NSNumber, index: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { s, p, _, _ in s.words(p).word(index.intValue) }
    }
    @objc func ayahs(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.ayahs.map { QvpRnMarshal.ayah($0) } }
    }
    @objc func lines(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.lines.map { QvpRnMarshal.line($0) } }
    }
    @objc func decorations(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.decorations.map { QvpRnMarshal.decoration($0) } }
    }
    @objc func findWord(_ tag: NSNumber, s: NSNumber, a: NSNumber, w: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.findWord(s.intValue, a.intValue, w.intValue) }
    }
    @objc func targetWords(_ tag: NSNumber, target: Any?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.targetWords(QvpRnMarshal.target(target, p)) }
    }
    @objc func wordForm(_ tag: NSNumber, index: NSNumber, form: String?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.wordForm(index.intValue, QvpRnMarshal.form(form)) }
    }
    @objc func hasForm(_ tag: NSNumber, form: String?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.hasForm(QvpRnMarshal.form(form)) }
    }
    @objc func attachWords(_ tag: NSNumber, json: String, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        inOrder(tag, resolve, reject) { v, p in v.attachWords(p, json) }
    }

    // ── metadata ──
    @objc func surahs(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.surahs().map { QvpRnMarshal.surah($0) } }
    }
    @objc func divisions(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.divisions().map { QvpRnMarshal.division($0) } }
    }
    @objc func surahHeaders(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.surahHeaders().map { QvpRnMarshal.surahHeader($0) } }
    }
    @objc func surahHeadersView(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.surahHeadersView().map { QvpRnMarshal.surahHeader($0) } }
    }
    @objc func ayahMarks(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.ayahMarks().map { QvpRnMarshal.ayahMark($0) } }
    }
    @objc func ayahMarksView(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.ayahMarksView().map { QvpRnMarshal.ayahMark($0) } }
    }
    @objc func rosettes(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.rosettes().map { QvpRnMarshal.rosette($0) } }
    }
    @objc func sajdahs(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.sajdahs().map { QvpRnMarshal.sajdah($0) } }
    }
    @objc func ayahKeys(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.ayahKeys().map { ["surah": $0.0, "ayah": $0.1, "ayahKey": "\($0.0):\($0.1)"] as [String: Any?] } }
    }
    @objc func ayahWordCount(_ tag: NSNumber, s: NSNumber, a: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in let c = p.ayahWordCount(s.intValue, a.intValue); return ["count": c.count, "isComplete": c.isComplete] as [String: Any?] }
    }
    @objc func reciteMap(_ tag: NSNumber, s: NSNumber, a: NSNumber, n: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.reciteMap(s.intValue, a.intValue, nSegments: n.intValue) }
    }
    @objc func wordLabel(_ tag: NSNumber, i: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.wordLabel(i.intValue) }
    }
    @objc func ayahLabel(_ tag: NSNumber, i: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in p.ayahLabel(i.intValue) }
    }

    // ── text and search ──
    @objc func text(_ tag: NSNumber, target: Any?, opts: NSDictionary?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let o = opt(opts)
        readPage(tag, resolve, reject) { _, p, _, _ in
            p.text(QvpRnMarshal.target(target, p), form: QvpRnMarshal.form(o["form"]), wordSep: QvpRnMarshal.str(o["wordSep"]) ?? " ", lineSep: QvpRnMarshal.str(o["lineSep"]) ?? "\n")
        }
    }
    @objc func search(_ tag: NSNumber, query: String, opts: NSDictionary?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let o = opt(opts)
        readPage(tag, resolve, reject) { _, p, _, _ in
            p.search(query, form: QvpRnMarshal.form(o["form"], .search), mode: QvpRnMarshal.searchMode(o["mode"]), normalize: !QvpRnMarshal.isFalse(o["normalize"]),
                     looseMatch: !QvpRnMarshal.isFalse(o["looseMatch"]), limit: QvpRnMarshal.num(o["limit"]) ?? 0).map { QvpRnMarshal.match($0) }
        }
    }
    @objc func citation(_ tag: NSNumber, words: NSArray, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let ws = ints(words)
        readPage(tag, resolve, reject) { _, p, _, _ in p.citation(ws) }
    }
    @objc func arabic(_ op: String, s: String, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        off(resolve, reject) {
            switch op {
            case "strip": return QvpEngine.strip(s)
            case "fold": return QvpEngine.fold(s)
            case "normalize": return QvpEngine.normalize(s)
            case "loose", "looseKey": return QvpEngine.looseKey(s)
            default: return s
            }
        }
    }

    // ── hit testing and layout (the view does the gestures; these are for scroll-into-view and the like) ──
    /// Exact hit at a point in page units: the outline under the point, else the box of a word or a decoration.
    @objc func hitTestExact(_ tag: NSNumber, x: NSNumber, y: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.hitTestExact(x.floatValue, y.floatValue).map { QvpRnMarshal.hit(p, $0) } }
    }
    /// Gap-aware hit at a point in page units.
    @objc func hitTest(_ tag: NSNumber, x: NSNumber, y: NSNumber, opts: NSDictionary?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let o = opt(opts)
        withPage(tag, resolve, reject) { _, p in p.hitTest(x.floatValue, y.floatValue, self.hitOptions(o)).map { QvpRnMarshal.hit(p, $0) } }
    }
    /// Exact hit at a point in view points (the pan and zoom removed here, the layout in the engine).
    @objc func hitTestExactView(_ tag: NSNumber, x: NSNumber, y: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in
            let l = p.viewToLayout(self.viewOf(v), x.floatValue, y.floatValue)
            return p.hitTestExactView(l.x, l.y).map { QvpRnMarshal.hit(p, $0) }
        }
    }
    @objc func hitTestView(_ tag: NSNumber, x: NSNumber, y: NSNumber, opts: NSDictionary?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let o = opt(opts)
        withPage(tag, resolve, reject) { v, p in
            let l = p.viewToLayout(self.viewOf(v), x.floatValue, y.floatValue)
            return p.hitTestView(l.x, l.y, self.hitOptions(o)).map { QvpRnMarshal.hit(p, $0) }
        }
    }
    @objc func wordBoundsView(_ tag: NSNumber, i: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in
            let s = Float(v.inner.viewScale), ox = Float(v.inner.viewOx), oy = Float(v.inner.viewOy)
            return p.wordBoundsView(i.intValue).map { b in ["x0": ox + b.x0 * s, "y0": oy + b.y0 * s, "x1": ox + b.x1 * s, "y1": oy + b.y1 * s] as [String: Any?] }
        }
    }
    @objc func currentLayout(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.currentLayout.map { QvpRnMarshal.layout($0) } }
    }
    @objc func relayout(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, _ in v.inner.relayout(); v.resetView(); return nil }
    }
    @objc func resetView(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, _ in v.resetView(); return nil }
    }
    /// The leading that fills this view's viewport. This call and the two after it measure the
    /// printed page, so they take the view's spec before the reader's zoom, as Android does.
    @objc func layoutLineSpacingToFill(_ tag: NSNumber, max: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in p.layoutLineSpacingToFill(v.inner.baseSpec, max: max.floatValue) }
    }
    @objc func layoutWastedFraction(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in p.layoutWastedFraction(v.inner.baseSpec) }
    }
    /// The page's height at the printed pitch for the view's spec before the reader's zoom.
    @objc func layoutPrintedHeight(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in p.layoutPrintedHeight(v.inner.baseSpec) }
    }
    @objc func grid(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        readPage(tag, resolve, reject) { _, p, _, _ in let g = p.grid; return ["lines": g.lines, "lineSpacing": g.lineSpacing] as [String: Any?] }
    }
    @objc func stats(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        ui(resolve, reject) { QvpRnRegistry.get(tag.intValue)?.stats() }
    }
    @objc func invalidate(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        ui(resolve, reject) { QvpRnRegistry.get(tag.intValue)?.inner.setNeedsDisplay(); return nil }
    }

    // ── selection ──
    @objc func select(_ tag: NSNumber, anchor: NSNumber, focus: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in p.select(anchor.intValue, focus.intValue); v.inner.setNeedsDisplay(); return QvpRnMarshal.selection(p) }
    }
    @objc func clearSelection(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, _ in v.inner.clearSelection(); return nil }
    }
    @objc func selection(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in QvpRnMarshal.selection(p) }
    }
    @objc func selectionText(_ tag: NSNumber, form: String?, includeCitation: Bool, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.selectionText(QvpRnMarshal.form(form), includeCitation: includeCitation) }
    }

    // ── memorisation (stepwise calls; the `mask` and `reveal` props hold the declarative part) ──
    @objc func unmaskNext(_ tag: NSNumber, n: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in let r = p.unmaskNext(n.intValue); v.inner.setNeedsDisplay(); return r }
    }
    @objc func maskBack(_ tag: NSNumber, n: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in let r = p.maskBack(n.intValue); v.inner.setNeedsDisplay(); return r }
    }
    @objc func unmaskWord(_ tag: NSNumber, i: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in let r = p.unmaskWord(i.intValue); v.inner.setNeedsDisplay(); return r }
    }
    @objc func maskWord(_ tag: NSNumber, i: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in let r = p.maskWord(i.intValue); v.inner.setNeedsDisplay(); return r }
    }
    @objc func unmaskAll(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in p.unmaskAll(); v.inner.setNeedsDisplay(); return nil }
    }
    @objc func maskAll(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in p.maskAll(); v.inner.setNeedsDisplay(); return nil }
    }
    @objc func maskHidden(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.maskHidden() }
    }
    @objc func maskWords(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.maskWords() }
    }
    @objc func revealStepCount(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.revealStepCount() }
    }
    @objc func revealPosition(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.revealPosition().map(Double.init) }
    }
    @objc func revealStepOf(_ tag: NSNumber, i: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in Double(p.revealStepOf(i.intValue)) }
    }

    // ── crop ──
    @objc func cropBounds(_ tag: NSNumber, target: Any?, opts: NSDictionary?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let o = opt(opts)
        readPage(tag, resolve, reject) { _, p, _, _ in
            p.cropBounds(QvpRnMarshal.target(target, p), pad: QvpRnMarshal.flt(o["pad"], QvpDefaults.CROP_PAD), keepAyahMarks: !QvpRnMarshal.isFalse(o["keepAyahMarks"]))
                .map { QvpRnMarshal.cropBounds($0) }
        }
    }
    @objc func cropSvg(_ tag: NSNumber, target: Any?, opts: NSDictionary?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let o = opt(opts)
        withPage(tag, resolve, reject) { _, p in
            p.cropSvg(QvpRnMarshal.target(target, p), pad: QvpRnMarshal.flt(o["pad"], QvpDefaults.CROP_PAD), keepAyahMarks: !QvpRnMarshal.isFalse(o["keepAyahMarks"]),
                      background: QvpRnMarshal.color(o["background"], 0))
        }
    }

    // ── atlas ──
    @objc func loadAtlas(_ uri: String, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        off(resolve, reject) {
            if let id = self.atlasByUri[uri] { return id }
            let bytes = try QvpRnBytes.read(uri)
            let a: QvpAtlas
            do { a = try QvpAtlas(bytes: bytes) } catch { throw QvpRnError("not a QVP atlas: \(uri) (QvpError.\(error))") }
            let id = self.nextAtlas; self.nextAtlas += 1
            self.atlases[id] = a; self.atlasByUri[uri] = id
            return id
        }
    }
    @objc func freeAtlas(_ id: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        off(resolve, reject) {
            self.atlases.removeValue(forKey: id.intValue)?.close()
            self.atlasByUri = self.atlasByUri.filter { $0.value != id.intValue }
            return nil
        }
    }
    @objc func atlasPageOf(_ id: NSNumber, s: NSNumber, a: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.pageOf(s.intValue, a.intValue) }
    }
    @objc func atlasPageRange(_ id: NSNumber, page: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { atlas in
            atlas.pageRange(page.intValue).map { r in
                ["first": ["surah": r.first.0, "ayah": r.first.1] as [String: Any?], "last": ["surah": r.last.0, "ayah": r.last.1] as [String: Any?]] as [String: Any?]
            }
        }
    }
    @objc func atlasPageCount(_ id: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.pageCount() }
    }
    @objc func atlasSurah(_ id: NSNumber, n: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.surah(n.intValue).map { QvpRnMarshal.atlasSurah($0) } }
    }
    @objc func atlasSurahs(_ id: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.surahs().map { QvpRnMarshal.atlasSurah($0) } }
    }
    @objc func atlasPageOfSurah(_ id: NSNumber, n: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.pageOfSurah(n.intValue) }
    }
    @objc func atlasDivision(_ id: NSNumber, division: String, n: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.division(QvpRnMarshal.division(division), n.intValue).map { QvpRnMarshal.atlasRubuAlHizb($0) } }
    }
    @objc func atlasDivisionOf(_ id: NSNumber, division: String, s: NSNumber, a: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.divisionOf(QvpRnMarshal.division(division), s.intValue, a.intValue) }
    }
    @objc func atlasJuzOf(_ id: NSNumber, s: NSNumber, a: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.juzOf(s.intValue, a.intValue) }
    }
    @objc func atlasPagesOfJuz(_ id: NSNumber, n: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.pagesOfJuz(n.intValue).map { [$0.0, $0.1] } }
    }
    @objc func atlasSearchSurahs(_ id: NSNumber, text: String, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withAtlas(id, resolve, reject) { $0.searchSurahs(text).map { QvpRnMarshal.atlasSurah($0) } }
    }

    // ── dress ──
    // An ornament set is held here by id, like an atlas. The C calls behind each method:
    // qvp_ornaments_load, qvp_ornaments_free, qvp_ornament_count, qvp_ornament_info and
    // qvp_ornament_part (one ornament and its parts), qvp_ornament_find, then on the page
    // qvp_dress, qvp_undress and qvp_dress_info. qvp_dress_geometry is the view's: it paints the layer.
    @objc func loadOrnaments(_ uri: String, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        ui(resolve, reject) {
            let bytes = try QvpRnBytes.read(uri)
            let set: QvpOrnaments
            do { set = try QvpOrnaments(bytes: bytes) } catch { throw QvpRnError("not an ornament set: \(uri) (QvpError.\(error))") }
            return QvpOrnamentSets.add(set)
        }
    }
    @objc func freeOrnaments(_ id: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        ui(resolve, reject) { QvpOrnamentSets.free(id.intValue); return nil }
    }
    @objc func ornamentCount(_ id: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withOrnaments(id, resolve, reject) { $0.count }
    }
    @objc func ornamentInfo(_ id: NSNumber, index: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withOrnaments(id, resolve, reject) { set in set.list.first { $0.index == index.intValue }.map { QvpRnMarshal.ornament($0) } }
    }
    @objc func ornamentPart(_ id: NSNumber, index: NSNumber, part: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withOrnaments(id, resolve, reject) { set in
            guard let o = set.list.first(where: { $0.index == index.intValue }), part.intValue >= 0, part.intValue < o.parts.count else { return nil }
            return QvpRnMarshal.ornamentPart(o.parts[part.intValue])
        }
    }
    @objc func ornamentList(_ id: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withOrnaments(id, resolve, reject) { $0.list.map { QvpRnMarshal.ornament($0) } }
    }
    @objc func ornamentFind(_ id: NSNumber, kind: String, style: String, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withOrnaments(id, resolve, reject) { set in QvpRnMarshal.ornamentKind(kind).flatMap { set.find($0, style)?.index } ?? -1 }
    }
    /// Dress the page behind a view. The `dress` prop does the same declaratively; this call lasts until the next page.
    @objc func dress(_ tag: NSNumber, id: NSNumber, spec: NSDictionary?, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        let s = QvpRnMarshal.dressSpec(opt(spec))
        withPage(tag, resolve, reject) { v, p in
            guard let set = QvpOrnamentSets.get(id.intValue) else { throw QvpRnError("no ornament set \(id)") }
            return v.dressNow(p, set, s).map { QvpRnMarshal.dressInfo($0) }
        }
    }
    @objc func undress(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { v, p in v.undressNow(p); return nil }
    }
    /// The engine's clamp of a view over the content plus the dress overflow. Offsets and viewport are in points, like the other view methods.
    @objc func dressClamp(_ tag: NSNumber, scale: NSNumber, offsetX: NSNumber, offsetY: NSNumber, viewportW: NSNumber, viewportH: NSNumber,
                          resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in
            let c = p.dressClamp(QvpView(scale: scale.floatValue, offsetX: offsetX.floatValue, offsetY: offsetY.floatValue), viewportW: viewportW.floatValue, viewportH: viewportH.floatValue)
            return ["scale": c.scale, "offsetX": c.offsetX, "offsetY": c.offsetY] as [String: Any?]
        }
    }
    /// The engine's view and dressed box size for a host that scrolls the page. The viewport, offsets and size are in points.
    @objc func dressView(_ tag: NSNumber, scale: NSNumber, viewportW: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in
            let r = p.dressView(scale.floatValue, viewportW: viewportW.floatValue)
            return ["view": ["scale": r.view.scale, "offsetX": r.view.offsetX, "offsetY": r.view.offsetY] as [String: Any?], "width": r.width, "height": r.height] as [String: Any?]
        }
    }
    @objc func dressInfo(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.dressInfo().map { QvpRnMarshal.dressInfo($0) } }
    }
    @objc func dressRevision(_ tag: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        withPage(tag, resolve, reject) { _, p in p.dressRevision() }
    }

    // ── names ──
    @objc func markName(_ m: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        off(resolve, reject) { QvpEngine.markName(m.intValue) }
    }
    @objc func markFromName(_ s: String, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        off(resolve, reject) { QvpEngine.markFromName(s) }
    }
    @objc func markCategory(_ m: NSNumber, resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        off(resolve, reject) { QvpEngine.markCategory(m.intValue) }
    }
    @objc func engineName(_ resolve: @escaping RCTPromiseResolveBlock, reject: @escaping RCTPromiseRejectBlock) {
        off(resolve, reject) { QvpEngine.engineName() }
    }

    /// Every name table, read from the engine; the JavaScript side resolves names with these.
    @objc func constantsToExport() -> [AnyHashable: Any]! {
        ["version": QvpEngine.version(), "formatVersion": QvpEngine.formatVersion(),
         "marks": QvpEngine.names(QvpEngine.Names.mark), "kinds": QvpEngine.names(QvpEngine.Names.kind),
         "families": QvpEngine.names(QvpEngine.Names.family), "categories": QvpEngine.names(QvpEngine.Names.category),
         "decorations": QvpEngine.names(QvpEngine.Names.decoration), "divisions": QvpEngine.names(QvpEngine.Names.division),
         "places": QvpEngine.names(QvpEngine.Names.place),
         // the whole defaults table by camelCase name, as the decimals the header states
         "defaults": QvpDefaults.decimal]
    }

    @objc func invalidate() {
        // The atlases belong to the query queue: free them there, after the queries already queued.
        queries.async { self.atlases.values.forEach { $0.close() }; self.atlases = [:]; self.atlasByUri = [:] }
        DispatchQueue.main.async { QvpOrnamentSets.clear() }
    }
}

/// Ornament sets by id, shared by the module and the views: the `dress` prop names its set by
/// this id. A dressed page keeps what it needs, so freeing a set never breaks a page that wears
/// it. Touched only on the main thread.
enum QvpOrnamentSets {
    private static var sets: [Int: QvpOrnaments] = [:]
    private static var next = 1
    static func add(_ set: QvpOrnaments) -> Int { let id = next; next += 1; sets[id] = set; return id }
    static func get(_ id: Int?) -> QvpOrnaments? { id.flatMap { sets[$0] } }
    static func free(_ id: Int) { sets.removeValue(forKey: id)?.close() }
    static func clear() { sets.values.forEach { $0.close() }; sets = [:] }
}

extension QvpModule: RCTInvalidating {}
