use crate::persistence::{
    save_queue::SaveQueue,
    store::{Note, Store},
};
use gtk::{prelude::*, ApplicationWindow, Box as GtkBox, Button, Label, SearchEntry, TextView};
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Instant};
use uuid::Uuid;

pub(super) type State = Rc<RefCell<AppState>>;
pub(super) enum Operation {
    Create(Note),
    Delete(Uuid),
}
pub(super) struct EditorWindow {
    pub(super) window: ApplicationWindow,
    pub(super) view: TextView,
    pub(super) status: Label,
    pub(super) retry: Button,
}
impl EditorWindow {
    pub(super) fn show_status(&self, text: &str, failed: bool) {
        self.status.set_text(text);
        self.retry.set_visible(failed);
    }
}
pub(super) struct AppState {
    pub(super) store: Store,
    pub(super) queue: SaveQueue,
    pub(super) windows: HashMap<Uuid, EditorWindow>,
    pub(super) list: GtkBox,
    pub(super) count: Label,
    pub(super) search: SearchEntry,
    pub(super) error: Label,
    pub(super) library: ApplicationWindow,
    pub(super) changed_at: Option<Instant>,
    pub(super) first_change: Option<Instant>,
    pub(super) refresh_at: Option<Instant>,
    pub(super) closing: Vec<Uuid>,
    pub(super) closing_library: bool,
    pub(super) bypass: bool,
    pub(super) operation: Option<(Operation, bool)>,
    pub(super) add: Button,
}

impl AppState {
    pub(super) fn save_failed(&mut self, error: &str) {
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
