mod cards;
mod chrome;
mod controller;
mod editor;
mod library;
mod menus;
use crate::paths::data_path;
use crate::theme::watch as dark;
use crate::{
    icons,
    save_queue::SaveQueue,
    store::{self, Note, Store},
};
use cards::refresh;
use chrome::header;
use controller::{dispatch, mark};
use editor::open_note;
use gtk::{
    glib, prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation,
    ScrolledWindow, SearchEntry, TextView,
};
use library::build;
use menus::menu_for;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};
use uuid::Uuid;
type State = Rc<RefCell<AppState>>;
enum Operation {
    Create(Note),
    Delete(Uuid),
}
struct EditorWindow {
    window: ApplicationWindow,
    view: TextView,
    status: Label,
    retry: Button,
}
impl EditorWindow {
    fn show_status(&self, text: &str, failed: bool) {
        self.status.set_text(text);
        self.retry.set_visible(failed);
    }
}
struct AppState {
    store: Store,
    queue: SaveQueue,
    windows: HashMap<Uuid, EditorWindow>,
    list: GtkBox,
    count: Label,
    search: SearchEntry,
    error: Label,
    library: ApplicationWindow,
    changed_at: Option<Instant>,
    first_change: Option<Instant>,
    refresh_at: Option<Instant>,
    closing: Vec<Uuid>,
    closing_library: bool,
    bypass: bool,
    operation: Option<(Operation, bool)>,
    add: Button,
}

impl AppState {
    fn save_failed(&mut self, error: &str) {
        self.error.set_text(&format!("저장하지 못했어요: {error}"));
        self.error.set_visible(true);
        for editor in self.windows.values() {
            editor.view.set_editable(true);
            editor.show_status(
                "저장하지 못했어요. 메모를 열어 둔 채 저장 위치를 확인해 주세요.",
                true,
            );
        }
        self.closing.clear();
        self.closing_library = false;
    }
}

pub(crate) fn run() {
    let app = Application::builder()
        .application_id("io.github.lsm.LinuxStickyMemo")
        .build();
    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_str!("../style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
    app.connect_activate(|app| {
        if let Some(w) = app
            .windows()
            .into_iter()
            .find(|w| w.title().as_deref() == Some("스티키 메모"))
        {
            w.present();
        } else {
            build(app);
        }
    });
    app.run();
}
