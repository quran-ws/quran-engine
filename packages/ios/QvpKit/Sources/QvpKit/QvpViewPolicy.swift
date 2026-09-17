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
    /// How many pages a released drag turns, in the muṣḥaf's own order: +1 the page after this
    /// one, -1 the page before it, nil when it is not a swipe. The engine decides, so no platform
    /// can read the book backwards.
    public static func swipeDirection(translation t: CGSize, velocity v: CGSize) -> Int? {
        qvpSwipePages(translation: t, velocity: v)
    }
}
