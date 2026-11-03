//! The reader's pan and zoom over a layout, and the zoom control a pinch drives.

use crate::abi::guard;
use crate::layout::layout_spec;
use crate::types::*;
use qvp_core::*;

/// Pan and zoom over a laid-out page: a point `p` in layout px draws at `offset + scale·p`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpView {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl From<QvpView> for qvp_core::View {
    fn from(v: QvpView) -> Self {
        qvp_core::View { scale: v.scale, offset_x: v.offset_x, offset_y: v.offset_y }
    }
}
impl From<qvp_core::View> for QvpView {
    fn from(v: qvp_core::View) -> Self {
        QvpView { scale: v.scale, offset_x: v.offset_x, offset_y: v.offset_y }
    }
}

/// Zoom by `factor` about a point of the viewport, holding the scale between `min` and `max`
/// (0 for the engine's own limits): what the point under two fingers asks for.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_zoom_about(
    view: *const QvpView,
    focal_x: f32,
    focal_y: f32,
    factor: f32,
    min: f32,
    max: f32,
    out: *mut QvpView,
) {
    guard(|| *out = qvp_core::View::from(*view).zoom_about(focal_x, focal_y, factor, min, max).into())
}
/// Move the view by a drag, in viewport px.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_pan(view: *const QvpView, dx: f32, dy: f32, out: *mut QvpView) {
    guard(|| *out = qvp_core::View::from(*view).pan(dx, dy).into())
}
/// Hold the content against the viewport: centre an axis it does not fill, cover the viewport
/// on an axis it overflows, so no drag opens a blank strip beside the page.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_clamp(
    view: *const QvpView,
    content_w: f32,
    content_h: f32,
    viewport_w: f32,
    viewport_h: f32,
    out: *mut QvpView,
) {
    guard(|| *out = qvp_core::View::from(*view).clamp(content_w, content_h, viewport_w, viewport_h).into())
}
/// The view that puts a point inside a word (`nx`, `ny` from 0 to 1) at a place on the screen,
/// then clamps. This is how a pinch holds its place when the layout reflows under it.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_anchor(
    page: *const Page,
    view: *const QvpView,
    word: u32,
    nx: f32,
    ny: f32,
    to_x: f32,
    to_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    out: *mut QvpView,
) {
    guard(|| *out = (*page).view_anchor((*view).into(), word, (nx, ny), (to_x, to_y), (viewport_w, viewport_h)).into())
}
/// Where a viewport point lands in the laid-out page, for turning a touch into a hit test.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_to_layout(page: *const Page, view: *const QvpView, vx: f32, vy: f32, out: *mut f32) {
    guard(|| {
        let (x, y) = (*page).view_to_layout((*view).into(), vx, vy);
        *out = x;
        *out.add(1) = y;
    })
}
/// +1 or -1 when a released drag is a page swipe, 0 when it is not. This is the direction the
/// finger went; `qvp_swipe_pages` reads it in the muṣḥaf's own order.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_swipe(dx: f32, dy: f32, vx: f32, vy: f32) -> i32 {
    guard(|| qvp_core::swipe_direction(dx, dy, vx, vy))
}

/// How many pages a released drag turns, in reading order: +1 the next page, -1 the one before,
/// 0 when it is not a swipe. A muṣḥaf is read right to left, so a flick right turns to the next.
#[no_mangle]
pub unsafe extern "C" fn qvp_swipe_pages(dx: f32, dy: f32, vx: f32, vy: f32) -> i32 {
    guard(|| qvp_core::swipe_pages(dx, dy, vx, vy))
}

// ───────────── the reader's zoom control ─────────────

/// Where the reader's zoom control stands. All zero is the control a page opens on: stepped,
/// on the printed page.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpZoom {
    /// 0 stepped, 1 continuous, 2 magnify. Anything else is stepped.
    pub mode: u32,
    /// 0 is the printed page, 1 upwards the page's own steps. What stepped reads.
    pub step: u32,
    /// The reflow zoom in force, 1.0 at the printed page. What continuous reads.
    pub zoom: f32,
}

/// What a gesture produced: the control to keep, the view to draw with, and whether the page
/// was laid out again under it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpZoomChange {
    pub zoom: QvpZoom,
    pub view: QvpView,
    pub relaid: u32,
}

impl From<QvpZoom> for qvp_core::Zoom {
    fn from(z: QvpZoom) -> Self {
        qvp_core::Zoom { mode: qvp_core::ZoomMode::from_u32(z.mode), step: z.step, zoom: z.zoom }
    }
}
impl From<qvp_core::Zoom> for QvpZoom {
    fn from(z: qvp_core::Zoom) -> Self {
        QvpZoom { mode: z.mode as u32, step: z.step, zoom: z.zoom }
    }
}
impl From<qvp_core::ZoomChange> for QvpZoomChange {
    fn from(c: qvp_core::ZoomChange) -> Self {
        QvpZoomChange { zoom: c.zoom.into(), view: c.view.into(), relaid: c.relaid as u32 }
    }
}

/// The same control under another policy, keeping the size the reader is already at.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_mode(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    mode: u32,
    out: *mut QvpZoom,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_mode(&s, (*zoom).into(), qvp_core::ZoomMode::from_u32(mode)).into()
    })
}

/// One frame of a pinch. `factor` is the distance between the fingers against the distance
/// when they went down — the whole gesture every time, not the change since the last frame.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_pinch(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    view: *const QvpView,
    factor: f32,
    focal_x: f32,
    focal_y: f32,
    out: *mut QvpZoomChange,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_pinch(&s, (*zoom).into(), (*view).into(), factor, (focal_x, focal_y)).into()
    })
}

/// The control moved straight to a step: a size button, a double tap, a reset. Step 0 is the
/// printed page.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_to_step(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    step: u32,
    view: *const QvpView,
    out: *mut QvpZoomChange,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_to_step(&s, (*zoom).into(), step, (*view).into()).into()
    })
}

/// The same control on another page: what the reader was reading at, carried onto the page they
/// turned to. A step carries as a step, a free zoom as a size held inside what the page can reach.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_carried(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    out: *mut QvpZoom,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_carried(&s, (*zoom).into()).into()
    })
}

/// The reflow zoom one step of this page's control means; step 0 is the printed page.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_at_step(page: *mut Page, spec: *const QvpLayoutSpec, step: u32) -> f32 {
    guard(|| {
        let s = layout_spec(spec);
        (*page).zoom_at_step(&s, step)
    })
}

/// 1 once the reader has zoomed in, by either road: a magnified view, or a page reflowed above
/// the printed size. `fit_scale` is the view scale the page is fitted at (0 = it is at it).
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_is_zoomed(zoom: *const QvpZoom, view: *const QvpView, fit_scale: f32) -> i32 {
    guard(|| qvp_core::Zoom::from(*zoom).is_zoomed((*view).into(), fit_scale) as i32)
}

/// What a sideways drag on this page means: 0 pan it, 1 turn the page.
#[no_mangle]
pub unsafe extern "C" fn qvp_sideways_drag(
    page: *const Page,
    zoom: *const QvpZoom,
    view: *const QvpView,
    fit_scale: f32,
) -> u32 {
    guard(|| (*page).sideways_drag((*zoom).into(), (*view).into(), fit_scale) as u32)
}

/// `spec` with this control's zoom in it: what the host lays out, draws and hit-tests with.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_spec(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    out: *mut QvpLayoutSpec,
) {
    guard(|| {
        let s = layout_spec(spec);
        // the control owns one field of the spec and leaves the reader's spacing knobs alone
        let z = (*page).zoom_spec(&s, (*zoom).into()).reflow.map(|r| r.zoom).unwrap_or(0.0);
        *out = QvpLayoutSpec { reflow_zoom: z, ..*spec };
    })
}
