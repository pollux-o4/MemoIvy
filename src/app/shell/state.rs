use crate::{app::canvas::CanvasView, persistence::Notebook};
use gtk::{prelude::*, ApplicationWindow, Box as GtkBox, Button, Label, SearchEntry, TextView};
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Instant};
use uuid::Uuid;

pub(in crate::app) type State = Rc<RefCell<AppState>>;
pub(in crate::app) struct EditorWindow {
    pub window: ApplicationWindow,
    pub view: TextView,
    pub status: Label,
    pub retry: Button,
}
impl EditorWindow {
    pub fn show_status(&self, text: &str, failed: bool) {
        self.status.set_text(text);
        self.retry.set_visible(failed);
    }
}
pub(in crate::app) struct LibraryView {
    pub window: ApplicationWindow,
    pub list: GtkBox,
    pub count: Label,
    pub search: SearchEntry,
    pub error: Label,
    pub add: Button,
}
pub(in crate::app) struct Views {
    pub library: LibraryView,
    pub windows: HashMap<Uuid, EditorWindow>,
    pub canvas: Option<Rc<CanvasView>>,
}
#[derive(Default)]
pub(in crate::app) enum Closing {
    #[default]
    None,
    Editors(Vec<Uuid>),
    Library(Vec<Uuid>),
    Committed,
}
pub(in crate::app) struct AppState {
    pub notebook: Notebook,
    pub views: Views,
    pub refresh_at: Option<Instant>,
    pub closing: Closing,
    pub timer_active: bool,
    pub delete_queue: Vec<Uuid>,
}
impl AppState {
    pub fn new(notebook: Notebook, library: LibraryView) -> State {
        Rc::new(RefCell::new(Self {
            notebook,
            views: Views {
                library,
                windows: HashMap::new(),
                canvas: None,
            },
            refresh_at: None,
            closing: Closing::None,
            timer_active: false,
            delete_queue: Vec::new(),
        }))
    }
    pub fn save_failed(&mut self, error: &str) {
        self.views
            .library
            .error
            .set_text(&format!("저장하지 못했어요: {error}"));
        self.views.library.error.set_visible(true);
        for editor in self.views.windows.values() {
            editor.view.set_editable(true);
            editor.show_status(
                "저장하지 못했어요. 메모를 열어 둔 채 저장 위치를 확인해 주세요.",
                true,
            );
        }
        self.closing = Closing::None;
    }
    #[cfg(test)]
    pub fn fixture(
        path: std::path::PathBuf,
        app: &gtk::Application,
        notes: Vec<crate::persistence::Note>,
    ) -> State {
        std::fs::write(&path, serde_json::to_vec(&notes).unwrap()).unwrap();
        Self::new(
            Notebook::open(path).unwrap(),
            LibraryView {
                window: ApplicationWindow::new(app),
                list: GtkBox::new(gtk::Orientation::Vertical, 0),
                count: Label::new(None),
                search: SearchEntry::new(),
                error: Label::new(None),
                add: Button::new(),
            },
        )
    }
}
