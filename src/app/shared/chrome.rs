use crate::presentation::icons;
use gtk::{prelude::*, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};

pub(in crate::app) fn canvas_button(label: &str) -> Button {
    let button = Button::new();
    button.set_child(Some(&icons::image(icons::FLOW)));
    button.add_css_class("icon-button");
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button
}

// shortcut: 창 위치 복원·항상 위 고정은 대상 세션 지원을 미확인, 해당 기능 요청과 대상 세션 확정 시 검토.
pub(in crate::app) fn header(window: &ApplicationWindow, title: &str) -> (GtkBox, GtkBox) {
    window.add_css_class("memoivy-window");
    window.connect_notify_local(Some("maximized"), |w, _| {
        if w.is_maximized() {
            w.add_css_class("maximized");
        } else {
            w.remove_css_class("maximized");
        }
    });
    let bar = GtkBox::new(Orientation::Horizontal, 0);
    bar.add_css_class("header");
    let controls = GtkBox::new(Orientation::Horizontal, 0);
    for (symbol, class, tip) in [
        (icons::CLOSE, "close", "닫기"),
        (icons::MINIMIZE, "min", "최소화"),
        (icons::MAXIMIZE, "max", "최대화"),
    ] {
        let b = Button::new();
        b.set_child(Some(&icons::image(symbol)));
        b.add_css_class("window-action");
        b.add_css_class(class);
        b.set_tooltip_text(Some(tip));
        b.update_property(&[gtk::accessible::Property::Label(tip)]);
        let w = window.clone();
        b.connect_clicked(move |_| match class {
            "close" => w.close(),
            "min" => w.minimize(),
            _ => {
                if w.is_maximized() {
                    w.unmaximize()
                } else {
                    w.maximize()
                }
            }
        });
        if class == "max" {
            let button = b.clone();
            window.connect_notify_local(Some("maximized"), move |w, _| {
                let tip = if w.is_maximized() {
                    "복원"
                } else {
                    "최대화"
                };
                button.set_child(Some(&icons::image(if w.is_maximized() {
                    icons::RESTORE
                } else {
                    icons::MAXIMIZE
                })));
                button.set_tooltip_text(Some(tip));
                button.update_property(&[gtk::accessible::Property::Label(tip)]);
            });
        }
        controls.append(&b);
    }
    let center = Label::new(Some(title));
    center.add_css_class("header-title");
    center.set_hexpand(true);
    center.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let actions = GtkBox::new(Orientation::Horizontal, 6);
    let layout = gtk::CenterBox::new();
    layout.set_hexpand(true);
    layout.set_start_widget(Some(&controls));
    layout.set_center_widget(Some(&center));
    layout.set_end_widget(Some(&actions));
    bar.append(&layout);
    let handle = gtk::WindowHandle::new();
    handle.add_css_class("memoivy-titlebar");
    handle.set_child(Some(&bar));
    window.set_titlebar(Some(&handle));
    (bar, actions)
}
