//! The reader's zoom control: what a pinch does to the page.
//!
//! A pinch can mean two different things on a muṣḥaf page. It can scale the printed page, the
//! way it scales a photograph, and leave the reader panning across lines that no longer fit.
//! Or it can ask for bigger ink at the same page width, which is reflow ([`crate::reflow`]):
//! fewer words to a row, the rest moved down, and the page grown taller.
//!
//! Which of those a pinch means, where it lands, when it commits and what holds the reader's
//! place across the relayout is one policy, and it lives here rather than in each host. A host
//! reads the gesture from its own recognizer and asks what it produced; it never sees the word
//! the page was held by, and never works out a zoom of its own
//! (`docs/standards/API-DESIGN.md`).
use crate::defaults;
use crate::hit::HitOptions;
use crate::layout::LayoutSpec;
use crate::view::View;
use crate::{Page, ReflowSpec};

/// What a pinch does to the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ZoomMode {
    /// The pinch lands on one of the page's own zoom steps and the page reflows onto it. What
    /// a reader gets unless a host asks otherwise: the rows only come out well at certain
    /// zooms, and the steps are the ones this page was measured to break best at.
    #[default]
    Stepped = 0,
    /// The pinch drives the reflow zoom itself, anywhere between the printed page and
    /// [`Page::reflow_max_zoom`].
    Continuous = 1,
    /// The pinch scales the laid-out page as it stands. The rows never change.
    Magnify = 2,
}

impl ZoomMode {
    /// The mode a host asked for by number, at the ABI. Anything else is the default.
    pub fn from_u32(v: u32) -> ZoomMode {
        match v {
            1 => ZoomMode::Continuous,
            2 => ZoomMode::Magnify,
            _ => ZoomMode::Stepped,
        }
    }
}

/// Where the reader's zoom control stands. A host keeps this beside its [`View`], and all zero
/// is the control a page opens on: stepped, on the printed page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    pub mode: ZoomMode,
    /// 0 is the printed page and 1 upwards are the page's steps ([`Page::zoom_steps`]). What
    /// [`ZoomMode::Stepped`] reads.
    pub step: u32,
    /// The reflow zoom in force, 1.0 at the printed page. What [`ZoomMode::Continuous`] reads.
    pub zoom: f32,
}

impl Default for Zoom {
    fn default() -> Self {
        Zoom { mode: ZoomMode::Stepped, step: 0, zoom: 1.0 }
    }
}

/// What a gesture produced: the control to keep, the view to draw with, and whether the page
/// was laid out again under it. A host that redraws its own overlays reads `relaid` to know
/// the geometry it cached has moved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomChange {
    pub zoom: Zoom,
    pub view: View,
    pub relaid: bool,
}

impl Page {
    /// The reflow zoom a control stands at: 1.0 for the printed page.
    fn zoom_of(&mut self, spec: &LayoutSpec, zoom: Zoom) -> f32 {
        match zoom.mode {
            ZoomMode::Continuous => zoom.zoom.max(1.0),
            _ => match zoom.step.checked_sub(1) {
                None => 1.0,
                Some(i) => self.zoom_steps(spec).get(i as usize).copied().unwrap_or(1.0),
            },
        }
    }

    /// The spec this control asks for: what the host lays out, draws and hit-tests with.
    ///
    /// A control on the printed page asks for no reflow at all, so every layout knob — leading,
    /// fill height, the grid a short page sits on — works as it does on a page nobody zoomed.
    pub fn zoom_spec(&mut self, spec: &LayoutSpec, zoom: Zoom) -> LayoutSpec {
        let z = self.zoom_of(spec, zoom);
        let mut out = *spec;
        out.reflow =
            if z > 1.0 + 1e-4 { Some(ReflowSpec { zoom: z, ..spec.reflow.unwrap_or_default() }) } else { None };
        out
    }

    /// The same control under another policy, keeping the size the reader is already at.
    ///
    /// Leaving [`ZoomMode::Stepped`] carries its step's zoom over as a free zoom; entering it
    /// takes the nearest step at or below that zoom, so the ink never jumps up when a reader
    /// only changed how the control behaves.
    pub fn zoom_mode(&mut self, spec: &LayoutSpec, zoom: Zoom, mode: ZoomMode) -> Zoom {
        if mode == zoom.mode {
            return zoom;
        }
        let z = self.zoom_of(spec, zoom);
        match mode {
            ZoomMode::Continuous => Zoom { mode, step: zoom.step, zoom: z },
            ZoomMode::Magnify => Zoom { mode, step: 0, zoom: 1.0 },
            ZoomMode::Stepped => {
                let steps = self.zoom_steps(spec);
                let step = steps.iter().filter(|&&s| s <= z + 1e-4).count() as u32;
                Zoom { mode, step, zoom: self.zoom_of(spec, Zoom { mode, step, zoom: z }) }
            }
        }
    }

    /// One frame of a pinch: the control it lands on, the view to draw with, and whether the
    /// page was laid out again.
    ///
    /// `factor` is the distance between the fingers against the distance when they went down —
    /// the whole gesture every time, not the change since the last frame — and `focal` is the
    /// point between them in viewport px. A host needs no call when the fingers lift: under
    /// the reflowing modes every frame leaves the control on a size it has committed to.
    pub fn zoom_pinch(
        &mut self,
        spec: &LayoutSpec,
        zoom: Zoom,
        view: View,
        factor: f32,
        focal: (f32, f32),
    ) -> ZoomChange {
        let factor = if factor.is_finite() && factor > 0.0 { factor } else { 1.0 };
        match zoom.mode {
            ZoomMode::Magnify => {
                ZoomChange { zoom, view: view.zoom_about(focal.0, focal.1, factor, 0.0, 0.0), relaid: false }
            }
            ZoomMode::Continuous => {
                let q = defaults::ZOOM_QUANTUM.max(1e-4);
                let cap = self.reflow_max_zoom(&spec.reflow.unwrap_or_default()).max(1.0);
                let want = ((self.zoom_of(spec, zoom) * factor).clamp(1.0, cap) / q).round() * q;
                self.commit(spec, zoom, Zoom { zoom: want, ..zoom }, view, focal)
            }
            ZoomMode::Stepped => {
                // Where the fingers stand, as a size. `zoom` is where the gesture began, so
                // this follows the fingers rather than the last step committed: opening them
                // further reaches the next step, closing them again comes back, and a step is
                // never passed through twice in one gesture.
                let live = self.zoom_of(spec, zoom) * factor;
                let steps = self.zoom_steps(spec);
                let n = steps.len() as u32;
                let at = |i: u32| -> f32 {
                    match i.checked_sub(1) {
                        None => 1.0,
                        Some(j) => steps.get(j as usize).copied().unwrap_or(1.0),
                    }
                };
                // Two neighbouring steps meet at their geometric mean, because a zoom is a
                // ratio and not a distance.
                let boundary = |i: u32| (at(i - 1) * at(i)).sqrt();
                let mut step = 0;
                for i in 1..=n {
                    if live >= boundary(i) {
                        step = i;
                    }
                }
                // The hysteresis holds fingers resting on a boundary from flickering between
                // two layouts. It applies to the step the page is on, so a gesture that has
                // already reached further is not dragged back by it.
                let h = 1.0 + defaults::ZOOM_SNAP_HYSTERESIS.max(0.0);
                let now = zoom.step;
                let inside_the_deadband = (step == now + 1 && live < boundary(step) * h)
                    || (now > 0 && step + 1 == now && live > boundary(now) / h);
                if inside_the_deadband {
                    step = now;
                }
                self.commit(spec, zoom, Zoom { step, ..zoom }, view, focal)
            }
        }
    }

    /// The control moved straight to a step: a size button, a double tap, a reset. Step 0 is
    /// the printed page.
    pub fn zoom_to_step(&mut self, spec: &LayoutSpec, zoom: Zoom, step: u32, view: View) -> ZoomChange {
        let n = self.zoom_steps(spec).len() as u32;
        let step = step.min(n);
        // A step names a size, so it answers under every mode: a host with a free zoom and a
        // row of size buttons keeps both.
        let next = match zoom.mode {
            ZoomMode::Continuous => {
                Zoom { zoom: self.zoom_of(spec, Zoom { mode: ZoomMode::Stepped, step, zoom: 1.0 }), step, ..zoom }
            }
            _ => Zoom { mode: ZoomMode::Stepped, step, zoom: 1.0 },
        };
        let focal = (spec.viewport_w / 2.0, spec.viewport_h / 2.0);
        self.commit(spec, zoom, next, view, focal)
    }

    /// Lay the page out at `next` and bring the reader's place back under `focal`.
    ///
    /// The word under the fingers is found on the layout in hand, before it is replaced: after
    /// the relayout that word is on another row and the point the reader was looking at is no
    /// longer where the pixels say. It is found again on every commit rather than remembered
    /// from the start of the gesture, because [`Page::view_anchor`] can only promise the
    /// vertical place — a word remembered from the start slides out from under the fingers,
    /// while the word found now is by definition the one they are on.
    fn commit(&mut self, spec: &LayoutSpec, from: Zoom, next: Zoom, view: View, focal: (f32, f32)) -> ZoomChange {
        // `zoom` always reads as the size in force, whichever field the mode steers by, so a
        // host can put it on screen without asking what a step means.
        let next = Zoom { zoom: self.zoom_of(spec, next), ..next };
        if (next.zoom - self.zoom_of(spec, from)).abs() < 1e-4 {
            return ZoomChange { zoom: next, view, relaid: false };
        }
        let (lx, ly) = self.view_to_layout(view, focal.0, focal.1);
        // The nearest word, however far: a tap refuses a hit that lands in the margin, but a
        // pinch only needs something on the page to hold on to, and the fingers often sit in
        // the air between two rows.
        let opt = HitOptions { prefer_exact: false, ..HitOptions::default() };
        let held = self.hit_test_view(lx, ly, &opt).filter(|h| h.word != u32::MAX).map(|h| {
            let (x0, y0, x1, y1) = self.word_bounds_view(h.word);
            let inside = |p: f32, a: f32, b: f32| if b > a { ((p - a) / (b - a)).clamp(0.0, 1.0) } else { 0.5 };
            (h.word, (inside(lx, x0, x1), inside(ly, y0, y1)))
        });
        let laid = self.zoom_spec(spec, next);
        let l = self.layout(&laid);
        let (content_w, content_h) = (l.content_w, l.content_h);
        let viewport = (spec.viewport_w, spec.viewport_h);
        let view = match held {
            Some((word, inside)) => self.view_anchor(view, word, inside, focal, viewport),
            None => view.clamp(content_w, content_h, viewport.0, viewport.1),
        };
        ZoomChange { zoom: next, view, relaid: true }
    }
}
