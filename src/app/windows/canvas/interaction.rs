use super::CanvasView;
use crate::{
    app::controller,
    persistence::{
        canvas::{position, HEIGHT, LIMIT, WIDTH},
        store::Note,
    },
};
use gtk::{gdk, glib, prelude::*, Box as GtkBox, Button, GestureDrag, Label, Orientation};
use std::{cell::Cell, rc::Rc};

pub(super) fn card(v: &Rc<CanvasView>, note: &Note, index: usize) -> GtkBox {
    let card = GtkBox::new(Orientation::Vertical, 0);
    card.add_css_class("canvas-card");
    card.add_css_class(&note.color);
    card.set_widget_name(&note.id.to_string());
    if v.selected.get() == Some(note.id) {
        card.add_css_class("selected");
    }
    let id = note.id;
    let handle = Button::with_label(if note.canvas.done {
        "✓ 완료  ·  드래그로 이동"
    } else {
        "⠿  드래그로 이동"
    });
    handle.add_css_class("canvas-handle");
    let weak = Rc::downgrade(v);
    let focus = gtk::EventControllerFocus::new();
    focus.connect_enter(move |_| {
        if let Some(v) = weak.upgrade() {
            if v.selected.get() != Some(id) {
                v.select(id);
            }
        }
    });
    handle.add_controller(focus);
    handle.update_property(&[gtk::accessible::Property::Label(&format!(
        "{} · 선택 및 이동",
        note.text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("빈 메모")
    ))]);
    handle.set_tooltip_text(Some("선택 · 드래그 또는 방향키로 이동 (Shift: 크게 이동)"));
    let weak = Rc::downgrade(v);
    handle.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.select(id);
        }
    });
    let drag = GestureDrag::new();
    drag.set_propagation_phase(gtk::PropagationPhase::Capture);
    let origin = Rc::new(Cell::new(position(note, index)));
    let pointer = Rc::new(Cell::new((0., 0.)));
    let weak = Rc::downgrade(v);
    let start = origin.clone();
    let pointer_start = pointer.clone();
    drag.connect_drag_begin(move |gesture, _, _| {
        if let Some(v) = weak.upgrade() {
            if let Some(st) = v.state.upgrade() {
                if st.borrow().operation.is_some() {
                    gesture.set_state(gtk::EventSequenceState::Denied);
                    return;
                }
                if let Some((i, n)) = st
                    .borrow()
                    .store
                    .notes
                    .iter()
                    .enumerate()
                    .find(|(_, n)| n.id == id)
                {
                    start.set(position(n, i));
                }
            }
            if let Some(p) = gesture.current_event().and_then(|e| e.position()) {
                pointer_start.set(p);
            }
            gesture.set_state(gtk::EventSequenceState::Claimed);
            if let Some(widget) = gesture.widget() {
                widget.grab_focus();
            }
            v.zoom_transition.set(None);
            v.checkpoint();
            v.dragging.set(true);
            v.select(id);
        }
    });
    let weak = Rc::downgrade(v);
    let widget = card.downgrade();
    let moving_origin = origin.clone();
    drag.connect_drag_update(move |gesture, dx, dy| {
        let (dx, dy) = gesture
            .current_event()
            .and_then(|e| e.position())
            .map(|p| (p.0 - pointer.get().0, p.1 - pointer.get().1))
            .unwrap_or((dx, dy));
        if dx.hypot(dy) > 3. {
            gesture.set_state(gtk::EventSequenceState::Claimed);
        }
        let (Some(v), Some(card)) = (weak.upgrade(), widget.upgrade()) else {
            return;
        };
        if !v.dragging.get() {
            return;
        }
        let z = v.zoom.get();
        let [x, y] = moving_origin.get();
        let p = [(x + dx / z).clamp(0., LIMIT), (y + dy / z).clamp(0., LIMIT)];
        if let Some(st) = v.state.upgrade() {
            if let Some(n) = st.borrow_mut().store.notes.iter_mut().find(|n| n.id == id) {
                n.canvas.position = Some(p);
            }
        }
        super::board::place(&v.fixed, &card, p[0], p[1], z);
        v.drawing.queue_draw();
    });
    let weak = Rc::downgrade(v);
    let end_origin = origin.clone();
    drag.connect_drag_end(move |_, _, _| {
        if let Some(v) = weak.upgrade() {
            if v.dragging.replace(false) {
                let moved = v.state.upgrade().is_some_and(|st| {
                    st.borrow()
                        .store
                        .notes
                        .iter()
                        .find(|n| n.id == id)
                        .is_some_and(|n| n.canvas.position != Some(end_origin.get()))
                });
                if moved {
                    if let Some(st) = v.state.upgrade() {
                        controller::mark(&st);
                    }
                    v.defer_render();
                } else {
                    v.history.borrow_mut().pop();
                }
            }
        }
    });
    let weak = Rc::downgrade(v);
    drag.connect_cancel(move |_, _| {
        if let Some(v) = weak.upgrade() {
            if v.dragging.replace(false) {
                if let Some(st) = v.state.upgrade() {
                    if let Some(n) = st.borrow_mut().store.notes.iter_mut().find(|n| n.id == id) {
                        n.canvas.position = Some(origin.get());
                    }
                }
                v.history.borrow_mut().pop();
                v.defer_render();
            }
        }
    });
    handle.add_controller(drag);
    let keys = gtk::EventControllerKey::new();
    let weak = Rc::downgrade(v);
    keys.connect_key_pressed(move |_, key, _, mods| {
        let (dx, dy) = match key {
            gdk::Key::Left => (-1., 0.),
            gdk::Key::Right => (1., 0.),
            gdk::Key::Up => (0., -1.),
            gdk::Key::Down => (0., 1.),
            _ => return glib::Propagation::Proceed,
        };
        if let Some(v) = weak.upgrade() {
            let step = if mods.contains(gdk::ModifierType::SHIFT_MASK) {
                80.
            } else {
                16.
            };
            v.mutate(|notes| {
                if let Some((i, n)) = notes.iter_mut().enumerate().find(|(_, n)| n.id == id) {
                    let [x, y] = position(n, i);
                    n.canvas.position = Some([
                        (x + dx * step).clamp(0., LIMIT),
                        (y + dy * step).clamp(0., LIMIT),
                    ]);
                }
            });
            if let Some(card) = v.fixed.first_child() {
                let mut next = Some(card);
                while let Some(c) = next {
                    if c.widget_name() == id.to_string() {
                        c.child_focus(gtk::DirectionType::TabForward);
                        break;
                    }
                    next = c.next_sibling();
                }
            }
        }
        glib::Propagation::Stop
    });
    handle.add_controller(keys);
    card.append(&handle);
    let text = GtkBox::new(Orientation::Vertical, 8);
    let title = Label::new(Some(
        note.text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("빈 메모"),
    ));
    title.set_xalign(0.);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.add_css_class("card-title");
    text.append(&title);
    let preview = Label::new(Some(
        &note.text.lines().skip(1).collect::<Vec<_>>().join(" "),
    ));
    preview.set_xalign(0.);
    preview.set_ellipsize(gtk::pango::EllipsizeMode::End);
    preview.add_css_class("preview");
    text.append(&preview);
    let detail = Label::new(Some(&format!(
        "{}   ·   → {}",
        if note.canvas.done {
            "완료"
        } else {
            "진행 전"
        },
        note.canvas.next.len()
    )));
    detail.set_xalign(0.);
    detail.add_css_class("preview");
    text.append(&detail);
    let open = Button::new();
    open.add_css_class("canvas-open");
    open.set_child(Some(&text));
    open.set_tooltip_text(Some("원본 메모 열기 · 내용과 색상이 함께 바뀝니다"));
    let weak = Rc::downgrade(v);
    open.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            if let Some(st) = v.state.upgrade() {
                super::super::open_note(&st, &v.app, id);
            }
        }
    });
    card.append(&open);
    card.set_size_request(WIDTH as i32, HEIGHT as i32);
    card
}
