use super::super::{
    model::layout::{self, clamp_delta, Rect},
    motion, port,
    scene::{self, card_root::CardRoot},
    CanvasView,
};
use super::{Interaction, Press};
use gtk::{gdk, prelude::*, GestureDrag};
use std::{cell::Cell, rc::Rc};
use uuid::Uuid;

const SNAP_PIXELS: f64 = 6.;

/// 카드 전체를 끌어 옮기고, 눌러서 선택하며, 두 번 눌러 원본을 연다.
pub(in super::super) fn attach(v: &Rc<CanvasView>, card: &CardRoot, id: Uuid) {
    let focus = gtk::EventControllerFocus::new();
    let weak = Rc::downgrade(v);
    focus.connect_enter(move |_| {
        if let Some(v) = weak.upgrade() {
            if v.session.presenting.get() {
                return;
            }
            if !v.session.selection.borrow().contains(id) {
                v.select_only(id);
            }
            if v.session.interaction.borrow().is_idle() {
                if let Some(rect) = v.shown_rect(id) {
                    motion::ensure_visible(&v, rect);
                }
            }
        }
    });
    card.add_controller(focus);

    let drag = GestureDrag::new();
    drag.set_button(gdk::BUTTON_PRIMARY);
    let raw_offset = Rc::new(Cell::new((0., 0.)));
    let weak = Rc::downgrade(v);
    let offset = raw_offset.clone();
    drag.connect_drag_begin(move |gesture, x, y| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        let busy = v
            .state
            .upgrade()
            .is_some_and(|st| st.borrow().notebook.busy());
        let Some(widget) = gesture.widget() else {
            return;
        };
        if busy || v.session.space.get() || over_button(&widget, x, y) {
            gesture.set_state(gtk::EventSequenceState::Denied);
            return;
        }
        // 카드는 끌리는 동안 움직이므로 창 기준 좌표로 포인터를 추적한다.
        let inside = widget
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
        motion::stop_camera(&v);
        let additive = gesture
            .current_event_state()
            .contains(gdk::ModifierType::SHIFT_MASK);
        let selected = v.session.selection.borrow().contains(id);
        let press = match (additive, selected) {
            (true, true) => Press::Toggle(id),
            (true, false) => {
                v.session.selection.borrow_mut().toggle(id);
                Press::Keep
            }
            (false, true) => Press::Collapse(id),
            (false, false) => {
                v.session.selection.borrow_mut().set([id]);
                Press::Keep
            }
        };
        v.sync_selection();
        let origins: Vec<(Uuid, [f64; 2])> = {
            let selection = v.session.selection.borrow();
            let shown = v.session.shown.borrow();
            selection
                .ids()
                .iter()
                .filter_map(|other| shown.get(other).map(|p| (*other, *p)))
                .collect()
        };
        *v.session.interaction.borrow_mut() = Interaction::Move {
            grab: v.session.camera.get().to_world(inside),
            origins,
            moved: false,
            press,
        };
        // 조작을 먼저 등록해 포커스 이동이 눌린 카드 아래의 보드를 움직이지 않게 한다.
        widget.grab_focus();
    });
    let weak = Rc::downgrade(v);
    let offset = raw_offset.clone();
    drag.connect_drag_update(move |gesture, _, _| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        if let Some(raw) = gesture.current_event().and_then(|e| e.position()) {
            v.session
                .pointer
                .set((raw.0 - offset.get().0, raw.1 - offset.get().1));
        }
        let state = gesture.current_event_state();
        v.session.drag_modifiers.set((
            state.contains(gdk::ModifierType::CONTROL_MASK),
            state.contains(gdk::ModifierType::SHIFT_MASK),
        ));
        follow(&v);
        motion::autopan_if_needed(&v);
    });
    let weak = Rc::downgrade(v);
    drag.connect_drag_end(move |gesture, _, _| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        let interaction = v.session.interaction.replace(Interaction::Idle);
        let Interaction::Move {
            origins,
            moved,
            press,
            ..
        } = interaction
        else {
            return;
        };
        v.session.guides.borrow_mut().clear();
        let target = drop_target(&v, &origins);
        clear_drag_style(&v);
        scene::paint::sync_hud(&v);
        if let (true, Some(target)) = (moved, target) {
            v.move_into(origins.iter().map(|o| o.0).collect(), Some(target));
            return;
        }
        if moved {
            let moves: layout::Moves = {
                let shown = v.session.shown.borrow();
                origins
                    .iter()
                    .filter_map(|(id, _)| shown.get(id).map(|p| (*id, *p)))
                    .collect()
            };
            v.mutate(|notes| port::move_nodes(notes, &moves));
            return;
        }
        match press {
            Press::Keep => {}
            Press::Collapse(id) => v.session.selection.borrow_mut().set([id]),
            Press::Toggle(id) => v.session.selection.borrow_mut().toggle(id),
        }
        v.sync_selection();
        let time = gesture.current_event_time();
        let window = i64::from(v.widgets.window.settings().gtk_double_click_time());
        let again = v
            .session
            .last_click
            .get()
            .is_some_and(|(last, at)| last == id && i64::from(time) - i64::from(at) <= window);
        if again {
            v.session.last_click.set(None);
            if v.node(id).is_some_and(|n| n.project.is_some()) {
                v.enter(id);
            } else {
                v.open_note(id);
            }
        } else {
            v.session.last_click.set(Some((id, time)));
        }
    });
    let weak = Rc::downgrade(v);
    drag.connect_cancel(move |_, _| {
        if let Some(v) = weak.upgrade() {
            cancel(&v);
        }
    });
    card.add_controller(drag);
}

/// 카드 안의 버튼을 누른 경우 끌기가 입력을 가로채지 않게 한다.
pub(super) fn over_button(card: &gtk::Widget, x: f64, y: f64) -> bool {
    let mut target = card.pick(x, y, gtk::PickFlags::DEFAULT);
    while let Some(widget) = target {
        if widget == *card {
            return false;
        }
        if widget.is::<gtk::Button>() {
            return true;
        }
        target = widget.parent();
    }
    false
}

/// 끌기를 취소하고 카드를 제자리로 돌려보낸다.
pub(in super::super) fn cancel(v: &Rc<CanvasView>) {
    let interaction = v.session.interaction.replace(Interaction::Idle);
    if let Interaction::Move { origins, moved, .. } = interaction {
        v.session.guides.borrow_mut().clear();
        clear_drag_style(v);
        if moved {
            for (id, origin) in origins {
                motion::card_to(v, id, origin);
            }
        }
        scene::paint::sync_hud(v);
    }
}

/// 현재 포인터·카메라로 끌기 결과를 다시 계산한다. 자동 팬 중에도 호출된다.
pub(in super::super) fn follow(v: &Rc<CanvasView>) {
    let (grab, origins, moved) = match &*v.session.interaction.borrow() {
        Interaction::Move {
            grab,
            origins,
            moved,
            ..
        } => (*grab, origins.clone(), *moved),
        _ => return,
    };
    let camera = v.session.camera.get();
    let world = camera.to_world(v.session.pointer.get());
    let mut delta = (world.0 - grab.0, world.1 - grab.1);
    if !moved {
        if delta.0.hypot(delta.1) * camera.zoom < super::drag_threshold() {
            return;
        }
        begin_move(v, &origins);
    }
    let (no_snap, lock_axis) = v.session.drag_modifiers.get();
    if lock_axis {
        if delta.0.abs() >= delta.1.abs() {
            delta.1 = 0.;
        } else {
            delta.0 = 0.;
        }
    }
    let positions: Vec<[f64; 2]> = origins.iter().map(|o| o.1).collect();
    delta = clamp_delta(&positions, delta, port::LIMIT);
    let mut guides = Vec::new();
    if !no_snap {
        let moving = layout::bounds(
            origins
                .iter()
                .map(|(_, p)| port::rect_at([p[0] + delta.0, p[1] + delta.1])),
        );
        if let Some(moving) = moving {
            let others = others(v, &origins);
            let (dx, dy, found) = layout::snap(moving, &others, SNAP_PIXELS / camera.zoom);
            let adjusted = clamp_delta(
                &positions,
                (
                    delta.0 + if lock_axis && delta.0 == 0. { 0. } else { dx },
                    delta.1 + if lock_axis && delta.1 == 0. { 0. } else { dy },
                ),
                port::LIMIT,
            );
            delta = adjusted;
            guides = found;
        }
    }
    for (id, origin) in &origins {
        scene::board::show_card(v, *id, [origin[0] + delta.0, origin[1] + delta.1]);
    }
    let target = drop_target(v, &origins);
    for (id, card) in v.widgets.cards.borrow().iter() {
        if Some(*id) == target {
            card.root.add_css_class("drop-target");
        } else {
            card.root.remove_css_class("drop-target");
        }
    }
    *v.session.guides.borrow_mut() = guides;
    v.widgets.backdrop.queue_draw();
    scene::paint::sync_hud(v);
}

fn begin_move(v: &Rc<CanvasView>, origins: &[(Uuid, [f64; 2])]) {
    if let Interaction::Move { moved, .. } = &mut *v.session.interaction.borrow_mut() {
        *moved = true;
    }
    for (id, _) in origins {
        motion::stop_card(v, *id);
        if let Some(card) = v.widgets.cards.borrow().get(id) {
            card.root.add_css_class("dragging");
            // 끄는 카드를 맨 위로. 연결 핸들은 항상 그보다 위에 둔다.
            card.root
                .insert_before(&v.widgets.world, Some(&v.widgets.port_handle));
        }
    }
    scene::board::update_port(v);
    v.session.drag_styled.set(true);
}

fn clear_drag_style(v: &CanvasView) {
    if v.session.drag_styled.replace(false) {
        for card in v.widgets.cards.borrow().values() {
            card.root.remove_css_class("dragging");
            card.root.remove_css_class("drop-target");
        }
    }
}

/// 포인터 아래에 끌고 있지 않은 프로젝트 카드가 있으면 그 안으로 넣는다.
fn drop_target(v: &CanvasView, moving: &[(Uuid, [f64; 2])]) -> Option<Uuid> {
    let point = v.session.camera.get().to_world(v.session.pointer.get());
    let shown = v.session.shown.borrow();
    v.session
        .nodes
        .borrow()
        .iter()
        .filter(|n| n.project.is_some() && !moving.iter().any(|(id, _)| *id == n.id))
        .find(|n| {
            shown.get(&n.id).is_some_and(|p| {
                let r = port::rect_at(*p);
                point.0 >= r.x && point.0 <= r.right() && point.1 >= r.y && point.1 <= r.bottom()
            })
        })
        .map(|n| n.id)
}

fn others(v: &CanvasView, moving: &[(Uuid, [f64; 2])]) -> Vec<Rect> {
    let shown = v.session.shown.borrow();
    let camera = v.session.camera.get();
    let size = (
        f64::from(v.widgets.surface.width()) / camera.zoom,
        f64::from(v.widgets.surface.height()) / camera.zoom,
    );
    let visible = Rect::new(camera.x, camera.y, size.0, size.1);
    shown
        .iter()
        .filter(|(id, _)| !moving.iter().any(|(other, _)| other == *id))
        .map(|(_, p)| port::rect_at(*p))
        .filter(|r| r.intersects(visible))
        .collect()
}
