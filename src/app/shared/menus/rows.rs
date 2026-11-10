use crate::presentation::icons;
use gtk::{prelude::*, Box as GtkBox, Button, Label, Orientation};

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

pub(super) fn more_button() -> gtk::MenuButton {
    let menu = gtk::MenuButton::new();
    menu.set_child(Some(&icons::image(icons::MORE)));
    menu.update_property(&[gtk::accessible::Property::Label("더 보기")]);
    menu.set_tooltip_text(Some("더 보기"));
    menu.add_css_class("icon-button");
    menu
}
