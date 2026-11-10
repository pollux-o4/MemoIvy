use super::super::{
    input::keyboard::{self, Command},
    model::layout::{Align, Axis},
    motion, CanvasView,
};
use super::Actions;
use crate::{app::shell::state::State, presentation::icons};
use gtk::{prelude::*, Box as GtkBox, Button, Label, Orientation, Revealer};
use std::rc::Rc;

pub(super) struct Dock {
    pub row: GtkBox,
    pub root: GtkBox,
    pub undo: Button,
    pub redo: Button,
    pub zoom_out: Button,
    pub zoom_in: Button,
    pub zoom_label: Button,
    pub fit: Button,
    pub summary: Label,
    pub status: Label,
    pub retry: Button,
}

fn icon_button(svg: &'static str, tip: &str) -> Button {
    let button = Button::new();
    button.set_child(Some(&icons::image(svg)));
    button.add_css_class("tool");
    button.set_tooltip_text(Some(tip));
    let name = tip.split(" · ").next().unwrap_or(tip);
    button.update_property(&[gtk::accessible::Property::Label(name)]);
    button
}

fn divider() -> gtk::Separator {
    let line = gtk::Separator::new(Orientation::Vertical);
    line.add_css_class("tool-divider");
    line
}

pub(super) fn dock(_state: &State) -> Dock {
    let root = GtkBox::new(Orientation::Horizontal, 2);
    root.add_css_class("canvas-dock");
    root.add_css_class("floating");
    let undo = icon_button(icons::UNDO, "되돌리기 · Ctrl+Z");
    let redo = icon_button(icons::REDO, "다시 실행 · Ctrl+Shift+Z");
    let zoom_out = icon_button(icons::ZOOM_OUT, "축소 · Ctrl+-");
    let zoom_label = Button::with_label("100%");
    zoom_label.add_css_class("tool");
    zoom_label.add_css_class("zoom-value");
    zoom_label.set_tooltip_text(Some("100%로 보기 · Ctrl+0"));
    let zoom_in = icon_button(icons::ZOOM_IN, "확대 · Ctrl++");
    let fit = icon_button(icons::FIT, "전체 보기 · Shift+1");
    let help = gtk::MenuButton::new();
    help.set_child(Some(&icons::image(icons::HELP)));
    help.add_css_class("tool");
    help.set_tooltip_text(Some("캔버스 사용법"));
    help.update_property(&[gtk::accessible::Property::Label("캔버스 사용법")]);
    help.set_popover(Some(&help_popover()));
    for widget in [undo.upcast_ref::<gtk::Widget>(), redo.upcast_ref()] {
        root.append(widget);
    }
    root.append(&divider());
    for widget in [
        zoom_out.upcast_ref::<gtk::Widget>(),
        zoom_label.upcast_ref(),
        zoom_in.upcast_ref(),
        fit.upcast_ref(),
    ] {
        root.append(widget);
    }
    root.append(&divider());
    root.append(&help);
    let summary = Label::new(None);
    summary.add_css_class("canvas-summary");
    summary.set_margin_start(6);
    root.append(&summary);
    let status = Label::new(None);
    status.add_css_class("canvas-status");
    status.set_margin_start(6);
    status.set_margin_end(8);
    root.append(&status);
    let retry = Button::with_label("다시 시도");
    retry.add_css_class("tool-text");
    retry.set_visible(false);
    root.append(&retry);
    let row = GtkBox::new(Orientation::Horizontal, 0);
    row.set_halign(gtk::Align::Start);
    row.set_valign(gtk::Align::End);
    row.set_margin_start(16);
    row.set_margin_bottom(16);
    row.append(&root);
    Dock {
        row,
        root,
        undo,
        redo,
        zoom_out,
        zoom_in,
        zoom_label,
        fit,
        summary,
        status,
        retry,
    }
}

fn help_popover() -> gtk::Popover {
    let popover = gtk::Popover::new();
    let body = GtkBox::new(Orientation::Vertical, 6);
    body.add_css_class("menu-body");
    body.add_css_class("help-body");
    for (gesture, meaning) in [
        ("끌기", "카드 이동 · 빈 곳은 범위 선택"),
        ("두 번 누르기 · Enter", "메모 열기 · 프로젝트는 들어가기"),
        ("프로젝트 카드에 끌어 놓기", "그 프로젝트로 옮기기"),
        ("Alt+G", "선택을 새 프로젝트로 묶기"),
        ("Esc · Alt+↑", "프로젝트 밖으로 나가기"),
        ("Shift + 누르기", "선택에 더하거나 빼기"),
        ("오른쪽 점 끌기", "다음 메모에 연결"),
        ("스크롤 · Space + 끌기", "보드 이동"),
        ("Shift + 휠", "좌우 이동"),
        ("Ctrl + 스크롤", "포인터 기준 확대"),
        ("Ctrl (끄는 중)", "맞춤선 끄기"),
        ("F1", "모든 단축키"),
    ] {
        let row = GtkBox::new(Orientation::Horizontal, 16);
        let key = Label::new(Some(gesture));
        key.set_xalign(0.);
        key.set_hexpand(true);
        key.add_css_class("help-key");
        let what = Label::new(Some(meaning));
        what.set_xalign(1.);
        what.add_css_class("menu-hint");
        row.append(&key);
        row.append(&what);
        body.append(&row);
    }
    popover.set_child(Some(&body));
    crate::app::shared::menu_popover(&popover);
    popover
}

pub(super) struct AlignBar {
    pub root: Revealer,
    pub count: Label,
    pub distribute: [Button; 2],
    pub delete: Button,
    pub group: Button,
    pub lift: Button,
    pub commands: Vec<(Button, Command)>,
}

pub(super) fn align_bar() -> AlignBar {
    let bar = GtkBox::new(Orientation::Horizontal, 2);
    bar.add_css_class("canvas-align");
    bar.add_css_class("floating");
    let count = Label::new(None);
    count.add_css_class("canvas-summary");
    count.set_margin_start(8);
    count.set_margin_end(6);
    bar.append(&count);
    bar.append(&divider());
    let mut commands = Vec::new();
    for (svg, tip, command) in [
        (
            icons::ALIGN_LEFT,
            "왼쪽 맞춤 · Alt+A",
            Command::Align(Align::Left),
        ),
        (
            icons::ALIGN_CENTER_X,
            "가로 가운데 맞춤 · Alt+H",
            Command::Align(Align::CenterX),
        ),
        (
            icons::ALIGN_RIGHT,
            "오른쪽 맞춤 · Alt+D",
            Command::Align(Align::Right),
        ),
        (
            icons::ALIGN_TOP,
            "위쪽 맞춤 · Alt+W",
            Command::Align(Align::Top),
        ),
        (
            icons::ALIGN_CENTER_Y,
            "세로 가운데 맞춤 · Alt+V",
            Command::Align(Align::CenterY),
        ),
        (
            icons::ALIGN_BOTTOM,
            "아래쪽 맞춤 · Alt+S",
            Command::Align(Align::Bottom),
        ),
    ] {
        let button = icon_button(svg, tip);
        bar.append(&button);
        commands.push((button, command));
    }
    bar.append(&divider());
    let across = icon_button(icons::DISTRIBUTE_X, "가로 간격 같게 · Alt+Shift+H");
    let down = icon_button(icons::DISTRIBUTE_Y, "세로 간격 같게 · Alt+Shift+V");
    bar.append(&across);
    bar.append(&down);
    commands.push((across.clone(), Command::Distribute(Axis::Horizontal)));
    commands.push((down.clone(), Command::Distribute(Axis::Vertical)));
    bar.append(&divider());
    for (svg, tip, command) in [
        (icons::TIDY, "격자로 정리 · Alt+T", Command::Tidy),
        (icons::FLOW, "연결 순서로 배치 · Alt+F", Command::Flow),
    ] {
        let button = icon_button(svg, tip);
        bar.append(&button);
        commands.push((button, command));
    }
    bar.append(&divider());
    let group = icon_button(icons::PROJECT, "새 프로젝트로 묶기 · Alt+G");
    let lift = icon_button(icons::UP, "프로젝트 밖으로 꺼내기 · Alt+Shift+G");
    bar.append(&group);
    bar.append(&lift);
    bar.append(&divider());
    let delete = icon_button(icons::TRASH, "선택한 메모 삭제 · Delete");
    delete.add_css_class("danger");
    bar.append(&delete);
    let root = Revealer::new();
    root.set_transition_type(gtk::RevealerTransitionType::Crossfade);
    root.set_transition_duration(160);
    root.set_child(Some(&bar));
    root.set_halign(gtk::Align::Center);
    root.set_valign(gtk::Align::Start);
    root.set_margin_top(16);
    AlignBar {
        root,
        count,
        distribute: [across, down],
        delete,
        group,
        lift,
        commands,
    }
}

pub(super) fn wire(v: &Rc<CanvasView>, actions: Actions) {
    let on = |button: &Button, action: fn(&Rc<CanvasView>)| {
        let weak = Rc::downgrade(v);
        button.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                action(&v);
            }
        });
    };
    on(&actions.delete, |v| v.delete_selected());
    on(&actions.group, |v| v.group_selected());
    on(&actions.lift, |v| v.lift_selected());
    on(&actions.undo, |v| v.undo());
    on(&actions.redo, |v| v.redo());
    on(&actions.zoom_out, |v| {
        motion::zoom_by(v, 1. / motion::ZOOM_STEP)
    });
    on(&actions.zoom_in, |v| motion::zoom_by(v, motion::ZOOM_STEP));
    on(&actions.fit, |v| v.fit());
    on(&v.widgets.zoom_label, |v| {
        motion::reset_zoom(v);
    });
    on(&actions.empty_add, |v| {
        if let Some(st) = v.state.upgrade() {
            crate::app::shell::commands::new_note(&st, v.session.scope.get());
        }
    });
    for (button, command) in actions.commands {
        let weak = Rc::downgrade(v);
        button.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                keyboard::run(&v, command);
            }
        });
    }
}
