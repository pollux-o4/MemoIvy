use super::*;

pub(in crate::app) fn project_003_new_menu_and_empty_project() {
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let app = Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.CreationTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let state = AppState::fixture(dir.path().join("notes.json"), &app, vec![]);
    open(&state, &app, None);
    let v = state.borrow().views.canvas.clone().unwrap();
    until(|| v.session.fitted.get());
    let menu = named(v.widgets.window.upcast_ref(), "canvas-create")
        .and_downcast::<gtk::MenuButton>()
        .unwrap();
    let popup = menu.popover().unwrap();
    panels::creation::assert_motion(&menu);
    menu.popup();
    until(|| popup.is_mapped());
    let create = named(popup.upcast_ref(), "create-project")
        .and_downcast::<Button>()
        .unwrap();
    assert!(create.is_sensitive());
    create.emit_clicked();
    until(|| state.borrow().notebook.notes().len() == 1 && !state.borrow().notebook.needs_tick());
    let id = state.borrow().notebook.notes()[0].id();
    assert!(state.borrow().notebook.note(id).unwrap().canvas().project);
    assert!(v.node(id).unwrap().project.is_some());
    assert_eq!(v.node(id).unwrap().project.unwrap().notes, 0);
    assert_eq!(port::move_targets(&state.borrow(), &[])[0].id, id);
    let editor = state.borrow().views.windows[&id].window.clone();
    assert_eq!(editor.title().as_deref(), Some("프로젝트"));
    editor.close();
    until(|| state.borrow().views.windows.is_empty());
    let settings = v.widgets.window.settings();
    let animated = settings.is_gtk_enable_animations();
    settings.set_gtk_enable_animations(false);
    v.enter(id);
    assert_eq!(v.session.scope.get(), Some(id));
    assert!(v.widgets.empty.is_visible());
    menu.popup();
    until(|| popup.is_mapped());
    named(popup.upcast_ref(), "create-note")
        .and_downcast::<Button>()
        .unwrap()
        .emit_clicked();
    until(|| state.borrow().notebook.notes().len() == 2 && !state.borrow().notebook.needs_tick());
    let child = state
        .borrow()
        .notebook
        .notes()
        .iter()
        .find(|n| n.id() != id)
        .unwrap()
        .id();
    assert_eq!(
        state.borrow().notebook.note(child).unwrap().canvas().parent,
        Some(id)
    );
    let editor = state.borrow().views.windows[&child].window.clone();
    assert_eq!(editor.title().as_deref(), Some("메모"));
    editor.close();
    until(|| state.borrow().views.windows.is_empty());
    v.select_only(child);
    v.lift_selected();
    wait_notebook(&state);
    assert!(state.borrow().notebook.note(id).unwrap().canvas().project);
    assert!(v.session.nodes.borrow().is_empty());
    v.go_to(None);
    assert!(v.node(id).unwrap().project.is_some());
    // Popover closure and reopening must not leave an animation or focus behind.
    menu.popup();
    until(|| popup.is_mapped());
    popup.popdown();
    until(|| !popup.is_mapped());
    menu.popup();
    until(|| popup.is_mapped());
    popup.popdown();
    settings.set_gtk_enable_animations(animated);
    v.destroy();
}
