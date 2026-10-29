use super::*;
use crate::{
    app::state::AppState,
    persistence::{
        save_queue::SaveQueue,
        store::{Note, Store},
    },
};
use std::collections::HashMap;

fn drain() {
    let context = gtk::glib::MainContext::default();
    while context.pending() {
        context.iteration(false);
    }
}

fn button(root: &GtkBox, label: &str) -> Button {
    let mut child = root.first_child();
    while let Some(widget) = child {
        if let Ok(b) = widget.clone().downcast::<Button>() {
            if b.label().as_deref() == Some(label) {
                return b;
            }
        }
        child = widget.next_sibling();
    }
    panic!("missing control: {label}");
}

fn capture(v: &Rc<CanvasView>) {
    let Ok(dir) = std::env::var("LSM_CANVAS_ARTIFACT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let settings = v.window.settings();
    let animated = settings.is_gtk_enable_animations();
    let dark = settings.is_gtk_application_prefer_dark_theme();
    let zoom = v.zoom.get();
    settings.set_gtk_enable_animations(false);
    v.fit();
    for (name, is_dark) in [("light", false), ("dark", true)] {
        settings.set_gtk_application_prefer_dark_theme(is_dark);
        if let Some(id) = v.selected.get() {
            v.signal(id);
        }
        for _ in 0..20 {
            drain();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let paintable = gtk::WidgetPaintable::new(Some(&v.window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, v.window.width() as f64, v.window.height() as f64);
        let node = snapshot.to_node().expect("rendered canvas scene");
        let texture = v.window.renderer().unwrap().render_texture(&node, None);
        texture
            .save_to_png(std::path::Path::new(&dir).join(format!("canvas-{name}.png")))
            .unwrap();
    }
    v.pulse.set(None);
    settings.set_gtk_application_prefer_dark_theme(dark);
    super::motion::transition(v, [zoom, 0., 0.]);
    settings.set_gtk_enable_animations(animated);
}

// Run on the existing GTK test thread; GTK cannot be initialized on parallel test threads.
pub(in crate::app) fn canvas_001_widget_flow_and_recovery() {
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let app = Application::builder()
        .application_id("io.github.lsm.CanvasTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let library = ApplicationWindow::new(&app);
    let mut notes = vec![Note::new(), Note::new(), Note::new()];
    for (i, n) in notes.iter_mut().enumerate() {
        n.text = format!("메모 {i}\n한글과 긴 제목도 보존합니다");
        n.canvas.position = Some([64. + i as f64 * 320., 64.]);
    }
    let (a, b) = (notes[0].id, notes[1].id);
    let state = Rc::new(RefCell::new(AppState {
        store: Store {
            path: path.clone(),
            notes,
        },
        queue: SaveQueue::new(path.clone()),
        canvas: None,
        windows: HashMap::new(),
        list: GtkBox::new(Orientation::Vertical, 0),
        count: Label::new(None),
        search: gtk::SearchEntry::new(),
        error: Label::new(None),
        library: library.clone(),
        changed_at: None,
        first_change: None,
        refresh_at: None,
        closing: Vec::new(),
        closing_library: false,
        bypass: false,
        operation: None,
        add: Button::new(),
    }));
    open(&state, &app, Some(a));
    drain();
    let v = state.borrow().canvas.clone().unwrap();
    v.window.hide();
    gtk::prelude::WidgetExt::unrealize(&v.window);
    v.window.set_default_size(820, 620);
    v.window.present();
    for _ in 0..200 {
        drain();
        if (760..=840).contains(&v.window.width()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        (760..=840).contains(&v.window.width()),
        "small window must not be forced wide: {}",
        v.window.width()
    );
    assert_eq!(v.summary.text(), "메모 3 · 연결 0");
    assert!(
        v.inspector.width() < 350,
        "selection must not consume the board: {}",
        v.inspector.width()
    );
    super::motion::tests::canvas_002_viewport(&v);
    button(&v.inspector, "연결 추가").emit_clicked();
    assert_eq!(state.borrow().store.notes[0].canvas.next, vec![b]);
    capture(&v);
    let mut child = v.inspector.first_child();
    let checkbox = loop {
        let w = child.expect("completion control");
        if let Ok(check) = w.clone().downcast::<gtk::CheckButton>() {
            break check;
        }
        child = w.next_sibling();
    };
    checkbox.grab_focus();
    checkbox.set_active(true);
    assert_eq!(
        gtk::prelude::GtkWindowExt::focus(&v.window)
            .unwrap()
            .widget_name(),
        "canvas-control-done"
    );
    assert!(state.borrow().store.notes[0].canvas.done);
    assert!(
        !state.borrow().store.notes[1].canvas.done,
        "signal never completes the target"
    );
    state.borrow_mut().store.notes[0].text = "원본 수정 📝".into();
    v.undo.emit_clicked();
    assert!(!state.borrow().store.notes[0].canvas.done);
    assert_eq!(
        state.borrow().store.notes[0].text,
        "원본 수정 📝",
        "canvas undo preserves source edits"
    );
    assert_eq!(state.borrow().store.notes[0].canvas.next, vec![b]);
    let card = v.fixed.first_child().unwrap();
    let handle = card.first_child().unwrap();
    handle.grab_focus();
    let controllers = handle.observe_controllers();
    let keys = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .unwrap();
    let stopped: bool = keys.emit_by_name(
        "key-pressed",
        &[
            &gtk::gdk::Key::Right,
            &0u32,
            &gtk::gdk::ModifierType::empty(),
        ],
    );
    assert!(stopped);
    assert_eq!(
        state.borrow().store.notes[0].canvas.position,
        Some([80., 64.])
    );
    drain();
    assert!(
        gtk::prelude::GtkWindowExt::focus(&v.window).is_some(),
        "rebuilding cards retains keyboard focus"
    );
    v.undo.emit_clicked();
    assert_eq!(
        state.borrow().store.notes[0].canvas.position,
        Some([64., 64.])
    );
    controller::dispatch(&mut state.borrow_mut());
    for _ in 0..200 {
        if let Some((_, result)) = state.borrow_mut().queue.poll() {
            result.unwrap();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let loaded = Store::load(path).unwrap();
    assert_eq!(loaded.notes.len(), 3);
    assert_eq!(loaded.notes[0].canvas.next, vec![b]);
    crate::persistence::canvas::remove(&mut state.borrow_mut().store.notes, b);
    v.render();
    v.undo.emit_clicked();
    assert_eq!(state.borrow().store.notes.len(), 2);
    assert!(
        state.borrow().store.notes[0].canvas.next.is_empty(),
        "undo must not restore deleted targets"
    );
    let failed_path = dir.path().join("blocked");
    std::fs::create_dir(&failed_path).unwrap();
    state.borrow_mut().queue = SaveQueue::new(failed_path);
    controller::mark(&state);
    controller::dispatch(&mut state.borrow_mut());
    for _ in 0..200 {
        if let Some((_, result)) = state.borrow_mut().queue.poll() {
            assert!(result.is_err());
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    v.update_status();
    assert!(v.retry.is_visible());
    assert!(v.status.text().contains("실패"));
    assert_eq!(state.borrow().store.notes[0].text, "원본 수정 📝");
    state.borrow_mut().store.notes.clear();
    v.render();
    assert_eq!(v.summary.text(), "메모 0 · 연결 0");
    assert!(v.fixed.first_child().is_none());
    v.window.close();
    assert!(!v.window.is_visible());
    v.destroy();
    library.destroy();
    drain();
    assert!(app.windows().is_empty());
}
