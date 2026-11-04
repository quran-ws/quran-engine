// The view-state helpers both iOS renderers share. They are platform conveniences
// (docs/API-PARITY.md) and decide nothing the engine could decide: every number comes from the
// engine's view and zoom calls.
import CoreGraphics
import QvpFFI

public enum QvpViewPolicy {
    /// The engine's own ceiling on the magnifying glass, `QVP_DEFAULT_MAX_ZOOM`: what
    /// `QvpPageView.maxZoom` and `QvpCanvasController.maxZoom` start at.
    public static let maxZoom: CGFloat = CGFloat(QvpDefaults.MAX_ZOOM)

    /// The scale a magnifying pinch lands on, from the engine (`qvp_view_zoom_about`): held
    /// between `fit`, the page's settled size, and `ceiling`, the host's own. A pinch magnifies
    /// the print and never shrinks the page inside the screen, so a ceiling under the floor is
    /// the floor. A ceiling of 0 or less is the engine's own.
    public static func clampZoom(_ scale: CGFloat, fit: CGFloat, ceiling: CGFloat) -> CGFloat {
        CGFloat(QvpView(scale: Float(scale)).zoomAbout(focalX: 0, focalY: 0, factor: 1, min: Float(fit), max: Float(ceiling)).scale)
    }
    /// How many pages a released drag turns, in the muṣḥaf's own order: +1 the page after this
    /// one, -1 the page before it, nil when it is not a swipe. The engine decides, so no platform
    /// can read the book backwards.
    public static func swipeDirection(translation t: CGSize, velocity v: CGSize) -> Int? {
        qvpSwipePages(translation: t, velocity: v)
    }
}
