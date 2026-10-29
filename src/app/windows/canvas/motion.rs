use super::{board, CanvasView};
use gtk::{glib, prelude::*};
use std::{
    rc::Rc,
    time::{Duration, Instant},
};
use uuid::Uuid;

const ZOOM_SECONDS: f64 = 0.22;
pub(super) const TRAVEL_SECONDS: f64 = 0.65;
pub(super) const SIGNAL_SECONDS: f64 = 0.95;

#[derive(Clone, Copy)]
pub(super) struct ZoomTransition {
    from: [f64; 3],
    to: [f64; 3],
    started: Instant,
}

fn ease(t: f64) -> f64 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

pub(super) fn pulse_sample(elapsed: f64, animated: bool) -> (f64, f64) {
    if !animated {
        return (1., 1.);
    }
    (
        ease(elapsed / TRAVEL_SECONDS),
        1. - ((elapsed - TRAVEL_SECONDS) / (SIGNAL_SECONDS - TRAVEL_SECONDS)).clamp(0., 1.),
    )
}

pub(super) fn zoom_by(v: &Rc<CanvasView>, factor: f64) {
    let zoom =
        (v.zoom_transition.get().map_or(v.zoom.get(), |t| t.to[0]) * factor).clamp(0.02, 1.5);
    let old = v.zoom.get();
    let (x, y) = (
        v.scroll.hadjustment().value(),
        v.scroll.vadjustment().value(),
    );
    let (cx, cy) = (v.scroll.width() as f64 / 2., v.scroll.height() as f64 / 2.);
    transition(
        v,
        [
            zoom,
            ((x + cx) / old * zoom - cx).max(0.),
            ((y + cy) / old * zoom - cy).max(0.),
        ],
    );
}

pub(super) fn transition(v: &Rc<CanvasView>, to: [f64; 3]) {
    if v.dragging.get() {
        return;
    }
    if !v.window.settings().is_gtk_enable_animations() {
        v.zoom_transition.set(None);
        apply(v, to);
        return;
    }
    v.zoom_transition.set(Some(ZoomTransition {
        from: [
            v.zoom.get(),
            v.scroll.hadjustment().value(),
            v.scroll.vadjustment().value(),
        ],
        to,
        started: Instant::now(),
    }));
    start(v);
}

fn apply(v: &CanvasView, viewport: [f64; 3]) {
    v.zoom.set(viewport[0]);
    board::rescale(v);
    v.scroll.hadjustment().set_value(viewport[1]);
    v.scroll.vadjustment().set_value(viewport[2]);
    v.drawing.queue_draw();
}

pub(super) fn signal(v: &Rc<CanvasView>, id: Uuid) {
    let started = Instant::now();
    v.pulse.set(Some((id, started)));
    v.drawing.queue_draw();
    let weak = Rc::downgrade(v);
    glib::timeout_add_local_once(Duration::from_secs_f64(SIGNAL_SECONDS), move || {
        if let Some(v) = weak.upgrade() {
            if v.pulse.get() == Some((id, started)) {
                v.pulse.set(None);
                v.drawing.queue_draw();
            }
        }
    });
    if v.window.settings().is_gtk_enable_animations() {
        start(v);
    }
}

fn start(v: &Rc<CanvasView>) {
    if v.frame_active.replace(true) {
        return;
    }
    let weak = Rc::downgrade(v);
    v.drawing.add_tick_callback(move |_, _| {
        let Some(v) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if advance(&v, Instant::now()) {
            glib::ControlFlow::Continue
        } else {
            v.frame_active.set(false);
            glib::ControlFlow::Break
        }
    });
}

fn advance(v: &CanvasView, now: Instant) -> bool {
    let animated = v.window.settings().is_gtk_enable_animations();
    if let Some(t) = v.zoom_transition.get() {
        let progress = if animated {
            now.duration_since(t.started).as_secs_f64() / ZOOM_SECONDS
        } else {
            1.
        };
        let viewport = std::array::from_fn(|i| t.from[i] + (t.to[i] - t.from[i]) * ease(progress));
        apply(v, viewport);
        if progress >= 1. {
            v.zoom_transition.set(None);
        }
    }
    let pulse = v
        .pulse
        .get()
        .is_some_and(|(_, t)| now.duration_since(t).as_secs_f64() < SIGNAL_SECONDS);
    if !pulse {
        v.pulse.set(None);
    }
    v.drawing.queue_draw();
    v.zoom_transition.get().is_some() || (pulse && animated)
}

#[cfg(test)]
pub(in crate::app) mod tests {
    use super::*;
    #[test]
    fn canvas_002_motion_is_bounded_and_reduced_motion_is_static() {
        assert_eq!(ease(-1.), 0.);
        assert_eq!(ease(2.), 1.);
        let samples: Vec<_> = (0..=100).map(|i| ease(i as f64 / 100.)).collect();
        assert!(samples.windows(2).all(|s| s[0] <= s[1]));
        assert!(ease(0.01) < 0.001 && ease(0.99) > 0.999);
        assert_eq!(pulse_sample(0., true).0, 0.);
        assert_eq!(pulse_sample(SIGNAL_SECONDS, true), (1., 0.));
        assert_eq!(pulse_sample(0., false), (1., 1.));
    }

    pub(in crate::app) fn canvas_002_viewport(v: &Rc<CanvasView>) {
        let settings = v.window.settings();
        let original = settings.is_gtk_enable_animations();
        settings.set_gtk_enable_animations(true);
        let card = v.fixed.first_child().unwrap();
        transition(v, [0.6, 0., 0.]);
        let t = v.zoom_transition.get().unwrap();
        assert!(advance(v, t.started + Duration::from_millis(110)));
        assert!(v.zoom.get() > 0.6 && v.zoom.get() < 1.);
        transition(v, [0.8, 0., 0.]);
        let t = v.zoom_transition.get().unwrap();
        assert!(!advance(v, t.started + Duration::from_secs(1)));
        assert_eq!(v.zoom.get(), 0.8);
        assert_eq!(
            v.fixed.first_child().unwrap(),
            card,
            "zoom preserves native widgets and focus"
        );
        settings.set_gtk_enable_animations(false);
        transition(v, [1., 0., 0.]);
        assert_eq!(v.zoom.get(), 1.);
        assert!(v.zoom_transition.get().is_none());
        settings.set_gtk_enable_animations(original);
    }
}
