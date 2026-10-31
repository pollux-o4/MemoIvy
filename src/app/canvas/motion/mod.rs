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
const INERTIA_FRICTION: f64 = 12.;
const INERTIA_START_SPEED: f64 = 120.;
const INERTIA_STOP_SPEED: f64 = 40.;
const INERTIA_MAX_SPEED: f64 = 1200.;

#[derive(Clone, Copy)]
struct Coast {
    velocity: (f64, f64),
}

impl Coast {
    fn new((vx, vy): (f64, f64)) -> Option<Self> {
        let speed = vx.hypot(vy);
        if !speed.is_finite() || speed < INERTIA_START_SPEED {
            return None;
        }
        let scale = (INERTIA_MAX_SPEED / speed).min(1.);
        Some(Self {
            velocity: (vx * scale, vy * scale),
        })
    }

    fn advance(self, elapsed: f64) -> (Option<Self>, (f64, f64)) {
        let (vx, vy) = self.velocity;
        let remaining = (vx.hypot(vy) / INERTIA_STOP_SPEED).ln() / INERTIA_FRICTION;
        let dt = elapsed.max(0.).min(remaining);
        let decay = (-INERTIA_FRICTION * dt).exp();
        let distance = (1. - decay) / INERTIA_FRICTION;
        let next = (elapsed < remaining).then_some(Self {
            velocity: (vx * decay, vy * decay),
        });
        (next, (vx * distance, vy * distance))
    }
}

const AUTOPAN_EDGE: f64 = 36.;
const AUTOPAN_SPEED: f64 = 900.;
const PANEL_INSET: f64 = 284.;

#[derive(Default)]
pub(super) struct MotionState {
    camera: Option<[Spring; 3]>,
    cards: HashMap<Uuid, [Spring; 2]>,
    inertia: Option<Coast>,
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
    v.session.scroll_panning.set(false);
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
    v.session.scroll_panning.set(false);
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
    let coast = animated(v).then(|| Coast::new(velocity)).flatten();
    v.motion.borrow_mut().inertia = coast;
    if coast.is_some() {
        start(v);
    }
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
    let began = Instant::now();
    let began_clock = glib::monotonic_time();
    motion.last = Some(began);
    drop(motion);
    let weak = Rc::downgrade(v);
    v.widgets.surface.add_tick_callback(move |_, clock| {
        let Some(v) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let elapsed =
            std::time::Duration::from_micros((clock.frame_time() - began_clock).max(0) as u64);
        if advance(&v, began + elapsed) {
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
        .map_or(0., |last| now.saturating_duration_since(last).as_secs_f64());
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
    if let Some(coast) = motion.inertia {
        if reduced {
            motion.inertia = None;
        } else {
            let current = camera.unwrap_or(v.session.camera.get());
            let (mut next, (dx, dy)) = coast.advance(elapsed);
            let candidate = current.panned(dx, dy);
            let kept = v.keep_camera(candidate);
            if let Some(coast) = &mut next {
                if kept.x != candidate.x {
                    coast.velocity.0 = 0.;
                }
                if kept.y != candidate.y {
                    coast.velocity.1 = 0.;
                }
                if coast.velocity.0.hypot(coast.velocity.1) <= INERTIA_STOP_SPEED {
                    next = None;
                }
            }
            motion.inertia = next;
            if kept != current {
                camera = Some(kept);
            }
        }
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

#[cfg(test)]
pub(in crate::app) mod tests;
