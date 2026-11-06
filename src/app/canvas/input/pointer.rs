use super::super::{
    model::{camera::Camera, layout::Rect},
    motion, port, scene, CanvasView,
};
use super::Interaction;
use gtk::{gdk, glib, prelude::*, EventControllerScrollFlags as ScrollFlags};
use std::{cell::Cell, rc::Rc};
use uuid::Uuid;

const WHEEL_PAN: f64 = 60.;
const WHEEL_ZOOM: f64 = 0.18;
const TOUCHPAD_ZOOM: f64 = 0.012;

pub(in super::super) fn attach(v: &Rc<CanvasView>) {
    attach_scroll(v);
    attach_pinch(v);
    attach_board_drag(v);
    attach_port_drag(v);
    let motion_controller = gtk::EventControllerMotion::new();
    let weak = Rc::downgrade(v);
    motion_controller.connect_motion(move |_, x, y| {
        if let Some(v) = weak.upgrade() {
            if v.session.interaction.borrow().is_idle() {
                v.session.pointer.set((x, y));
                scene::board::update_port(&v);
            }
        }
    });
    let weak = Rc::downgrade(v);
    motion_controller.connect_leave(move |_| {
        if let Some(v) = weak.upgrade() {
            if v.session.interaction.borrow().is_idle() {
                v.session.pointer.set((-10_000., -10_000.));
                scene::board::update_port(&v);
            }
        }
    });
    v.widgets.surface.add_controller(motion_controller);
}

fn attach_scroll(v: &Rc<CanvasView>) {
    let scroll = gtk::EventControllerScroll::new(ScrollFlags::BOTH_AXES | ScrollFlags::KINETIC);
    let weak = Rc::downgrade(v);
    scroll.connect_scroll_begin(move |_| {
        if let Some(v) = weak.upgrade() {
            motion::stop_camera(&v);
        }
    });
    let weak = Rc::downgrade(v);
    scroll.connect_scroll(move |controller, dx, dy| {
        let Some(v) = weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let zooming = controller
            .current_event_state()
            .contains(gdk::ModifierType::CONTROL_MASK);
        scroll_input(&v, controller.unit(), (dx, dy), zooming);
        glib::Propagation::Stop
    });
    let weak = Rc::downgrade(v);
    scroll.connect_decelerate(move |controller, vx, vy| {
        if let Some(v) = weak.upgrade() {
            decelerate(&v, controller.unit(), (vx, vy));
        }
    });
    v.widgets.surface.add_controller(scroll);
}

fn scroll_input(v: &Rc<CanvasView>, unit: gdk::ScrollUnit, delta: (f64, f64), zooming: bool) {
    if unit == gdk::ScrollUnit::Wheel && !zooming {
        motion::wheel_pan(v, delta.0 * WHEEL_PAN, delta.1 * WHEEL_PAN);
    } else {
        motion::stop_camera(v);
        let next = scrolled(
            v.session.camera.get(),
            unit,
            delta,
            zooming,
            v.session.pointer.get(),
        );
        v.queue_camera(next);
        v.session.scroll_panning.set(!zooming);
    }
}

fn scrolled(
    camera: Camera,
    unit: gdk::ScrollUnit,
    (dx, dy): (f64, f64),
    zooming: bool,
    anchor: (f64, f64),
) -> Camera {
    let wheel = unit == gdk::ScrollUnit::Wheel;
    if zooming {
        let strength = if wheel { WHEEL_ZOOM } else { TOUCHPAD_ZOOM };
        camera.zoomed_at(anchor, camera.zoom * (-dy * strength).exp())
    } else {
        let unit = if wheel { WHEEL_PAN } else { 1. };
        camera.panned(dx * unit, dy * unit)
    }
}

fn decelerate(v: &Rc<CanvasView>, unit: gdk::ScrollUnit, velocity: (f64, f64)) {
    if v.session.scroll_panning.replace(false) && unit == gdk::ScrollUnit::Surface {
        // shortcut: GTK 4.22 구현은 초당 속도(문서는 ms 표기), GTK 업그레이드 시 scroll_history_finish 단위 재확인.
        motion::inertia(v, velocity);
    }
}

fn attach_pinch(v: &Rc<CanvasView>) {
    let pinch = gtk::GestureZoom::new();
    let begin = Rc::new(Cell::new(1.));
    let weak = Rc::downgrade(v);
    let start = begin.clone();
    pinch.connect_begin(move |_, _| {
        if let Some(v) = weak.upgrade() {
            motion::stop_camera(&v);
            start.set(v.session.camera.get().zoom);
        }
    });
    let weak = Rc::downgrade(v);
    pinch.connect_scale_changed(move |gesture, scale| {
        if let Some(v) = weak.upgrade() {
            let anchor = gesture
                .bounding_box_center()
                .unwrap_or(v.session.pointer.get());
            let next = v
                .session
                .camera
                .get()
                .zoomed_at(anchor, begin.get() * scale);
            scene::board::set_camera(&v, next);
        }
    });
    v.widgets.surface.add_controller(pinch);
}

fn attach_board_drag(v: &Rc<CanvasView>) {
    let drag = gtk::GestureDrag::new();
    drag.set_button(0);
    let weak = Rc::downgrade(v);
    drag.connect_drag_begin(move |gesture, x, y| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        if super::card::over_button(v.widgets.surface.upcast_ref(), x, y) {
            gesture.set_state(gtk::EventSequenceState::Denied);
            return;
        }
        let button = gesture.current_button();
        v.session.pointer.set((x, y));
        motion::stop_camera(&v);
        if button == gdk::BUTTON_MIDDLE || (button == gdk::BUTTON_PRIMARY && v.session.space.get())
        {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            *v.session.interaction.borrow_mut() = Interaction::Pan { last: (0., 0.) };
            v.widgets.surface.set_cursor_from_name(Some("grabbing"));
        } else if button == gdk::BUTTON_PRIMARY {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            let additive = gesture
                .current_event_state()
                .contains(gdk::ModifierType::SHIFT_MASK);
            let base = if additive {
                v.session.selection.borrow().ids().to_vec()
            } else {
                Vec::new()
            };
            *v.session.interaction.borrow_mut() = Interaction::Marquee {
                anchor: v.session.camera.get().to_world((x, y)),
                base,
                rect: None,
            };
            gtk::prelude::GtkWindowExt::set_focus(&v.widgets.window, None::<&gtk::Widget>);
        } else {
            gesture.set_state(gtk::EventSequenceState::Denied);
        }
    });
    let weak = Rc::downgrade(v);
    drag.connect_drag_update(move |gesture, dx, dy| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        if let Some((x, y)) = gesture.start_point() {
            v.session.pointer.set((x + dx, y + dy));
        }
        let pan = match &mut *v.session.interaction.borrow_mut() {
            Interaction::Pan { last } => {
                let step = (last.0 - dx, last.1 - dy);
                *last = (dx, dy);
                Some(step)
            }
            _ => None,
        };
        if let Some(step) = pan {
            pan_by(&v, step);
            return;
        }
        follow(&v);
        motion::autopan_if_needed(&v);
    });
    let weak = Rc::downgrade(v);
    drag.connect_drag_end(move |gesture, _, _| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        let interaction = v.session.interaction.replace(Interaction::Idle);
        match interaction {
            Interaction::Pan { .. } => {
                v.restore_cursor();
                v.render();
            }
            Interaction::Marquee { rect, .. } => {
                let additive = gesture
                    .current_event_state()
                    .contains(gdk::ModifierType::SHIFT_MASK);
                if rect.is_none() && !additive {
                    v.session.selection.borrow_mut().set([]);
                }
                v.render();
            }
            _ => {}
        }
    });
    let weak = Rc::downgrade(v);
    drag.connect_cancel(move |_, _| {
        if let Some(v) = weak.upgrade() {
            if !matches!(&*v.session.interaction.borrow(), Interaction::Move { .. }) {
                v.session.interaction.replace(Interaction::Idle);
                v.restore_cursor();
                scene::paint::sync_hud(&v);
            }
        }
    });
    v.widgets.surface.add_controller(drag);
}

/// 고속 마우스는 프레임마다 여러 번 움직임을 보낸다. 좌표만 누적하고 화면 갱신은 프레임 클록에 한 번 맞춘다.
pub(in super::super) fn pan_by(v: &Rc<CanvasView>, step: (f64, f64)) {
    v.queue_camera(v.session.camera.get().panned(step.0, step.1));
}

fn attach_port_drag(v: &Rc<CanvasView>) {
    let drag = gtk::GestureDrag::new();
    drag.set_button(gdk::BUTTON_PRIMARY);
    let raw_offset = Rc::new(Cell::new((0., 0.)));
    let weak = Rc::downgrade(v);
    let offset = raw_offset.clone();
    drag.connect_drag_begin(move |gesture, x, y| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        let Some(from) = v.session.port_owner.get() else {
            gesture.set_state(gtk::EventSequenceState::Denied);
            return;
        };
        let inside = v
            .widgets
            .port_handle
            .compute_point(
                &v.widgets.surface,
                &gtk::graphene::Point::new(x as f32, y as f32),
            )
            .map_or(v.session.pointer.get(), |p| {
                (f64::from(p.x()), f64::from(p.y()))
            });
        if let Some(raw) = gesture.current_event().and_then(|e| e.position()) {
            offset.set((raw.0 - inside.0, raw.1 - inside.1));
        }
        v.session.pointer.set(inside);
        gesture.set_state(gtk::EventSequenceState::Claimed);
        v.session.selection.borrow_mut().set([from]);
        v.sync_selection();
        *v.session.interaction.borrow_mut() = Interaction::Link {
            from,
            end: v.session.camera.get().to_world(inside),
            target: None,
        };
        scene::paint::sync_hud(&v);
    });
    let weak = Rc::downgrade(v);
    drag.connect_drag_update(move |gesture, _, _| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        if let Some(raw) = gesture.current_event().and_then(|e| e.position()) {
            v.session
                .pointer
                .set((raw.0 - raw_offset.get().0, raw.1 - raw_offset.get().1));
        }
        follow(&v);
        motion::autopan_if_needed(&v);
    });
    let weak = Rc::downgrade(v);
    drag.connect_drag_end(move |_, _, _| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        let interaction = v.session.interaction.replace(Interaction::Idle);
        if let Interaction::Link { from, target, .. } = interaction {
            if let Some(target) = target {
                v.connect_nodes(from, target);
            }
            scene::board::update_port(&v);
            scene::paint::sync_hud(&v);
        }
    });
    let weak = Rc::downgrade(v);
    drag.connect_cancel(move |_, _| {
        if let Some(v) = weak.upgrade() {
            if matches!(&*v.session.interaction.borrow(), Interaction::Link { .. }) {
                v.session.interaction.replace(Interaction::Idle);
                scene::paint::sync_hud(&v);
            }
        }
    });
    v.widgets.port_handle.add_controller(drag);
}

/// 포인터 또는 카메라가 움직였을 때 진행 중인 조작을 다시 계산한다.
pub(in super::super) fn follow(v: &Rc<CanvasView>) {
    let world = v.session.camera.get().to_world(v.session.pointer.get());
    let mut marquee = None;
    {
        let mut interaction = v.session.interaction.borrow_mut();
        match &mut *interaction {
            Interaction::Marquee { anchor, base, rect } => {
                let zoom = v.session.camera.get().zoom;
                let far = (world.0 - anchor.0).hypot(world.1 - anchor.1) * zoom;
                if rect.is_some() || far >= super::drag_threshold() {
                    let area = Rect::from_corners(*anchor, world);
                    *rect = Some(area);
                    marquee = Some((area, base.clone()));
                }
            }
            Interaction::Link { from, end, target } => {
                *end = world;
                let nodes = v.session.nodes.borrow();
                let from = *from;
                *target = nodes
                    .iter()
                    .rev()
                    .find(|n| n.id != from && v.shown_rect(n.id).is_some_and(|r| r.contains(world)))
                    .map(|n| n.id)
                    .filter(|id| port::can_connect(&nodes, from, *id));
            }
            Interaction::Move { .. } => {
                drop(interaction);
                super::card::follow(v);
                return;
            }
            _ => return,
        }
    }
    if let Some((area, base)) = marquee {
        let hit: Vec<Uuid> = v
            .session
            .shown
            .borrow()
            .iter()
            .filter(|(_, p)| port::rect_at(**p).intersects(area))
            .map(|(id, _)| *id)
            .collect();
        v.session
            .selection
            .borrow_mut()
            .set(base.into_iter().chain(hit));
        for (id, card) in v.widgets.cards.borrow().iter() {
            card.set_selected(v.session.selection.borrow().contains(*id));
        }
        v.widgets.backdrop.queue_draw();
    }
    scene::paint::sync_hud(v);
}

#[cfg(test)]
pub(in super::super) fn test_pinch_interrupts_scroll(v: &Rc<CanvasView>) {
    let controllers = v.widgets.surface.observe_controllers();
    let scroll = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerScroll>()
        })
        .unwrap();
    let pinch = (0..controllers.n_items())
        .find_map(|i| controllers.item(i).and_downcast::<gtk::GestureZoom>())
        .unwrap();
    motion::stop_camera(v);
    scene::board::set_camera(v, super::super::model::camera::Camera::default());
    let advance = || {
        let now = std::time::Instant::now();
        motion::advance(v, now);
        motion::advance(v, now + std::time::Duration::from_millis(100));
    };
    scroll.emit_by_name::<()>("scroll-begin", &[]);
    scroll_input(v, gdk::ScrollUnit::Surface, (0., 0.6), false);
    let before = v.session.camera.get();
    decelerate(v, gdk::ScrollUnit::Wheel, (1000., 0.));
    advance();
    assert_eq!(v.session.camera.get(), before, "wheel has no extra coast");
    scroll_input(v, gdk::ScrollUnit::Surface, (0., 0.6), false);
    let before = v.session.camera.get();
    decelerate(v, gdk::ScrollUnit::Surface, (1000., 0.));
    advance();
    assert!(
        v.session.camera.get().x > before.x,
        "Surface release keeps bounded coast"
    );
    scroll_input(v, gdk::ScrollUnit::Surface, (0., 0.6), false);
    pinch.emit_by_name::<()>("begin", &[&None::<gdk::EventSequence>]);
    // shortcut: 안전한 GDK Surface 이벤트 생성 API가 없어 감속 경계에 합성, 실제 터치패드 검증 때 재확인.
    decelerate(v, gdk::ScrollUnit::Surface, (1000., 0.));
    let before = v.session.camera.get();
    advance();
    assert_eq!(
        v.session.camera.get(),
        before,
        "pinch must invalidate prior scroll coast"
    );
    scroll_input(v, gdk::ScrollUnit::Surface, (0., 0.6), false);
    let before = v.session.camera.get();
    motion::camera_to(v, before);
    decelerate(v, gdk::ScrollUnit::Surface, (1000., 0.));
    advance();
    let after = v.session.camera.get();
    assert!(
        (after.x - before.x).abs() < 1e-6 && (after.y - before.y).abs() < 1e-6,
        "camera command must invalidate prior scroll coast"
    );
    motion::stop_camera(v);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_002_scroll_units_and_moving_zoom_anchor() {
        let camera = Camera {
            x: 120.,
            y: -40.,
            zoom: 0.8,
        };
        let surface = scrolled(
            camera,
            gdk::ScrollUnit::Surface,
            (0.25, -0.125),
            false,
            (0., 0.),
        );
        assert_eq!(surface, camera.panned(0.25, -0.125));
        let wheel = scrolled(camera, gdk::ScrollUnit::Wheel, (1., -1.), false, (0., 0.));
        assert_eq!(wheel, camera.panned(60., -60.));
        for unit in [gdk::ScrollUnit::Wheel, gdk::ScrollUnit::Surface] {
            let mut current = camera;
            for (anchor, dy) in [
                ((250., 180.), -0.25),
                ((270., 185.), -0.125),
                ((260., 200.), 0.5),
            ] {
                let world = current.to_world(anchor);
                let next = scrolled(current, unit, (0., dy), true, anchor);
                let after = next.to_world(anchor);
                assert!((after.0 - world.0).abs() < 1e-9 && (after.1 - world.1).abs() < 1e-9);
                current = next;
            }
        }
    }
}
