use crate::app::{
    controller::{self, dispatch},
    state::{AppState, Operation},
    widgets::{header, refresh},
};
use crate::persistence::{
    paths::data_path,
    save_queue::SaveQueue,
    store::{Note, Store},
};
use crate::presentation::{icons, theme::watch as dark};
use gtk::{
    glib, prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation,
    ScrolledWindow, SearchEntry,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub(in crate::app) fn build(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("스티키 메모")
        .default_width(440)
        .default_height(600)
        .build();
    super::focus::watch(&window);
    let (_, actions) = header(&window, "스티키 메모");
    let add = Button::new();
    let add_contents = GtkBox::new(Orientation::Horizontal, 4);
    add_contents.append(&icons::image(icons::ADD, true));
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
    let store = match Store::load(path.clone()) {
        Ok(s) => s,
        Err(e) => {
            let dialog = gtk::MessageDialog::builder()
                .transient_for(&window)
                .modal(true)
                .text(format!(
                    "저장 데이터를 읽을 수 없습니다: {e}\n파일을 보존했습니다: {}",
                    path.display()
                ))
                .buttons(gtk::ButtonsType::Close)
                .build();
            dialog.connect_response(|d, _| d.close());
            window.present();
            dialog.present();
            return;
        }
    };
    let outer = GtkBox::new(Orientation::Vertical, 16);
    outer.add_css_class("library");
    dark(outer.upcast_ref());
    outer.set_margin_start(20);
    outer.set_margin_end(20);
    outer.set_margin_top(16);
    outer.set_margin_bottom(16);
    let search = SearchEntry::new();
    search.add_css_class("lsm-search");
    search.set_placeholder_text(Some("메모 검색"));
    search.set_tooltip_text(Some("메모 검색 · Ctrl+F"));
    outer.append(&search);
    let count = Label::new(None);
    count.set_xalign(0.);
    count.add_css_class("count");
    outer.append(&count);
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
    let state = Rc::new(RefCell::new(AppState {
        store,
        canvas: None,
        queue: SaveQueue::new(path),
        windows: HashMap::new(),
        list,
        count,
        search: search.clone(),
        error,
        library: window.clone(),
        changed_at: None,
        first_change: None,
        refresh_at: None,
        closing: Vec::new(),
        closing_library: false,
        bypass: false,
        operation: None,
        add: add.clone(),
    }));
    let canvas = Button::with_label("캔버스");
    canvas.add_css_class("lsm-action");
    let st = state.clone();
    let a = app.clone();
    canvas.connect_clicked(move |_| super::canvas::open(&st, &a, None));
    actions.prepend(&canvas);
    super::shortcuts::attach(&window, &state);
    let st = state.clone();
    let a = app.clone();
    search.connect_search_changed(move |_| refresh(&st, &a));
    let st = state.clone();
    let _a = app.clone();
    add.connect_clicked(move |_| {
        let mut s = st.borrow_mut();
        if s.operation.is_none() {
            let mut note = Note::new();
            note.canvas.position = Some(crate::persistence::canvas::vacant(&s.store.notes));
            s.operation = Some((Operation::Create(note), false));
            s.add.set_sensitive(false);
        }
    });
    let st = state.clone();
    window.connect_close_request(move |w| {
        let mut s = st.borrow_mut();
        if s.windows.is_empty() && (s.bypass || (s.queue.clean() && s.operation.is_none())) {
            let canvas = s.canvas.take();
            drop(s);
            if let Some(canvas) = canvas {
                canvas.destroy();
            }
            return glib::Propagation::Proceed;
        }
        if s.bypass {
            return glib::Propagation::Proceed;
        }
        if s.queue.clean() && s.operation.is_none() {
            if s.windows.is_empty() {
                return glib::Propagation::Proceed;
            }
            w.hide();
            return glib::Propagation::Stop;
        }
        s.closing_library = true;
        for editor in s.windows.values() {
            editor.view.set_editable(false);
        }
        s.queue.retry();
        if s.operation.is_none() {
            dispatch(&mut s);
        }
        glib::Propagation::Stop
    });
    controller::start(&state, app);
    refresh(&state, app);
    window.present();
}
