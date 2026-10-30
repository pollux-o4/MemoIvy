use super::super::{port, port::NodeView, scene::preview::Preview, CanvasView};
use crate::presentation::{icons, markdown};
use gtk::{
    prelude::*, Box as GtkBox, Button, Label, Orientation, Revealer, ScrolledWindow, Switch,
};
use std::{fmt::Write as _, rc::Rc};
use uuid::Uuid;

const PANEL_WIDTH: i32 = 252;
const PREVIEW_ROWS: i32 = 5;

pub(super) fn shell() -> (Revealer, GtkBox) {
    let inspector = GtkBox::new(Orientation::Vertical, 12);
    inspector.add_css_class("canvas-inspector");
    inspector.set_size_request(PANEL_WIDTH, -1);
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .min_content_width(PANEL_WIDTH)
        .max_content_width(PANEL_WIDTH)
        .child(&inspector)
        .build();
    scroll.add_css_class("floating");
    scroll.add_css_class("canvas-panel");
    scroll.set_overflow(gtk::Overflow::Hidden);
    let panel = Revealer::new();
    panel.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
    panel.set_transition_duration(220);
    panel.set_child(Some(&scroll));
    panel.set_halign(gtk::Align::End);
    panel.set_valign(gtk::Align::Start);
    panel.set_margin_top(16);
    panel.set_margin_end(16);
    panel.set_margin_bottom(16);
    (panel, inspector)
}

/// 내용이 바뀌었을 때만 패널을 다시 만든다. 저장 주기마다 포커스·호버가 끊기지 않는다.
pub(super) fn sync(v: &Rc<CanvasView>) {
    let id = v.session.selection.borrow().single();
    let idle = v.session.interaction.borrow().is_idle();
    v.widgets.panel.set_reveal_child(id.is_some() && idle);
    let Some(id) = id else {
        // 접히는 패널에 포커스를 남기면 키 입력이 숨겨진 위젯으로 간다.
        if focused_control(v).is_some() {
            gtk::prelude::GtkWindowExt::set_focus(&v.widgets.window, None::<&gtk::Widget>);
        }
        return;
    };
    let nodes = v.session.nodes.borrow().clone();
    let busy = v
        .state
        .upgrade()
        .is_some_and(|st| st.borrow().notebook.busy());
    v.widgets.inspector.set_sensitive(!busy);
    let key = fingerprint(id, &nodes);
    if *v.session.inspector_key.borrow() == key {
        return;
    }
    *v.session.inspector_key.borrow_mut() = key;
    let name = focused_control(v);
    if name.is_some() {
        gtk::prelude::GtkWindowExt::set_focus(&v.widgets.window, None::<&gtk::Widget>);
    }
    build(v, id, &nodes);
    if let Some(name) = name {
        if !restore_focus(v.widgets.inspector.upcast_ref(), &name) {
            restore_focus(v.widgets.inspector.upcast_ref(), "canvas-control-connect");
        }
    }
}

fn fingerprint(id: Uuid, nodes: &[NodeView]) -> String {
    let mut key = id.to_string();
    for node in nodes {
        let _ = write!(
            key,
            "|{}{}{}{}{}{:?}",
            node.id, node.title, node.preview, node.color, node.done, node.next
        );
        // 체크 상태는 요약 글자에 드러나지 않는다.
        if node.id == id {
            let _ = write!(key, "{:?}{:?}", node.title_marker, node.lines);
        }
    }
    key
}

fn focused_control(v: &CanvasView) -> Option<String> {
    let mut focused = gtk::prelude::GtkWindowExt::focus(&v.widgets.window);
    while let Some(widget) = focused {
        if widget.widget_name().starts_with("canvas-control-") {
            return Some(widget.widget_name().to_string());
        }
        focused = widget.parent();
    }
    None
}

fn restore_focus(widget: &gtk::Widget, name: &str) -> bool {
    crate::app::shared::find_named(widget, name).is_some_and(|widget| widget.grab_focus())
}

fn row_label(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_xalign(0.);
    label.set_hexpand(true);
    label
}

fn section(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_xalign(0.);
    label.add_css_class("panel-section");
    label
}

fn action_row(svg: &'static str, text: &str, name: &str) -> Button {
    let button = Button::new();
    button.add_css_class("panel-row");
    button.set_widget_name(name);
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&icons::image(svg));
    row.append(&row_label(text));
    button.set_child(Some(&row));
    button
}

fn dot(color: &str) -> GtkBox {
    let dot = GtkBox::new(Orientation::Horizontal, 0);
    dot.add_css_class("color-dot");
    dot.add_css_class(color);
    dot.set_valign(gtk::Align::Center);
    dot
}

fn build(v: &Rc<CanvasView>, id: Uuid, nodes: &[NodeView]) {
    v.widgets.connect_menu.borrow_mut().take();
    while let Some(child) = v.widgets.inspector.first_child() {
        v.widgets.inspector.remove(&child);
    }
    let Some(note) = nodes.iter().find(|n| n.id == id) else {
        return;
    };
    let head = GtkBox::new(Orientation::Horizontal, 8);
    head.append(&dot(&note.color));
    let title = Label::new(Some(note.display_title()));
    title.set_wrap(true);
    title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    title.set_lines(2);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_max_width_chars(1);
    title.set_hexpand(true);
    title.set_xalign(0.);
    title.add_css_class("panel-title");
    head.append(&title);
    let close = Button::new();
    close.set_child(Some(&icons::image(icons::CLOSE)));
    close.add_css_class("tool");
    close.add_css_class("small");
    close.add_css_class("panel-close");
    close.set_valign(gtk::Align::Start);
    close.set_tooltip_text(Some("선택 해제 · Esc"));
    close.update_property(&[gtk::accessible::Property::Label("선택 해제")]);
    let weak = Rc::downgrade(v);
    close.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.session.selection.borrow_mut().set([]);
            v.sync_selection();
        }
    });
    head.append(&close);
    v.widgets.inspector.append(&head);
    if !note.lines.is_empty() {
        let preview = Preview::new();
        preview.root.add_css_class("panel-muted");
        preview.set(&markdown::fit_preview(&note.lines, PREVIEW_ROWS).0);
        v.widgets.inspector.append(&preview.root);
    }

    let group = GtkBox::new(Orientation::Vertical, 0);
    group.add_css_class("panel-group");
    let open = action_row(icons::OPEN, "메모 열기", "canvas-control-open");
    let weak = Rc::downgrade(v);
    open.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.open_note(id);
        }
    });
    group.append(&open);
    let done_row = GtkBox::new(Orientation::Horizontal, 8);
    done_row.add_css_class("panel-row");
    done_row.add_css_class("static");
    done_row.set_tooltip_text(Some("완료 상태 변경 · Ctrl+Enter"));
    done_row.append(&icons::image(icons::CHECK));
    done_row.append(&row_label("완료"));
    let done = Switch::new();
    done.set_widget_name("canvas-control-done");
    done.set_valign(gtk::Align::Center);
    done.set_active(note.done);
    done.update_property(&[gtk::accessible::Property::Label("이 메모 완료")]);
    let weak = Rc::downgrade(v);
    done.connect_active_notify(move |switch| {
        if let Some(v) = weak.upgrade() {
            if v.node(id).is_some_and(|n| n.done != switch.is_active()) {
                v.toggle_done(id);
            }
        }
    });
    done_row.append(&done);
    group.append(&done_row);
    let replay = action_row(icons::SIGNAL, "신호 보내기", "canvas-control-replay");
    replay.set_sensitive(!note.next.is_empty());
    replay.set_tooltip_text(Some(
        "연결된 다음 메모로 신호를 보냅니다. 완료 상태는 바꾸지 않습니다.",
    ));
    let weak = Rc::downgrade(v);
    replay.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.signal(id);
        }
    });
    group.append(&replay);
    v.widgets.inspector.append(&group);

    v.widgets.inspector.append(&section("다음 메모"));
    let links = GtkBox::new(Orientation::Vertical, 0);
    links.add_css_class("panel-group");
    for next in &note.next {
        let Some(target) = nodes.iter().find(|n| n.id == *next) else {
            continue;
        };
        links.append(&link_row(v, id, target));
    }
    links.append(&connect_menu(v, note, nodes));
    v.widgets.inspector.append(&links);

    let incoming: Vec<&NodeView> = nodes.iter().filter(|n| n.next.contains(&id)).collect();
    if !incoming.is_empty() {
        v.widgets.inspector.append(&section("이전 메모"));
        let group = GtkBox::new(Orientation::Vertical, 0);
        group.add_css_class("panel-group");
        for source in incoming {
            group.append(&visit_button(v, source));
        }
        v.widgets.inspector.append(&group);
    }

    let group = GtkBox::new(Orientation::Vertical, 0);
    group.add_css_class("panel-group");
    let delete = action_row(icons::TRASH, "삭제…", "canvas-control-delete");
    delete.add_css_class("destructive");
    delete.set_tooltip_text(Some("이 메모 삭제 · Delete"));
    let weak = Rc::downgrade(v);
    delete.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.delete_selected();
        }
    });
    group.append(&delete);
    v.widgets.inspector.append(&group);
}

fn visit_button(v: &Rc<CanvasView>, target: &NodeView) -> Button {
    let button = Button::new();
    button.add_css_class("panel-row");
    button.set_hexpand(true);
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&dot(&target.color));
    let label = row_label(target.display_title());
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_max_width_chars(1);
    row.append(&label);
    button.set_child(Some(&row));
    button.set_tooltip_text(Some(target.display_title()));
    button.update_property(&[gtk::accessible::Property::Label(&format!(
        "메모 선택: {}",
        target.display_title()
    ))]);
    let weak = Rc::downgrade(v);
    let destination = target.id;
    button.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.select_only(destination);
            v.reveal();
        }
    });
    button
}

fn link_row(v: &Rc<CanvasView>, from: Uuid, target: &NodeView) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 0);
    row.add_css_class("panel-link");
    row.append(&visit_button(v, target));
    let remove = Button::new();
    remove.set_child(Some(&icons::image(icons::CLOSE)));
    remove.add_css_class("tool");
    remove.add_css_class("small");
    remove.set_widget_name(&format!("canvas-control-remove-{}", target.id));
    remove.set_tooltip_text(Some("이 연결 해제 · 메모는 유지"));
    remove.update_property(&[gtk::accessible::Property::Label(&format!(
        "{} 연결 해제",
        target.display_title()
    ))]);
    let weak = Rc::downgrade(v);
    let to = target.id;
    remove.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.mutate(|notes| port::disconnect(notes, from, to));
        }
    });
    row.append(&remove);
    row
}

fn connect_menu(v: &Rc<CanvasView>, note: &NodeView, nodes: &[NodeView]) -> gtk::Widget {
    let candidates: Vec<&NodeView> = nodes
        .iter()
        .filter(|n| n.id != note.id && !note.next.contains(&n.id))
        .collect();
    if candidates.is_empty() {
        let hint = Label::new(Some(if nodes.len() < 2 {
            "메모를 하나 더 만들면 연결할 수 있어요."
        } else {
            "모든 메모에 연결했어요."
        }));
        hint.set_wrap(true);
        hint.set_xalign(0.);
        hint.add_css_class("panel-hint");
        return hint.upcast();
    }
    let menu = gtk::MenuButton::new();
    menu.set_widget_name("canvas-control-connect");
    *v.widgets.connect_menu.borrow_mut() = Some(menu.clone());
    menu.add_css_class("panel-row");
    menu.set_tooltip_text(Some("다음 메모 연결 · Ctrl+L"));
    let contents = GtkBox::new(Orientation::Horizontal, 8);
    contents.append(&icons::image(icons::ADD));
    contents.append(&row_label("연결 추가"));
    menu.set_child(Some(&contents));
    menu.set_always_show_arrow(false);
    menu.update_property(&[gtk::accessible::Property::Label("연결할 다음 메모 선택")]);
    let popover = gtk::Popover::new();
    let list = GtkBox::new(Orientation::Vertical, 0);
    list.add_css_class("menu-body");
    for target in candidates {
        let row = Button::new();
        row.add_css_class("menu-row");
        row.add_css_class("two-line");
        row.set_widget_name(&format!("canvas-connect-{}", target.id));
        let line = GtkBox::new(Orientation::Horizontal, 8);
        line.append(&dot(&target.color));
        let text = GtkBox::new(Orientation::Vertical, 0);
        text.set_hexpand(true);
        let title = Label::new(Some(target.display_title()));
        title.set_xalign(0.);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.set_max_width_chars(26);
        text.append(&title);
        if !target.preview.is_empty() {
            let preview = Label::new(Some(&target.preview));
            preview.set_xalign(0.);
            preview.set_ellipsize(gtk::pango::EllipsizeMode::End);
            preview.set_max_width_chars(26);
            preview.add_css_class("menu-hint");
            text.append(&preview);
        }
        line.append(&text);
        row.set_child(Some(&line));
        let weak = Rc::downgrade(v);
        let (from, to) = (note.id, target.id);
        let close = popover.downgrade();
        row.connect_clicked(move |_| {
            if let Some(popover) = close.upgrade() {
                popover.popdown();
            }
            if let Some(v) = weak.upgrade() {
                v.connect_nodes(from, to);
            }
        });
        list.append(&row);
    }
    let scroll = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .propagate_natural_width(true)
        .max_content_height(280)
        .child(&list)
        .build();
    popover.set_child(Some(&scroll));
    crate::app::shared::menu_popover(&popover);
    menu.set_popover(Some(&popover));
    menu.upcast()
}
