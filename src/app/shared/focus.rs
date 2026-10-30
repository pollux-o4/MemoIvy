use gtk::{glib, prelude::*, ApplicationWindow, Widget};
use std::{cell::RefCell, rc::Rc};

type SavedFocus = RefCell<Option<glib::WeakRef<Widget>>>;

// Release the previous IM context before another memo/search window accepts input.
fn sync(window: &ApplicationWindow, active: bool, saved: &SavedFocus) {
    if active {
        if gtk::prelude::GtkWindowExt::focus(window).is_none() {
            // 그사이 숨겨진 위젯에 포커스를 돌려주면 이후 키 입력이 모두 사라진다.
            if let Some(widget) = saved
                .borrow()
                .as_ref()
                .and_then(|weak| weak.upgrade())
                .filter(|widget| widget.is_drawable())
            {
                widget.grab_focus();
            }
        }
    } else if let Some(widget) = gtk::prelude::GtkWindowExt::focus(window) {
        *saved.borrow_mut() = Some(widget.downgrade());
        gtk::prelude::GtkWindowExt::set_focus(window, None::<&Widget>);
    }
}

pub(in crate::app) fn watch(window: &ApplicationWindow) {
    let saved = Rc::new(SavedFocus::new(None));
    window.connect_is_active_notify(move |window| sync(window, window.is_active(), &saved));
}

#[cfg(test)]
pub(in crate::app) mod tests {
    use super::*;

    // Called by the display test on its GTK thread.
    pub(in crate::app) fn focus_001_releases_and_restores_input_owner() {
        let app = gtk::Application::builder()
            .application_id("io.github.lsm.FocusTest")
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let window = ApplicationWindow::new(&app);
        let entry = gtk::SearchEntry::new();
        window.set_child(Some(&entry));
        window.present();
        let context = gtk::glib::MainContext::default();
        for _ in 0..50 {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        entry.grab_focus();
        let saved = SavedFocus::new(None);
        assert!(gtk::prelude::GtkWindowExt::focus(&window).is_some());
        sync(&window, false, &saved);
        assert!(gtk::prelude::GtkWindowExt::focus(&window).is_none());
        sync(&window, true, &saved);
        assert!(gtk::prelude::GtkWindowExt::focus(&window).is_some());
        sync(&window, false, &saved);
        entry.set_visible(false);
        sync(&window, true, &saved);
        assert!(
            gtk::prelude::GtkWindowExt::focus(&window).is_none(),
            "a widget hidden while inactive must not take keyboard input back"
        );
        window.destroy();
    }
}
