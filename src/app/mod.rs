mod canvas;
mod editor;
#[cfg(test)]
mod gtk_tests;
mod library;
mod shared;
mod shell;
#[cfg(test)]
mod test_support;

use gtk::{prelude::*, Application};

pub(crate) fn run() {
    let app = Application::builder()
        .application_id("io.github.lsm.LinuxStickyMemo")
        .build();
    app.connect_startup(|_| crate::presentation::install());
    app.connect_activate(|app| {
        if let Some(w) = app
            .windows()
            .into_iter()
            .find(|w| w.title().as_deref() == Some("스티키 메모"))
        {
            w.present();
        } else {
            library::build(app);
        }
    });
    app.run();
}
