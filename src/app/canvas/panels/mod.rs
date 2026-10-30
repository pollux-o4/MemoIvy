//! 보드 위에 떠 있는 도구. 조립·배선(build·wire)과 상태 반영(sync)을 분리한다.
mod inspector;
mod toolbar;

use super::{input::keyboard::Command, CanvasView};
use crate::{app::shell::state::State, presentation::icons};
use gtk::{prelude::*, Box as GtkBox, Button, Label, Orientation, Revealer};
use std::rc::Rc;

const COMPACT_WIDTH: i32 = 760;

pub(super) struct Actions {
    pub undo: Button,
    pub redo: Button,
    pub zoom_out: Button,
    pub zoom_in: Button,
    pub fit: Button,
    pub delete: Button,
    pub commands: Vec<(Button, Command)>,
    pub empty_add: Button,
}

pub(super) struct Floating {
    pub dock_row: GtkBox,
    pub dock: GtkBox,
    pub undo: Button,
    pub redo: Button,
    pub zoom_label: Button,
    pub summary: Label,
    pub status: Label,
    pub retry: Button,
    pub align_bar: Revealer,
    pub align_count: Label,
    pub distribute: [Button; 2],
    pub panel: Revealer,
    pub inspector: GtkBox,
    pub empty: GtkBox,
    pub actions: Actions,
}

pub(super) fn build(state: &State) -> Floating {
    let dock = toolbar::dock(state);
    let align = toolbar::align_bar();
    let (panel, inspector) = inspector::shell();
    let (empty, empty_add) = empty_state();
    Floating {
        dock_row: dock.row,
        dock: dock.root,
        undo: dock.undo.clone(),
        redo: dock.redo.clone(),
        zoom_label: dock.zoom_label,
        summary: dock.summary,
        status: dock.status,
        retry: dock.retry,
        align_bar: align.root,
        align_count: align.count,
        distribute: align.distribute,
        panel,
        inspector,
        empty,
        actions: Actions {
            undo: dock.undo,
            redo: dock.redo,
            zoom_out: dock.zoom_out,
            zoom_in: dock.zoom_in,
            fit: dock.fit,
            delete: align.delete,
            commands: align.commands,
            empty_add,
        },
    }
}

fn empty_state() -> (GtkBox, Button) {
    let empty = GtkBox::new(Orientation::Vertical, 8);
    empty.add_css_class("canvas-empty");
    empty.set_halign(gtk::Align::Center);
    empty.set_valign(gtk::Align::Center);
    empty.append(&icons::illustration(icons::CANVAS_EMPTY, 168, 112));
    let title = Label::new(Some("아직 메모가 없어요"));
    title.add_css_class("canvas-empty-title");
    title.set_margin_top(12);
    empty.append(&title);
    let sub = Label::new(Some(
        "메모를 만들면 카드로 나타나고, 끌어서 배치하고 연결할 수 있어요.",
    ));
    sub.add_css_class("canvas-empty-sub");
    empty.append(&sub);
    let add = Button::new();
    let contents = GtkBox::new(Orientation::Horizontal, 4);
    contents.append(&icons::image(icons::ADD));
    contents.append(&Label::new(Some("첫 메모 만들기")));
    add.set_child(Some(&contents));
    add.add_css_class("new-note");
    add.set_halign(gtk::Align::Center);
    add.set_margin_top(12);
    empty.append(&add);
    empty.set_visible(false);
    (empty, add)
}

pub(super) fn wire(v: &Rc<CanvasView>, actions: Actions) {
    toolbar::wire(v, actions);
    let weak = Rc::downgrade(v);
    v.widgets.retry.connect_clicked(move |_| {
        if let Some(st) = weak.upgrade().and_then(|v| v.state.upgrade()) {
            crate::app::shell::lifecycle::flush(&st);
        }
    });
}

/// 선택과 메모 수에 맞춰 떠 있는 도구의 표시 상태를 맞춘다.
pub(super) fn sync(v: &Rc<CanvasView>) {
    let nodes = v.session.nodes.borrow();
    let edges: usize = nodes.iter().map(|n| n.next.len()).sum();
    v.widgets
        .summary
        .set_text(&format!("메모 {} · 연결 {}", nodes.len(), edges));
    v.widgets.empty.set_visible(nodes.is_empty());
    let count = nodes.len();
    drop(nodes);
    let selected = v.session.selection.borrow().len();
    v.widgets.align_bar.set_reveal_child(selected >= 2);
    if selected >= 2 {
        v.widgets
            .align_count
            .set_text(&format!("{selected}개 선택"));
    }
    for button in &v.widgets.distribute {
        button.set_sensitive(selected >= 3);
    }
    v.widgets
        .dock
        .set_visible(count > 0 || v.widgets.retry.is_visible());
    inspector::sync(v);
    v.widgets
        .summary
        .set_visible(v.widgets.surface.width() == 0 || v.widgets.surface.width() >= COMPACT_WIDTH);
}
