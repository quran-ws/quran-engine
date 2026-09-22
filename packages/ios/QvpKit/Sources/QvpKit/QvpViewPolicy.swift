// The view-state constants both iOS renderers share: zoom limits, the zoomed threshold and
// the swipe classifier. They are platform conveniences (docs/API-PARITY.md) and decide nothing
// the engine could decide.
import CoreGraphics

public enum QvpViewPolicy {
    /// Pinch limits as multiples of the fitted scale; the same pair on every platform.
    public static let minZoom: CGFloat = 0.5
    public static let maxZoom: CGFloat = 12
    /// A page counts as zoomed once it is 2% above the fitted scale, so a settled pinch is not a zoom.
    public static let zoomedThreshold: CGFloat = 1.02
    /// A drag is a page swipe when it is mostly horizontal and either long or fast enough.
    public static let swipeAxisRatio: CGFloat = 1.5
    public static let swipeDistance: CGFloat = 40
    public static let swipeVelocity: CGFloat = 500

    /// The pinch clamp every renderer applies.
    public static func clampZoom(_ scale: CGFloat) -> CGFloat { min(max(scale, minZoom), maxZoom) }

    /// The same clamp with the page's settled size as the floor: a pinch magnifies the print and
    /// never shrinks the page inside the screen. `fit` is `QvpLayout.scale` for the page in hand.
    public static func clampZoom(_ scale: CGFloat, fit: CGFloat) -> CGFloat {
        min(max(scale, max(fit, minZoom)), maxZoom)
    }
    /// The same clamp under a ceiling of the host's own (`QvpCanvasController.maxZoom`): a
    /// host whose overlays scale with the glass says how far it may go. A ceiling under the
    /// floor is the floor — the settled page, which a pinch never shrinks inside the screen.
    /// A host that lowers the ceiling below the size its own page is fitted at is saying it
    /// wants no glass at all, and gets the fitted page rather than a page shrunk to a
    /// ceiling it can then neither leave nor pinch back out of.
    public static func clampZoom(_ scale: CGFloat, fit: CGFloat, ceiling: CGFloat) -> CGFloat {
        let floor = max(fit, minZoom)
        return min(max(scale, floor), max(ceiling, floor))
    }
    /// How many pages a released drag turns, in the muṣḥaf's own order: +1 the page after this
    /// one, -1 the page before it, nil when it is not a swipe. The engine decides, so no platform
    /// can read the book backwards.
    public static func swipeDirection(translation t: CGSize, velocity v: CGSize) -> Int? {
        qvpSwipePages(translation: t, velocity: v)
    }
}
