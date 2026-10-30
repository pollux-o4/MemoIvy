use super::super::{
    model::layout::{self, Align, Axis},
    motion, port, scene, CanvasView,
};
use super::Interaction;
use gtk::{gdk, glib, prelude::*};
use std::rc::Rc;

const NUDGE: f64 = 16.;
const NUDGE_LARGE: f64 = 80.;

pub(in super::super) fn attach(v: &Rc<CanvasView>) {
    let keys = gtk::EventControllerKey::new();
    // GtkWindow의 방향키 포커스 이동보다 먼저 받아야 카드를 옮길 수 있다.
    // 입력 위젯이 쓰는 키는 handle에서 그대로 돌려보낸다.
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = Rc::downgrade(v);
    keys.connect_key_pressed(move |_, key, _, modifiers| match weak.upgrade() {
        Some(v) if handle(&v, key, modifiers) => glib::Propagation::Stop,
        _ => glib::Propagation::Proceed,
    });
    let weak = Rc::downgrade(v);
    keys.connect_key_released(move |_, key, _, _| {
        if key == gdk::Key::space {
            if let Some(v) = weak.upgrade() {
                v.session.space.set(false);
                v.restore_cursor();
            }
        }
    });
    v.widgets.window.add_controller(keys);
    // 키를 누른 채 창을 떠나면 놓는 이벤트가 오지 않는다.
    let weak = Rc::downgrade(v);
    v.widgets.window.connect_is_active_notify(move |window| {
        if !window.is_active() {
            if let Some(v) = weak.upgrade() {
                v.session.space.set(false);
                v.restore_cursor();
            }
        }
    });
}

/// 입력 위젯이 쓰는 키를 빼앗지 않도록 보드에 포커스가 있을 때만 보드 키를 처리한다.
fn board_has_focus(v: &CanvasView) -> bool {
    let mut focus = gtk::prelude::GtkWindowExt::focus(&v.widgets.window);
    if focus.is_none() {
        return true;
    }
    while let Some(widget) = focus {
        if widget == *v.widgets.surface.upcast_ref::<gtk::Widget>() {
            return true;
        }
        focus = widget.parent();
    }
    false
}

fn focus_in_popover(v: &CanvasView) -> bool {
    let mut focus = gtk::prelude::GtkWindowExt::focus(&v.widgets.window);
    while let Some(widget) = focus {
        if widget.is::<gtk::Popover>() {
            return true;
        }
        focus = widget.parent();
    }
    false
}

fn handle(v: &Rc<CanvasView>, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
    let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
    let shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
    let alt = modifiers.contains(gdk::ModifierType::ALT_MASK);
    if focus_in_popover(v) {
        return false;
    }
    if key == gdk::Key::Escape {
        return escape(v);
    }
    if !ctrl
        && !alt
        && !shift
        && matches!(
            key,
            gdk::Key::Delete | gdk::Key::KP_Delete | gdk::Key::BackSpace
        )
    {
        // 보드에 글자 입력 위젯이 없어 패널 버튼에 포커스가 있어도 선택을 지운다.
        if v.session.selection.borrow().is_empty() {
            return false;
        }
        v.delete_selected();
        return true;
    }
    if ctrl && !alt {
        return match key.to_lower() {
            gdk::Key::Return | gdk::Key::KP_Enter => {
                v.toggle_selected_done();
                true
            }
            gdk::Key::l => connect_menu(v),
            gdk::Key::z if shift => {
                v.redo();
                true
            }
            gdk::Key::z => {
                v.undo();
                true
            }
            gdk::Key::y => {
                v.redo();
                true
            }
            gdk::Key::a if board_has_focus(v) => {
                let ids: Vec<_> = v.session.nodes.borrow().iter().map(|n| n.id).collect();
                v.session.selection.borrow_mut().set(ids);
                v.sync_selection();
                true
            }
            gdk::Key::_0 | gdk::Key::KP_0 => {
                motion::reset_zoom(v);
                true
            }
            gdk::Key::equal | gdk::Key::plus | gdk::Key::KP_Add => {
                motion::zoom_by(v, motion::ZOOM_STEP);
                true
            }
            gdk::Key::minus | gdk::Key::KP_Subtract => {
                motion::zoom_by(v, 1. / motion::ZOOM_STEP);
                true
            }
            _ => false,
        };
    }
    if alt && !ctrl {
        return alignment(v, key, shift);
    }
    if !board_has_focus(v) {
        return false;
    }
    if matches!(key, gdk::Key::Tab | gdk::Key::ISO_Left_Tab) && !ctrl && !alt {
        return tab_to_card(v, shift || key == gdk::Key::ISO_Left_Tab);
    }
    match key {
        gdk::Key::exclam | gdk::Key::_1 if shift => {
            v.fit();
            true
        }
        gdk::Key::at | gdk::Key::_2 if shift => {
            v.fit_selection();
            true
        }
        gdk::Key::space => {
            if !v.session.space.replace(true) {
                v.widgets.surface.set_cursor_from_name(Some("grab"));
            }
            true
        }
        gdk::Key::Return | gdk::Key::KP_Enter => {
            let id = v.session.selection.borrow().single();
            if let Some(id) = id {
                v.open_note(id);
            }
            id.is_some()
        }
        gdk::Key::Left | gdk::Key::Right | gdk::Key::Up | gdk::Key::Down => {
            nudge(v, key, if shift { NUDGE_LARGE } else { NUDGE })
        }
        _ => false,
    }
}

/// 선택한 메모의 다음 메모 목록을 연다. 후보가 없거나 다중 선택이면 하지 않는다.
fn connect_menu(v: &Rc<CanvasView>) -> bool {
    if v.session.selection.borrow().single().is_none() {
        return false;
    }
    let mut stack = vec![v.widgets.inspector.clone().upcast::<gtk::Widget>()];
    while let Some(widget) = stack.pop() {
        if widget.widget_name() == "canvas-control-connect" {
            if let Some(menu) = widget.downcast_ref::<gtk::MenuButton>() {
                menu.popup();
                return true;
            }
        }
        let mut child = widget.first_child();
        while let Some(next) = child {
            child = next.next_sibling();
            stack.push(next);
        }
    }
    false
}

/// 화면 밖이라 숨겨진 카드까지 읽는 순서대로 포커스를 옮긴다. 끝에 닿으면 도구 막대로 넘긴다.
fn tab_to_card(v: &Rc<CanvasView>, backwards: bool) -> bool {
    let mut order: Vec<_> = v
        .session
        .nodes
        .borrow()
        .iter()
        .map(|n| (n.position, n.id))
        .collect();
    order.sort_by(|a, b| a.0[1].total_cmp(&b.0[1]).then(a.0[0].total_cmp(&b.0[0])));
    let current = gtk::prelude::GtkWindowExt::focus(&v.widgets.window)
        .and_then(|w| uuid::Uuid::parse_str(w.widget_name().as_str()).ok())
        .and_then(|id| order.iter().position(|(_, other)| *other == id));
    let next = match (current, backwards) {
        (None, false) => Some(0),
        (None, true) => order.len().checked_sub(1),
        (Some(i), false) => (i + 1 < order.len()).then_some(i + 1),
        (Some(i), true) => i.checked_sub(1),
    };
    let Some((_, id)) = next.and_then(|i| order.get(i)) else {
        return false;
    };
    if let Some(card) = v.widgets.cards.borrow().get(id) {
        card.root.set_visible(true);
        card.root.grab_focus();
    }
    true
}

fn escape(v: &Rc<CanvasView>) -> bool {
    let interaction = v.session.interaction.borrow().clone();
    match interaction {
        Interaction::Idle => {
            if v.session.selection.borrow().is_empty() {
                return false;
            }
            v.session.selection.borrow_mut().set([]);
            v.sync_selection();
            gtk::prelude::GtkWindowExt::set_focus(&v.widgets.window, None::<&gtk::Widget>);
        }
        Interaction::Move { .. } => super::card::cancel(v),
        _ => {
            v.session.interaction.replace(Interaction::Idle);
            v.restore_cursor();
            scene::paint::sync_hud(v);
        }
    }
    true
}

fn nudge(v: &Rc<CanvasView>, key: gdk::Key, step: f64) -> bool {
    let (dx, dy) = match key {
        gdk::Key::Left => (-step, 0.),
        gdk::Key::Right => (step, 0.),
        gdk::Key::Up => (0., -step),
        _ => (0., step),
    };
    let selected = v.selected_nodes();
    if selected.is_empty() {
        return false;
    }
    let positions: Vec<[f64; 2]> = selected.iter().map(|n| [n.1.x, n.1.y]).collect();
    let (dx, dy) = layout::clamp_delta(&positions, (dx, dy), port::LIMIT);
    let moves: layout::Moves = selected
        .iter()
        .map(|n| (n.0, [n.1.x + dx, n.1.y + dy]))
        .collect();
    v.apply_moves(moves);
    true
}

fn alignment(v: &Rc<CanvasView>, key: gdk::Key, shift: bool) -> bool {
    let command = match (key.to_lower(), shift) {
        (gdk::Key::a, false) => Command::Align(Align::Left),
        (gdk::Key::d, false) => Command::Align(Align::Right),
        (gdk::Key::w, false) => Command::Align(Align::Top),
        (gdk::Key::s, false) => Command::Align(Align::Bottom),
        (gdk::Key::h, false) => Command::Align(Align::CenterX),
        (gdk::Key::v, false) => Command::Align(Align::CenterY),
        (gdk::Key::h, true) => Command::Distribute(Axis::Horizontal),
        (gdk::Key::v, true) => Command::Distribute(Axis::Vertical),
        (gdk::Key::t, false) => Command::Tidy,
        (gdk::Key::f, false) => Command::Flow,
        _ => return false,
    };
    run(v, command);
    true
}

#[derive(Clone, Copy)]
pub(in super::super) enum Command {
    Align(Align),
    Distribute(Axis),
    Tidy,
    Flow,
}

pub(in super::super) fn run(v: &Rc<CanvasView>, command: Command) {
    let selected = v.selected_nodes();
    let moves = match command {
        Command::Align(how) => layout::align(&selected, how),
        Command::Distribute(axis) => layout::distribute(&selected, axis),
        Command::Tidy => layout::tidy(&selected),
        Command::Flow => {
            // 선택이 없으면 보드 전체를 정리한다.
            let selected = if selected.is_empty() {
                v.session
                    .nodes
                    .borrow()
                    .iter()
                    .map(|n| (n.id, n.rect()))
                    .collect()
            } else {
                selected
            };
            let edges: Vec<_> = v
                .session
                .nodes
                .borrow()
                .iter()
                .flat_map(|n| n.next.iter().map(move |to| (n.id, *to)))
                .collect();
            layout::flow(&selected, &edges)
        }
    };
    v.apply_moves(moves);
}
