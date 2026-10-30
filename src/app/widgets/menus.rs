use crate::app::state::{Operation, State};
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
fn confirm_delete(parent: &ApplicationWindow, state: &State, id: Uuid) {
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
    let title = Label::new(Some("이 메모를 삭제할까요?"));
    title.set_xalign(0.);
    title.add_css_class("confirm-title");
    body.append(&title);
    let detail = Label::new(Some("삭제한 메모는 복구할 수 없습니다."));
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
        let mut s = st.borrow_mut();
        if s.operation.is_none() {
            if let Some(editor) = s.windows.get(&id) {
                editor.view.set_editable(false);
            }
            s.operation = Some((Operation::Delete(id), false));
            s.add.set_sensitive(false);
        }
        drop(s);
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    let keys = gtk::EventControllerKey::new();
    let weak = dialog.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    dialog.add_controller(keys);
    dialog.present();
    cancel.grab_focus();
}

fn popover_for(state: &State, app: &Application, id: Uuid) -> gtk::Popover {
    let pop = gtk::Popover::new();
    let body = GtkBox::new(Orientation::Vertical, 0);
    body.add_css_class("menu-body");
    let delete = menu_row(icons::TRASH, "삭제…", None);
    delete.add_css_class("destructive");
    let st = state.clone();
    let _a = app.clone();
    let close = pop.downgrade();
    delete.connect_clicked(move |b| {
        let Some(parent) = b.root().and_downcast::<ApplicationWindow>() else {
            return;
        };
        if let Some(popover) = close.upgrade() {
            popover.popdown();
        }
        confirm_delete(&parent, &st, id);
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
