mod chrome;
pub(in crate::app) mod focus;
mod menus;
pub(in crate::app) mod shortcuts;
pub(in crate::app) use chrome::header;
pub(in crate::app) use menus::{
    confirm, confirm_delete, lazy_menu_for, menu_for, menu_popover, menu_row, message, source_row,
};
pub(in crate::app) const COLORS: [(&str, &str); 5] = [
    ("yellow", "노랑"),
    ("pink", "분홍"),
    ("blue", "파랑"),
    ("green", "초록"),
    ("purple", "보라"),
];

use gtk::prelude::*;

pub(in crate::app) fn find_named(widget: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
    if widget.widget_name() == name {
        return Some(widget.clone());
    }
    let mut child = widget.first_child();
    while let Some(next) = child {
        if let Some(found) = find_named(&next, name) {
            return Some(found);
        }
        child = next.next_sibling();
    }
    None
}

pub(in crate::app) fn close_on_escape(window: &impl IsA<gtk::Window>) {
    let keys = gtk::EventControllerKey::new();
    let weak = window.as_ref().downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(window) = weak.upgrade() {
                window.close();
            }
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    window.as_ref().add_controller(keys);
}

pub(in crate::app) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
