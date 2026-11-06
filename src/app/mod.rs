mod canvas;
mod editor;
mod files;
#[cfg(test)]
mod gtk_tests;
mod library;
mod shared;
mod shell;
#[cfg(test)]
mod test_support;
mod widgets;

use gtk::{prelude::*, Application};

pub(crate) fn run() {
    let app = Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy")
        .build();
    app.connect_startup(|_| crate::presentation::install());
    app.connect_activate(|app| {
        if let Some(w) = app
            .windows()
            .into_iter()
            .find(|w| w.title().as_deref() == Some("MemoIvy"))
        {
            w.present();
        } else {
            library::build(app);
        }
    });
    app.run();
}
