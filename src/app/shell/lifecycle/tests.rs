use super::super::state::Closing;
use super::*;

pub(in crate::app) fn save_002_close_failure_and_idle_timer() {
    use super::super::state::AppState;
    use crate::app::test_support::until;
    let app = gtk::Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.LifecycleTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let state = AppState::fixture(path.clone(), &app, Vec::new());
    let weak = std::rc::Rc::downgrade(&state);
    state
        .borrow()
        .views
        .library
        .window
        .connect_close_request(move |_| {
            weak.upgrade()
                .map_or(glib::Propagation::Proceed, |state| close_library(&state))
        });
    assert!(!state.borrow().timer_active);
    wake(&state);
    assert!(state.borrow().timer_active);
    until(|| !state.borrow().timer_active);
    assert!(
        !state.borrow().timer_active,
        "idle notebook stops its timer"
    );
    commands::new_note(&state, None);
    flush(&state);
    until(|| matches!(state.borrow().notebook.status(), SaveStatus::Saved));
    let id = state.borrow().notebook.notes()[0].id();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    state
        .borrow_mut()
        .notebook
        .set_text(id, "failed close".into());
    assert_eq!(close_editor(&state, id), glib::Propagation::Stop);
    assert!(!state.borrow().views.windows[&id].view.is_editable());
    until(|| matches!(state.borrow().notebook.status(), SaveStatus::Failed(_)));
    assert!(matches!(state.borrow().closing, Closing::None));
    assert!(state.borrow().views.windows[&id].view.is_editable());
    assert!(state.borrow().views.windows[&id].retry.is_visible());
    std::fs::remove_dir(&path).unwrap();
    flush(&state);
    until(|| matches!(state.borrow().notebook.status(), SaveStatus::Saved));
    until(|| !state.borrow().timer_active);
    mark(&state);
    assert_eq!(
        state.borrow().views.windows[&id].status.text(),
        "저장됨",
        "no-op refresh must retain saved status"
    );
    until(|| !state.borrow().timer_active);
    state
        .borrow_mut()
        .notebook
        .set_text(id, "close both".into());
    assert_eq!(close_editor(&state, id), glib::Propagation::Stop);
    assert_eq!(close_library(&state), glib::Propagation::Stop);
    assert!(matches!(&state.borrow().closing, Closing::Library(ids) if ids == &[id]));
    until(|| state.borrow().views.windows.is_empty());
    until(|| !state.borrow().timer_active);
    let pending = AppState::fixture(dir.path().join("pending.json"), &app, Vec::new());
    let weak = std::rc::Rc::downgrade(&pending);
    pending
        .borrow()
        .views
        .library
        .window
        .connect_close_request(move |_| {
            weak.upgrade()
                .map_or(glib::Propagation::Proceed, |state| close_library(&state))
        });
    commands::new_note(&pending, None);
    assert_eq!(close_library(&pending), glib::Propagation::Stop);
    until(|| matches!(pending.borrow().notebook.status(), SaveStatus::Saved));
    let created = pending.borrow().notebook.notes()[0].id();
    assert!(pending.borrow().views.windows[&created].view.is_editable());
    assert!(!pending.borrow().views.library.window.is_visible());
    commands::delete_notes(&pending, vec![created]);
    assert_eq!(close_library(&pending), glib::Propagation::Stop);
    until(|| !pending.borrow().timer_active);
    assert!(pending.borrow().notebook.notes().is_empty());
    assert!(pending.borrow().views.windows.is_empty());
    // A queued wake must tolerate GTK tearing down the owning application window.
    let detached = AppState::fixture(dir.path().join("detached.json"), &app, Vec::new());
    wake(&detached);
    detached.borrow().views.library.window.destroy();
    until(|| !detached.borrow().timer_active);
}
