use crate::app::shell::{commands, state::State};
use crate::presentation::icons;
use gtk::{prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};
use uuid::Uuid;

/// 모든 팝오버의 공통 모양과 등장 모션. 시작 클래스를 다음 프레임에 지워 CSS 전환을 일으킨다.
pub(in crate::app) fn menu_popover(popover: &gtk::Popover) {
    popover.set_has_arrow(false);
    popover.add_css_class("lsm-menu");
    popover.connect_closed(|popover| {
        if let Some(parent) = popover.parent() {
            parent.set_has_tooltip(true);
        }
    });
    popover.connect_show(|popover| {
        // 메뉴가 열린 동안 버튼 툴팁이 항목을 가리지 않게 한다.
        if let Some(parent) = popover.parent() {
            parent.set_has_tooltip(false);
        }
        popover.add_css_class("opening");
        popover.add_tick_callback(|widget, _| {
            widget.remove_css_class("opening");
            gtk::glib::ControlFlow::Break
        });
    });
}

fn row_contents(icon: &'static str, label: &str, hint: Option<&str>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.append(&icons::image(icon));
    let text = Label::new(Some(label));
    text.set_xalign(0.);
    text.set_hexpand(true);
    row.append(&text);
    if let Some(hint) = hint {
        let hint = Label::new(Some(hint));
        hint.add_css_class("menu-hint");
        row.append(&hint);
    }
    row
}

pub(in crate::app) fn menu_row(icon: &'static str, label: &str, hint: Option<&str>) -> Button {
    let button = Button::new();
    button.add_css_class("menu-row");
    button.set_child(Some(&row_contents(icon, label, hint)));
    button
}

/// 편집기의 원문 토글을 메뉴 항목 모양으로 맞춘다. 켜짐은 왼쪽 체크로 보여 준다.
pub(in crate::app) fn source_row(toggle: &gtk::CheckButton) {
    toggle.add_css_class("menu-row");
    toggle.set_child(Some(&row_contents(
        icons::SOURCE,
        "원문 보기",
        Some("Ctrl+Shift+M"),
    )));
    toggle.set_tooltip_text(None);
}

/// NOTE-002 삭제 확인. 기본 동작은 취소이며 Esc·닫기도 취소다.
/// `name`을 주면 어떤 메모를 지우는지 함께 보여 준다.
pub(in crate::app) fn confirm_delete(
    parent: &ApplicationWindow,
    state: &State,
    ids: Vec<Uuid>,
    name: Option<String>,
) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("메모 삭제")
        .default_width(340)
        .resizable(false)
        .build();
    dialog.add_css_class("lsm-confirm");
    // 시스템 제목 표시줄 없이 내용만 보인다.
    dialog.set_titlebar(Some(&GtkBox::new(Orientation::Horizontal, 0)));
    let body = GtkBox::new(Orientation::Vertical, 6);
    body.add_css_class("confirm-body");
    let question = match ids.len() {
        1 => "이 메모를 삭제할까요?".to_owned(),
        count => format!("메모 {count}개를 삭제할까요?"),
    };
    let title = Label::new(Some(&question));
    title.set_xalign(0.);
    title.add_css_class("confirm-title");
    body.append(&title);
    if let Some(name) = name {
        let subject = Label::new(Some(&name));
        subject.set_xalign(0.);
        subject.set_ellipsize(gtk::pango::EllipsizeMode::End);
        subject.set_max_width_chars(36);
        subject.add_css_class("confirm-subject");
        body.append(&subject);
    }
    let detail = Label::new(Some(if ids.len() > 1 {
        "삭제한 메모는 복구할 수 없습니다. 내용과 연결이 모두 사라져요."
    } else {
        "삭제한 메모는 복구할 수 없습니다."
    }));
    detail.set_wrap(true);
    detail.set_xalign(0.);
    detail.add_css_class("confirm-detail");
    body.append(&detail);
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_homogeneous(true);
    actions.set_margin_top(14);
    let cancel = Button::with_label("취소");
    cancel.add_css_class("confirm-cancel");
    let accept = Button::with_label("삭제");
    accept.add_css_class("confirm-accept");
    actions.append(&cancel);
    actions.append(&accept);
    body.append(&actions);
    dialog.set_child(Some(&body));
    dialog.set_default_widget(Some(&cancel));
    let weak = dialog.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    let st = state.clone();
    let weak = dialog.downgrade();
    accept.connect_clicked(move |_| {
        commands::delete_notes(&st, ids.clone());
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    super::close_on_escape(&dialog);
    dialog.present();
    cancel.grab_focus();
}

fn popover_for(state: &State, _app: &Application, id: Uuid) -> gtk::Popover {
    let pop = gtk::Popover::new();
    let body = GtkBox::new(Orientation::Vertical, 0);
    body.add_css_class("menu-body");
    let delete = menu_row(icons::TRASH, "삭제…", None);
    delete.add_css_class("destructive");
    let st = state.clone();
    let close = pop.downgrade();
    delete.connect_clicked(move |b| {
        let Some(parent) = b.root().and_downcast::<ApplicationWindow>() else {
            return;
        };
        if let Some(popover) = close.upgrade() {
            popover.popdown();
        }
        confirm_delete(&parent, &st, vec![id], None);
    });
    body.append(&delete);
    pop.set_child(Some(&body));
    menu_popover(&pop);
    pop
}

fn more_button() -> gtk::MenuButton {
    let menu = gtk::MenuButton::new();
    menu.set_child(Some(&icons::image(icons::MORE)));
    menu.update_property(&[gtk::accessible::Property::Label("더 보기")]);
    menu.set_tooltip_text(Some("더 보기"));
    menu.add_css_class("icon-button");
    menu
}

pub(in crate::app) fn menu_for(state: &State, app: &Application, id: Uuid) -> gtk::MenuButton {
    let menu = more_button();
    menu.set_popover(Some(&popover_for(state, app, id)));
    menu
}

/// 메뉴를 처음 열 때 만든다. 긴 목록의 항목마다 팝오버를 미리 두지 않는다.
pub(in crate::app) fn lazy_menu_for(state: &State, app: &Application, id: Uuid) -> gtk::MenuButton {
    let menu = more_button();
    let state = state.clone();
    let app = app.clone();
    menu.set_create_popup_func(move |menu| {
        if menu.popover().is_none() {
            menu.set_popover(Some(&popover_for(&state, &app, id)));
        }
    });
    menu
}

/// 복구 안내 등 확인 응답만 필요한 공용 창.
pub(in crate::app) fn message(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .default_width(380)
        .build();
    let body = GtkBox::new(Orientation::Vertical, 16);
    body.set_margin_start(20);
    body.set_margin_end(20);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    let label = Label::new(Some(message));
    label.set_wrap(true);
    body.append(&label);
    let close = Button::with_label("닫기");
    body.append(&close);
    dialog.set_child(Some(&body));
    let weak = dialog.downgrade();
    close.connect_clicked(move |_| {
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    super::close_on_escape(&dialog);
    dialog.present();
}
