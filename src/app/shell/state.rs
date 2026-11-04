use crate::{
    app::canvas::CanvasView,
    persistence::{Navigation, Notebook},
};
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
    /// 켜면 프로젝트별 묶음, 끄면 최근 수정 순 목록.
    pub grouped: gtk::ToggleButton,
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
    /// 프로젝트 방문 기록. 바로가기 순위에만 쓰는 파생 데이터다.
    pub navigation: Navigation,
    /// 새 프로젝트 메모가 만들어지면 그 안으로 옮길 메모들.
    pub pending_group: Option<Vec<Uuid>>,
    pub views: Views,
    pub refresh_at: Option<Instant>,
    pub closing: Closing,
    pub timer_active: bool,
    pub delete_queue: Vec<Uuid>,
}
impl AppState {
    pub fn new(notebook: Notebook, navigation: Navigation, library: LibraryView) -> State {
        Rc::new(RefCell::new(Self {
            notebook,
            navigation,
            pending_group: None,
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
            Notebook::open(path.clone()).unwrap(),
            Navigation::beside(&path),
            LibraryView {
                window: ApplicationWindow::new(app),
                list: GtkBox::new(gtk::Orientation::Vertical, 0),
                count: Label::new(None),
                search: SearchEntry::new(),
                grouped: gtk::ToggleButton::new(),
                error: Label::new(None),
                add: Button::new(),
            },
        )
    }
}
