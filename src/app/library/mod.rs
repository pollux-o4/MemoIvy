mod cards;
use crate::app::{
    shared::header,
    shell::{
        commands, lifecycle,
        state::{AppState, LibraryView},
    },
};
use crate::persistence::{data_path, Navigation, Notebook};
#[cfg(test)]
pub(in crate::app) use cards::note_004_preview_disclosure;
#[cfg(test)]
pub(in crate::app) use cards::project_002_sections_toggle_in_place_and_keep_preview;
pub(in crate::app) use cards::refresh;
use gtk::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Label, Orientation, ScrolledWindow,
    SearchEntry,
};

pub(in crate::app) fn build(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("MemoIvy")
        .default_width(440)
        .default_height(600)
        .build();
    crate::app::shared::focus::watch(&window);
    let (_, actions) = header(&window, "MemoIvy");
    let add = gtk::MenuButton::new();
    actions.append(&add);
    let path = match data_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let store = match Notebook::open(path.clone()) {
        Ok(s) => s,
        Err(e) => {
            show_load_error(
                &window,
                &format!(
                    "저장 데이터를 읽을 수 없습니다: {e}\n파일을 보존했습니다: {}",
                    path.display()
                ),
            );
            return;
        }
    };
    let library = build_contents(&window, add.clone(), &path);
    let search = library.search.clone();
    let mut notebook = store;
    commands::initialize_positions(&mut notebook);
    let state = AppState::new(
        notebook,
        Navigation::beside(&path),
        crate::persistence::Attachments::beside(&path),
        library,
    );
    let canvas = crate::app::shared::canvas_button("캔버스 열기");
    let st = state.clone();
    let a = app.clone();
    canvas.connect_clicked(move |_| commands::open_canvas(&st, &a, None));
    actions.prepend(&canvas);
    crate::app::shared::shortcuts::attach(&window, &state);
    let st = state.clone();
    let a = app.clone();
    search.connect_search_changed(move |_| crate::app::shell::views::refresh_all(&st, &a));
    let grouped = state.borrow().views.library.grouped.clone();
    let st = state.clone();
    let a = app.clone();
    grouped.connect_toggled(move |_| refresh(&st, &a));
    let weak = std::rc::Rc::downgrade(&state);
    let permissions = weak.clone();
    crate::app::shared::creation::configure(
        &add,
        "library-create",
        move |project| {
            if let Some(state) = weak.upgrade() {
                if project {
                    commands::new_project(&state, None);
                } else {
                    commands::new_note(&state, None);
                }
            }
        },
        move || {
            permissions
                .upgrade()
                .map(|state| commands::creation_allowed(&state.borrow(), None))
                .unwrap_or((false, false))
        },
    );
    let st = state.clone();
    window.connect_close_request(move |_| lifecycle::close_library(&st));
    lifecycle::wake(&state);
    refresh(&state, app);
    window.present();
}

fn show_load_error(window: &ApplicationWindow, message: &str) {
    window.present();
    crate::app::shared::message(window, "저장 오류", message);
}

fn build_contents(
    window: &ApplicationWindow,
    add: gtk::MenuButton,
    path: &std::path::Path,
) -> LibraryView {
    let outer = GtkBox::new(Orientation::Vertical, 16);
    outer.add_css_class("library");
    outer.set_margin_start(20);
    outer.set_margin_end(20);
    outer.set_margin_top(16);
    outer.set_margin_bottom(16);
    let search = SearchEntry::new();
    search.add_css_class("memoivy-search");
    search.set_placeholder_text(Some("메모 검색"));
    search.set_tooltip_text(Some("메모 검색 · Ctrl+F"));
    outer.append(&search);
    let count = Label::new(None);
    count.set_xalign(0.);
    count.set_hexpand(true);
    count.add_css_class("count");
    // 프로젝트별 묶음과 최근 순 목록을 오가는 세그먼트.
    let grouped = gtk::ToggleButton::with_label("프로젝트별");
    grouped.set_active(true);
    let recent = gtk::ToggleButton::with_label("최근");
    recent.set_group(Some(&grouped));
    let segment = GtkBox::new(Orientation::Horizontal, 0);
    segment.add_css_class("segmented");
    segment.append(&grouped);
    segment.append(&recent);
    segment.set_visible(false);
    let bar = GtkBox::new(Orientation::Horizontal, 8);
    bar.append(&count);
    bar.append(&segment);
    outer.append(&bar);
    let list = GtkBox::new(Orientation::Vertical, 0);
    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    outer.append(&scroll);
    let error = Label::new(None);
    error.set_wrap(true);
    error.add_css_class("error");
    error.set_visible(false);
    outer.append(&error);
    window.set_child(Some(&outer));
    LibraryView {
        preferences: std::cell::RefCell::new(crate::persistence::LibraryPreferences::beside(path)),
        shown: std::cell::Cell::new(None),
        window: window.clone(),
        list,
        count,
        search,
        grouped,
        error,
        add,
    }
}
