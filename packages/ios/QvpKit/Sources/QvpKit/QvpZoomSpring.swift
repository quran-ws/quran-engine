// Release animation for `zoomSpringsBack`: the view transform eases from where the pinch
// left it to the fitted one. Shared by QvpPageView (display link) and QvpCanvasController
// (main-actor task), so both views move the same way.
import Foundation
import CoreGraphics

struct QvpZoomSpring {
    typealias ViewTransform = (scale: CGFloat, offsetX: CGFloat, offsetY: CGFloat)
    static let duration = 0.25

    let from: ViewTransform
    let to: ViewTransform
    /// Seconds, on the same clock as the `now` passed to `value(at:)`.
    let start: Double

    /// The transform at `now`, and whether it has arrived.
    func value(at now: Double) -> (transform: ViewTransform, done: Bool) {
        let t = min(max((now - start) / Self.duration, 0), 1)
        let e = CGFloat(1 - pow(1 - t, 3)) // ease-out cubic
        return ((from.scale + (to.scale - from.scale) * e, from.offsetX + (to.offsetX - from.offsetX) * e, from.offsetY + (to.offsetY - from.offsetY) * e), t >= 1)
    }
}
