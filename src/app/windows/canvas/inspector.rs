use super::CanvasView;
use gtk::{prelude::*, Box as GtkBox, Button, CheckButton, DropDown, Label, Orientation};
use std::rc::Rc;

pub(super) fn render(v: &Rc<CanvasView>) {
    let mut focused = gtk::prelude::GtkWindowExt::focus(&v.window);
    let mut name = None;
    while let Some(widget) = focused {
        if widget.widget_name().starts_with("canvas-control-") {
            name = Some(widget.widget_name().to_string());
            break;
        }
        focused = widget.parent();
    }
    if name.is_some() {
        gtk::prelude::GtkWindowExt::set_focus(&v.window, None::<&gtk::Widget>);
    }
    build(v);
    if let Some(name) = name {
        if !restore_focus(v.inspector.upcast_ref(), &name) {
            restore_focus(v.inspector.upcast_ref(), "canvas-control-connect");
        }
    }
}

fn restore_focus(widget: &gtk::Widget, name: &str) -> bool {
    if widget.widget_name() == name {
        return widget.grab_focus();
    }
    let mut child = widget.first_child();
    while let Some(w) = child {
        if restore_focus(&w, name) {
            return true;
        }
        child = w.next_sibling();
    }
    false
}

fn build(v: &Rc<CanvasView>) {
    while let Some(child) = v.inspector.first_child() {
        v.inspector.remove(&child);
    }
    let Some(st) = v.state.upgrade() else {
        return;
    };
    let s = st.borrow();
    let notes = s.store.notes.clone();
    let busy = s.operation.is_some();
    v.inspector.set_sensitive(!busy);
    drop(s);
    let heading = Label::new(Some("생각을 흐름으로"));
    heading.add_css_class("canvas-heading");
    heading.set_xalign(0.);
    v.inspector.append(&heading);
    let Some(note) = notes.iter().find(|n| Some(n.id) == v.selected.get()) else {
        let help = Label::new(Some(if notes.is_empty() {
            "새 메모로 시작하세요.\n메모 하나가 카드 하나로 연결됩니다."
        } else {
            "카드 위의 이동 핸들을 선택하세요.\n\n1. 드래그로 생각 배치\n2. 다음 메모 연결\n3. 완료하고 신호 보내기\n\n카드 본문을 누르면 원본 메모가 열립니다. 색상도 함께 바뀝니다."
        }));
        help.set_wrap(true);
        help.set_max_width_chars(22);
        help.set_xalign(0.);
        help.add_css_class("canvas-help");
        v.inspector.append(&help);
        return;
    };
    let id = note.id;
    let title = Label::new(Some(
        note.text
            .lines()
            .find(|s| !s.trim().is_empty())
            .unwrap_or("빈 메모"),
    ));
    title.set_wrap(true);
    title.set_max_width_chars(22);
    title.set_xalign(0.);
    title.add_css_class("card-title");
    v.inspector.append(&title);
    let open = Button::with_label("메모 열기 ↗");
    open.set_widget_name("canvas-control-open");
    let weak = Rc::downgrade(v);
    open.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            if let Some(st) = v.state.upgrade() {
                super::super::open_note(&st, &v.app, id);
            }
        }
    });
    v.inspector.append(&open);
    let done = CheckButton::with_label("이 메모 완료");
    done.set_widget_name("canvas-control-done");
    done.set_active(note.canvas.done);
    let weak = Rc::downgrade(v);
    done.connect_toggled(move |b| {
        if let Some(v) = weak.upgrade() {
            let active = b.is_active();
            v.mutate(|notes| {
                if let Some(n) = notes.iter_mut().find(|n| n.id == id) {
                    n.canvas.done = active;
                }
            });
            if active {
                v.signal(id);
            }
        }
    });
    v.inspector.append(&done);
    let replay = Button::with_label("신호 보내기 →");
    replay.set_widget_name("canvas-control-replay");
    replay.set_sensitive(!note.canvas.next.is_empty());
    replay.set_tooltip_text(Some(
        "다음 메모에 신호를 보냅니다. 완료 상태는 바꾸지 않습니다.",
    ));
    let weak = Rc::downgrade(v);
    replay.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.signal(id);
        }
    });
    v.inspector.append(&replay);
    v.inspector
        .append(&gtk::Separator::new(Orientation::Horizontal));
    let label = Label::new(Some("다음 메모"));
    label.set_xalign(0.);
    v.inspector.append(&label);
    let candidates: Vec<_> = notes
        .iter()
        .filter(|n| n.id != id && !note.canvas.next.contains(&n.id))
        .collect();
    if !candidates.is_empty() {
        let names: Vec<_> = candidates
            .iter()
            .enumerate()
            .map(|(i, n)| {
                format!(
                    "{}. {}",
                    i + 1,
                    n.text
                        .lines()
                        .find(|s| !s.trim().is_empty())
                        .unwrap_or("빈 메모")
                        .chars()
                        .take(16)
                        .collect::<String>()
                )
            })
            .collect();
        let dropdown =
            DropDown::from_strings(&names.iter().map(String::as_str).collect::<Vec<_>>());
        dropdown.set_widget_name("canvas-control-target");
        dropdown.update_property(&[gtk::accessible::Property::Label("연결할 다음 메모")]);
        dropdown.set_size_request(200, -1);
        v.inspector.append(&dropdown);
        let context = Label::new(None);
        context.set_wrap(true);
        context.set_max_width_chars(22);
        context.set_xalign(0.);
        let descriptions: Vec<String> = candidates.iter().map(|n| n.text.clone()).collect();
        context.set_text(&descriptions[0]);
        let detail = context.clone();
        dropdown.connect_selected_notify(move |d| {
            if let Some(text) = descriptions.get(d.selected() as usize) {
                detail.set_text(text);
            }
        });
        v.inspector.append(&context);
        let connect = Button::with_label("연결 추가");
        connect.set_widget_name("canvas-control-connect");
        let ids: Vec<_> = candidates.iter().map(|n| n.id).collect();
        let weak = Rc::downgrade(v);
        connect.connect_clicked(move |_| {
            if let (Some(v), Some(target)) = (weak.upgrade(), ids.get(dropdown.selected() as usize))
            {
                v.mutate(|notes| {
                    crate::persistence::canvas::connect(notes, id, *target);
                });
            }
        });
        v.inspector.append(&connect);
    } else {
        let hint = Label::new(Some(if notes.len() < 2 {
            "메모를 하나 더 만들면 연결할 수 있어요."
        } else {
            "모든 메모에 연결했어요."
        }));
        hint.set_wrap(true);
        hint.set_max_width_chars(22);
        hint.set_xalign(0.);
        hint.add_css_class("canvas-help");
        v.inspector.append(&hint);
    }
    for next in &note.canvas.next {
        let Some(target) = notes.iter().find(|n| n.id == *next) else {
            continue;
        };
        let row = GtkBox::new(Orientation::Horizontal, 4);
        let label = Label::new(Some(&format!(
            "→ {}",
            target
                .text
                .lines()
                .find(|s| !s.trim().is_empty())
                .unwrap_or("빈 메모")
        )));
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(16);
        label.set_hexpand(true);
        label.set_xalign(0.);
        let visit = Button::new();
        visit.set_child(Some(&label));
        visit.set_tooltip_text(Some("다음 메모 선택"));
        visit.update_property(&[gtk::accessible::Property::Label(&format!(
            "다음 메모 선택: {}",
            target
                .text
                .lines()
                .find(|s| !s.trim().is_empty())
                .unwrap_or("빈 메모")
        ))]);
        let weak = Rc::downgrade(v);
        let destination = *next;
        visit.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                v.select(destination);
                v.reveal();
            }
        });
        row.append(&visit);
        let remove = Button::with_label("×");
        remove.set_widget_name(&format!("canvas-control-remove-{next}"));
        remove.set_tooltip_text(Some("이 연결 해제 · 메모는 유지"));
        remove.update_property(&[gtk::accessible::Property::Label(&format!(
            "{} 연결 해제",
            target.text.lines().next().unwrap_or("빈 메모")
        ))]);
        let next = *next;
        let weak = Rc::downgrade(v);
        remove.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                v.mutate(|notes| {
                    if let Some(n) = notes.iter_mut().find(|n| n.id == id) {
                        n.canvas.next.retain(|id| *id != next);
                    }
                });
            }
        });
        row.append(&remove);
        v.inspector.append(&row);
    }
    let hint = Label::new(Some(
        "완료하면 연결된 다음 메모로 신호가 흐릅니다. 다음 메모의 완료 여부는 직접 결정하세요.",
    ));
    hint.set_wrap(true);
    hint.set_max_width_chars(22);
    hint.set_xalign(0.);
    hint.add_css_class("canvas-help");
    v.inspector.append(&hint);
}
