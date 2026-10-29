mod controller;
mod state;
mod widgets;
mod windows;

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
            windows::build(app);
        }
    });
    app.run();
}
