// The reader's zoom control: what a pinch does to the page.
//
// A pinch can scale the printed page, the way it scales a photograph, or ask for bigger ink at
// the same page width — which is reflow: fewer words to a row, the rest moved down. Which of
// those a pinch means, where it lands, when it commits and what holds the reader's place across
// the relayout is the engine's, so every platform pinches alike.
import CoreGraphics
import QvpFFI

/// What a pinch does to the page.
public enum QvpZoomMode: UInt32 {
    /// The pinch lands on one of the page's own zoom steps and the page reflows onto it. What a
    /// reader gets unless a host asks otherwise.
    case stepped = 0
    /// The pinch drives the reflow zoom itself, up to the page's own limit.
    case continuous = 1
    /// The pinch scales the laid-out page as it stands. The rows never change.
    case magnify = 2
}

/// Where the reader's zoom control stands. A host keeps this beside its view transform, and the
/// default is the control a page opens on: stepped, on the printed page.
public struct QvpZoom: Equatable {
    /// True once the reader has zoomed in, by either road: the view magnified past the size the
    /// page is fitted at, or the page reflowed above the printed size.
    public func isZoomed(_ view: QvpView, fitScale: Float) -> Bool {
        var z = c, v = view.c
        return qvp_zoom_is_zoomed(&z, &v, fitScale) != 0
    }

    public var mode: QvpZoomMode
    /// 0 is the printed page, 1 upwards the page's own steps.
    public var step: Int
    /// The size in force, 1 at the printed page, whichever field the mode steers by.
    public var zoom: Float
    public init(mode: QvpZoomMode = .stepped, step: Int = 0, zoom: Float = 1) { self.mode = mode; self.step = step; self.zoom = zoom }
    var c: QvpFFI.QvpZoom { QvpFFI.QvpZoom(mode: mode.rawValue, step: UInt32(step), zoom: zoom) }
    init(_ z: QvpFFI.QvpZoom) { self.mode = QvpZoomMode(rawValue: z.mode) ?? .stepped; self.step = Int(z.step); self.zoom = z.zoom }
}

/// How many pages a released drag turns, in reading order: +1 the page after this one, -1 the
/// page before it, nil when the drag is not a swipe. A muṣḥaf is read right to left, so a flick
/// to the right turns to the next page — the engine says so, rather than each platform deciding.
public func qvpSwipePages(translation t: CGSize, velocity v: CGSize) -> Int? {
    let pages = qvp_swipe_pages(Float(t.width), Float(t.height), Float(v.width), Float(v.height))
    return pages == 0 ? nil : Int(pages)
}

/// What a sideways drag on a page means.
public enum QvpSideways { case pan, turnPage }

/// What a gesture produced: the control to keep, the view to draw with, and whether the page was
/// laid out again under it. A host redraws its overlays when `relaid` is true.
public struct QvpZoomChange {
    public let zoom: QvpZoom, view: QvpView, relaid: Bool
}

/// Pan and zoom over a laid-out page: a point `p` in layout points draws at `offset + scale·p`.
public struct QvpView: Equatable {
    public var scale: Float, offsetX: Float, offsetY: Float
    public init(scale: Float = 1, offsetX: Float = 0, offsetY: Float = 0) { self.scale = scale; self.offsetX = offsetX; self.offsetY = offsetY }
    var c: QvpFFI.QvpView { QvpFFI.QvpView(scale: scale, offset_x: offsetX, offset_y: offsetY) }
    init(_ v: QvpFFI.QvpView) { self.scale = v.scale; self.offsetX = v.offset_x; self.offsetY = v.offset_y }
}

extension QvpPage {
    /// The same control under another policy, keeping the size the reader is already at.
    public func zoomMode(_ spec: QvpLayoutSpec, _ zoom: QvpZoom, _ mode: QvpZoomMode) -> QvpZoom {
        var s = spec.c, z = zoom.c, out = QvpFFI.QvpZoom()
        qvp_zoom_mode(p, &s, &z, mode.rawValue, &out)
        return QvpZoom(out)
    }
    /// One frame of a pinch. `factor` is the distance between the fingers against their distance
    /// when they went down — the whole gesture every time, not the change since the last frame —
    /// and `focal` the point between them, in view points.
    public func zoomPinch(_ spec: QvpLayoutSpec, _ zoom: QvpZoom, _ view: QvpView, factor: Float, focalX: Float, focalY: Float) -> QvpZoomChange {
        var s = spec.c, z = zoom.c, v = view.c, out = QvpFFI.QvpZoomChange()
        qvp_zoom_pinch(p, &s, &z, &v, factor, focalX, focalY, &out)
        // the engine laid the page out inside the call: keep the cached layout with it
        if out.relaid != 0 { readLayout() }
        return QvpZoomChange(zoom: QvpZoom(out.zoom), view: QvpView(out.view), relaid: out.relaid != 0)
    }
    /// The control moved straight to a step: a size button, a double tap, a reset. Step 0 is the
    /// printed page.
    public func zoomToStep(_ spec: QvpLayoutSpec, _ zoom: QvpZoom, _ step: Int, _ view: QvpView) -> QvpZoomChange {
        var s = spec.c, z = zoom.c, v = view.c, out = QvpFFI.QvpZoomChange()
        qvp_zoom_to_step(p, &s, &z, UInt32(step), &v, &out)
        if out.relaid != 0 { readLayout() }
        return QvpZoomChange(zoom: QvpZoom(out.zoom), view: QvpView(out.view), relaid: out.relaid != 0)
    }
    /// The same control on this page: what the reader was reading at, carried onto the page they
    /// turned to. A step carries as a step, because every page's steps are its own; a free zoom
    /// carries as a size, held inside what this page can reach.
    public func zoomCarried(_ spec: QvpLayoutSpec, _ zoom: QvpZoom) -> QvpZoom {
        var s = spec.c, z = zoom.c, out = QvpFFI.QvpZoom()
        qvp_zoom_carried(p, &s, &z, &out)
        return QvpZoom(out)
    }
    /// The reflow zoom one step of this page's control means; step 0 is the printed page.
    public func zoomAtStep(_ spec: QvpLayoutSpec, _ step: Int) -> Float {
        var s = spec.c
        return qvp_zoom_at_step(p, &s, UInt32(step))
    }
    /// What a sideways drag on this page means: pan it, or turn the page.
    public func sidewaysDrag(_ zoom: QvpZoom, _ view: QvpView, fitScale: Float) -> QvpSideways {
        var z = zoom.c, v = view.c
        return qvp_sideways_drag(p, &z, &v, fitScale) == 1 ? .turnPage : .pan
    }
    /// `spec` with this control's zoom in it: what the host lays out, draws and hit-tests with.
    public func zoomSpec(_ spec: QvpLayoutSpec, _ zoom: QvpZoom) -> QvpLayoutSpec {
        var s = spec.c, z = zoom.c, out = QvpFFI.QvpLayoutSpec()
        qvp_zoom_spec(p, &s, &z, &out)
        var spec = spec
        spec.reflow = out.reflow_zoom > 0 ? QvpReflowSpec(zoom: out.reflow_zoom, wordGap: spec.reflow?.wordGap ?? 1,
                                                          relax: spec.reflow?.relax ?? QvpDefaults.REFLOW_RELAX,
                                                          maxStretch: spec.reflow?.maxStretch ?? QvpDefaults.REFLOW_MAX_STRETCH) : nil
        return spec
    }
    /// Hold the content against the viewport: centre an axis it does not fill, cover the
    /// viewport on an axis it overflows, so no drag opens a blank strip beside the page.
    public func viewClamp(_ view: QvpView, contentW: Float, contentH: Float, viewportW: Float, viewportH: Float) -> QvpView {
        var v = view.c, out = QvpFFI.QvpView()
        qvp_view_clamp(&v, contentW, contentH, viewportW, viewportH, &out)
        return QvpView(out)
    }
    /// The view that holds a point inside a word at a place on the screen, then clamps: how a
    /// pinch keeps its place when the layout reflows under it.
    public func viewAnchor(_ view: QvpView, word: Int, nx: Float, ny: Float, toX: Float, toY: Float, viewportW: Float, viewportH: Float) -> QvpView {
        var v = view.c, out = QvpFFI.QvpView()
        qvp_view_anchor(p, &v, UInt32(word), nx, ny, toX, toY, viewportW, viewportH, &out)
        return QvpView(out)
    }
}
