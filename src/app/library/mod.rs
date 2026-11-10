mod cards;
use crate::app::{
    shared::header,
    shell::{
        commands, lifecycle,
        state::{AppState, LibraryView},
    },
};
use crate::persistence::{data_path, Navigation, Notebook};
use crate::presentation::icons;
pub(in crate::app) use cards::refresh;
use gtk::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation,
    ScrolledWindow, SearchEntry,
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
    let add = Button::new();
    let add_contents = GtkBox::new(Orientation::Horizontal, 4);
    add_contents.append(&icons::image(icons::ADD));
    add_contents.append(&Label::new(Some("새 메모")));
    add.set_child(Some(&add_contents));
    add.add_css_class("new-note");
    add.set_tooltip_text(Some("새 메모 · Ctrl+N"));
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
    let canvas = Button::with_label("캔버스");
    canvas.add_css_class("memoivy-action");
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
    let st = state.clone();
    add.connect_clicked(move |_| commands::new_note(&st, None));
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

fn build_contents(window: &ApplicationWindow, add: Button, path: &std::path::Path) -> LibraryView {
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
