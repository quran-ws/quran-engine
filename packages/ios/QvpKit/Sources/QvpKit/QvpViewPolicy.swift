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
    /// +1 or -1 when a released drag is a page swipe, nil otherwise.
    public static func swipeDirection(translation t: CGSize, velocity v: CGSize) -> Int? {
        guard abs(t.width) > abs(t.height) * swipeAxisRatio, abs(t.width) > swipeDistance || abs(v.width) > swipeVelocity else { return nil }
        return t.width > 0 ? 1 : -1
    }
}
