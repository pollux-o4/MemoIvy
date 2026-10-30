use super::{
    input,
    model::{camera::Camera, layout::Rect, spring::Spring},
    scene, CanvasView,
};
use gtk::{glib, prelude::*};
use std::{collections::HashMap, rc::Rc, time::Instant};
use uuid::Uuid;

pub(super) const ZOOM_STEP: f64 = 1.25;

pub(super) const TRAVEL_SECONDS: f64 = 0.65;
pub(super) const SIGNAL_SECONDS: f64 = 0.95;
pub(super) const LINK_SECONDS: f64 = 0.28;
const CAMERA_RESPONSE: f64 = 0.34;
const CARD_RESPONSE: f64 = 0.36;
const CARD_DAMPING: f64 = 0.84;
const INERTIA_FRICTION: f64 = 4.5;
const AUTOPAN_EDGE: f64 = 36.;
const AUTOPAN_SPEED: f64 = 900.;
const PANEL_INSET: f64 = 284.;

#[derive(Default)]
pub(super) struct MotionState {
    camera: Option<[Spring; 3]>,
    cards: HashMap<Uuid, [Spring; 2]>,
    inertia: Option<(f64, f64)>,
    pub(super) pulse: Option<(Uuid, Instant)>,
    pub(super) link: Option<(Uuid, Uuid, Instant)>,
    last: Option<Instant>,
    running: bool,
}

fn ease(t: f64) -> f64 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// 곡선 위 위치와 불투명도. 모션 줄이기에서는 도착 상태만 정적으로 보여 준다.
pub(super) fn pulse_sample(elapsed: f64, animated: bool) -> (f64, f64) {
    if !animated {
        return (1., 1.);
    }
    (
        ease(elapsed / TRAVEL_SECONDS),
        1. - ((elapsed - TRAVEL_SECONDS) / (SIGNAL_SECONDS - TRAVEL_SECONDS)).clamp(0., 1.),
    )
}

pub(super) fn link_progress(elapsed: f64) -> f64 {
    ease(elapsed / LINK_SECONDS)
}

fn animated(v: &CanvasView) -> bool {
    v.widgets.window.settings().is_gtk_enable_animations() && v.widgets.surface.is_mapped()
}

fn viewport(v: &CanvasView) -> (f64, f64) {
    (
        f64::from(v.widgets.surface.width()).max(1.),
        f64::from(v.widgets.surface.height()).max(1.),
    )
}

fn center(camera: Camera, size: (f64, f64)) -> (f64, f64) {
    (
        camera.x + size.0 / camera.zoom / 2.,
        camera.y + size.1 / camera.zoom / 2.,
    )
}

/// 화면 중심과 로그 배율을 따로 보간해 확대가 일정한 속도로 느껴지게 한다.
pub(super) fn camera_to(v: &Rc<CanvasView>, target: Camera) {
    let mut motion = v.motion.borrow_mut();
    motion.inertia = None;
    if !animated(v) {
        motion.camera = None;
        drop(motion);
        scene::board::set_camera(v, target);
        return;
    }
    let size = viewport(v);
    let (cx, cy) = center(target, size);
    let values = [target.zoom.ln(), cx, cy];
    if let Some(springs) = &mut motion.camera {
        for (spring, value) in springs.iter_mut().zip(values) {
            spring.target = value;
        }
    } else {
        let now = v.session.camera.get();
        let (x, y) = center(now, size);
        let from = [now.zoom.ln(), x, y];
        motion.camera = Some(std::array::from_fn(|i| {
            let tolerance = if i == 0 { 0.0004 } else { 0.08 };
            Spring::new(from[i], values[i], CAMERA_RESPONSE, 1., tolerance)
        }));
    }
    drop(motion);
    start(v);
}

pub(super) fn stop_camera(v: &CanvasView) {
    let mut motion = v.motion.borrow_mut();
    motion.camera = None;
    motion.inertia = None;
}

pub(super) fn zoom_target(v: &CanvasView) -> f64 {
    v.motion
        .borrow()
        .camera
        .as_ref()
        .map_or(v.session.camera.get().zoom, |springs| {
            springs[0].target.exp()
        })
}

/// 뷰포트 중심을 기준으로 배율을 바꾼다. 전환 중 호출하면 현재 움직임에서 이어 간다.
pub(super) fn zoom_by(v: &Rc<CanvasView>, factor: f64) {
    let size = viewport(v);
    let anchor = (size.0 / 2., size.1 / 2.);
    let now = v.session.camera.get();
    camera_to(v, now.zoomed_at(anchor, zoom_target(v) * factor));
}

/// 카드를 목표 위치로 부드럽게 옮긴다. 이미 움직이는 카드는 속도를 유지한 채 방향을 바꾼다.
pub(super) fn card_to(v: &Rc<CanvasView>, id: Uuid, target: [f64; 2]) {
    if !animated(v) {
        v.motion.borrow_mut().cards.remove(&id);
        scene::board::show_card(v, id, target);
        return;
    }
    let from = v.session.shown.borrow().get(&id).copied().unwrap_or(target);
    let mut motion = v.motion.borrow_mut();
    let springs = motion.cards.entry(id).or_insert_with(|| {
        std::array::from_fn(|i| Spring::new(from[i], from[i], CARD_RESPONSE, CARD_DAMPING, 0.05))
    });
    springs[0].target = target[0];
    springs[1].target = target[1];
    drop(motion);
    start(v);
}

pub(super) fn stop_card(v: &CanvasView, id: Uuid) {
    v.motion.borrow_mut().cards.remove(&id);
}

pub(super) fn inertia(v: &Rc<CanvasView>, velocity: (f64, f64)) {
    if !animated(v) || velocity.0.hypot(velocity.1) < 60. {
        return;
    }
    v.motion.borrow_mut().inertia = Some(velocity);
    start(v);
}

pub(super) fn signal(v: &Rc<CanvasView>, id: Uuid) {
    let started = Instant::now();
    v.motion.borrow_mut().pulse = Some((id, started));
    v.widgets.backdrop.queue_draw();
    scene::paint::sync_hud(v);
    let weak = Rc::downgrade(v);
    glib::timeout_add_local_once(
        std::time::Duration::from_secs_f64(SIGNAL_SECONDS),
        move || {
            if let Some(v) = weak.upgrade() {
                let mut motion = v.motion.borrow_mut();
                if motion.pulse == Some((id, started)) {
                    motion.pulse = None;
                    drop(motion);
                    v.widgets.backdrop.queue_draw();
                    scene::paint::sync_hud(&v);
                }
            }
        },
    );
    start(v);
}

pub(super) fn link_added(v: &Rc<CanvasView>, from: Uuid, to: Uuid) {
    if animated(v) {
        v.motion.borrow_mut().link = Some((from, to, Instant::now()));
        start(v);
    }
}

/// 드래그 중 포인터가 가장자리에 닿으면 보드를 함께 밀어 준다.
pub(super) fn autopan_if_needed(v: &Rc<CanvasView>) {
    if autopan_direction(v) != (0., 0.) {
        start(v);
    }
}

fn autopan_direction(v: &CanvasView) -> (f64, f64) {
    if !v.session.interaction.borrow().follows_pointer() {
        return (0., 0.);
    }
    let (w, h) = viewport(v);
    let (x, y) = v.session.pointer.get();
    let axis = |p: f64, size: f64| {
        if p < AUTOPAN_EDGE {
            -((AUTOPAN_EDGE - p) / AUTOPAN_EDGE).min(1.)
        } else if p > size - AUTOPAN_EDGE {
            ((p - size + AUTOPAN_EDGE) / AUTOPAN_EDGE).min(1.)
        } else {
            0.
        }
    };
    (axis(x, w), axis(y, h))
}

fn start(v: &Rc<CanvasView>) {
    let mut motion = v.motion.borrow_mut();
    if motion.running {
        return;
    }
    motion.running = true;
    motion.last = None;
    drop(motion);
    let weak = Rc::downgrade(v);
    v.widgets.surface.add_tick_callback(move |_, _| {
        let Some(v) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if advance(&v, Instant::now()) {
            glib::ControlFlow::Continue
        } else {
            v.motion.borrow_mut().running = false;
            glib::ControlFlow::Break
        }
    });
}

/// 한 프레임을 진행하고 다음 프레임이 필요한지 돌려준다.
pub(super) fn advance(v: &Rc<CanvasView>, now: Instant) -> bool {
    let reduced = !v.widgets.window.settings().is_gtk_enable_animations();
    let size = viewport(v);
    let mut motion = v.motion.borrow_mut();
    let elapsed = motion
        .last
        .map_or(1. / 60., |last| now.duration_since(last).as_secs_f64());
    motion.last = Some(now);

    let mut camera = None;
    if let Some(springs) = &mut motion.camera {
        for spring in springs.iter_mut() {
            if reduced {
                spring.value = spring.target;
            } else {
                spring.advance(elapsed);
            }
        }
        let zoom = springs[0].value.exp();
        camera = Some(Camera {
            x: springs[1].value - size.0 / zoom / 2.,
            y: springs[2].value - size.1 / zoom / 2.,
            zoom,
        });
        if springs.iter().all(Spring::settled) || reduced {
            motion.camera = None;
        }
    }
    if let Some((vx, vy)) = motion.inertia {
        let current = camera.unwrap_or(v.session.camera.get());
        camera = Some(current.panned(vx * elapsed, vy * elapsed));
        let decay = (-INERTIA_FRICTION * elapsed).exp();
        let (vx, vy) = (vx * decay, vy * decay);
        motion.inertia = (vx.hypot(vy) >= 20. && !reduced).then_some((vx, vy));
    }

    let mut moved = Vec::with_capacity(motion.cards.len());
    motion.cards.retain(|id, springs| {
        for spring in springs.iter_mut() {
            if reduced {
                spring.value = spring.target;
            } else {
                spring.advance(elapsed);
            }
        }
        moved.push((*id, [springs[0].value, springs[1].value]));
        !(springs[0].settled() && springs[1].settled()) && !reduced
    });

    let pulse = motion
        .pulse
        .is_some_and(|(_, t)| now.duration_since(t).as_secs_f64() < SIGNAL_SECONDS);
    if !pulse {
        motion.pulse = None;
    }
    let link = motion
        .link
        .is_some_and(|(_, _, t)| now.duration_since(t).as_secs_f64() < LINK_SECONDS);
    if !link {
        motion.link = None;
    }
    let active = motion.camera.is_some()
        || motion.inertia.is_some()
        || !motion.cards.is_empty()
        || (pulse && !reduced)
        || link;
    drop(motion);

    for (id, position) in moved {
        scene::board::show_card(v, id, position);
    }
    let edge = autopan_direction(v);
    let mut panning = false;
    if edge != (0., 0.) {
        let step = AUTOPAN_SPEED * elapsed.min(0.05);
        camera = Some(
            camera
                .unwrap_or(v.session.camera.get())
                .panned(edge.0 * step, edge.1 * step),
        );
        panning = true;
    }
    if let Some(camera) = camera {
        scene::board::set_camera(v, camera);
    }
    if panning {
        input::pointer::follow(v);
    }
    v.widgets.backdrop.queue_draw();
    scene::paint::sync_hud(v);
    active || panning
}

/// 사각형이 보이지 않을 때만 옮긴다. 선택 패널에 가려지는 영역은 보이는 곳으로 치지 않는다.
pub(super) fn ensure_visible(v: &Rc<CanvasView>, rect: Rect) {
    let size = viewport(v);
    let covered = if v.session.selection.borrow().single().is_some() {
        PANEL_INSET
    } else {
        0.
    };
    let open = (size.0 - covered, size.1);
    let target = motion_target(v);
    if target.shows(rect, open, 24.) {
        return;
    }
    let next = if rect.w * target.zoom > open.0 - 48. || rect.h * target.zoom > open.1 - 48. {
        Camera::fitting(rect, open, 72.)
    } else {
        Camera::centered_on(rect, open, target.zoom)
    };
    camera_to(v, next);
}

fn motion_target(v: &CanvasView) -> Camera {
    let size = viewport(v);
    v.motion
        .borrow()
        .camera
        .as_ref()
        .map_or(v.session.camera.get(), |springs| {
            let zoom = springs[0].target.exp();
            Camera {
                x: springs[1].target - size.0 / zoom / 2.,
                y: springs[2].target - size.1 / zoom / 2.,
                zoom,
            }
        })
}

#[cfg(test)]
pub(in crate::app) mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn canvas_002_signal_is_bounded_and_reduced_motion_is_static() {
        assert_eq!(ease(-1.), 0.);
        assert_eq!(ease(2.), 1.);
        let samples: Vec<_> = (0..=100).map(|i| ease(i as f64 / 100.)).collect();
        assert!(samples.windows(2).all(|s| s[0] <= s[1]));
        assert_eq!(pulse_sample(0., true).0, 0.);
        assert_eq!(pulse_sample(SIGNAL_SECONDS, true), (1., 0.));
        assert_eq!(pulse_sample(0., false), (1., 1.));
        assert_eq!(link_progress(LINK_SECONDS), 1.);
    }

    pub(in crate::app) fn canvas_002_viewport(v: &Rc<CanvasView>) {
        let settings = v.widgets.window.settings();
        let original = settings.is_gtk_enable_animations();
        settings.set_gtk_enable_animations(true);
        let card = v.widgets.world.first_child().unwrap();
        let begin = Instant::now();
        let goal = |zoom| Camera { x: 0., y: 0., zoom };
        stop_camera(v);
        v.session.camera.set(goal(1.));
        camera_to(v, goal(0.6));
        v.motion.borrow_mut().last = Some(begin);
        assert!(advance(v, begin + Duration::from_millis(90)));
        let middle = v.session.camera.get().zoom;
        assert!(middle > 0.6 && middle < 1., "zoom interpolates: {middle}");
        camera_to(v, goal(0.8));
        assert!(!advance(v, begin + Duration::from_millis(1800)));
        assert!((v.session.camera.get().zoom - 0.8).abs() < 1e-6);
        assert_eq!(
            v.widgets.world.first_child().unwrap(),
            card,
            "zoom preserves native widgets and focus"
        );
        settings.set_gtk_enable_animations(false);
        camera_to(v, goal(1.));
        assert_eq!(v.session.camera.get().zoom, 1.);
        assert!(v.motion.borrow().camera.is_none());
        settings.set_gtk_enable_animations(original);
    }
}

pub(super) fn reset_zoom(v: &Rc<CanvasView>) {
    let size = (
        f64::from(v.widgets.surface.width()),
        f64::from(v.widgets.surface.height()),
    );
    camera_to(
        v,
        v.session
            .camera
            .get()
            .zoomed_at((size.0 / 2., size.1 / 2.), 1.),
    );
}
