use super::{
    commands,
    state::{Closing, State},
    views::refresh_all,
};
use crate::persistence::{NotebookEvent, SaveStatus};
use gtk::{glib, prelude::*};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub(in crate::app) fn mark(state: &State) {
    let mut s = state.borrow_mut();
    s.refresh_at = Some(Instant::now());
    match s.notebook.status() {
        SaveStatus::Saved => {
            for editor in s.views.windows.values() {
                editor.show_status("저장됨", false);
            }
        }
        SaveStatus::Pending | SaveStatus::Saving => {
            for editor in s.views.windows.values() {
                editor.show_status("저장 중…", false);
            }
        }
        SaveStatus::Failed(_) => {}
    }
    drop(s);
    wake(state);
}
pub(in crate::app) fn flush(state: &State) {
    state.borrow_mut().notebook.flush();
    wake(state);
}
pub(in crate::app) fn wake(state: &State) {
    if state.borrow().timer_active {
        return;
    }
    state.borrow_mut().timer_active = true;
    let weak = std::rc::Rc::downgrade(state);
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let Some(state) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        tick(&state)
    });
}
pub(in crate::app) fn close_editor(state: &State, id: Uuid) -> glib::Propagation {
    let mut s = state.borrow_mut();
    if matches!(s.closing, Closing::Committed)
        || s.notebook.note(id).is_none()
        || matches!(s.notebook.status(), SaveStatus::Saved)
    {
        s.views.windows.remove(&id);
        if s.views.windows.is_empty() {
            s.views.library.window.present();
        }
        return glib::Propagation::Proceed;
    }
    match &mut s.closing {
        Closing::Editors(ids) | Closing::Library(ids) => {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        _ => s.closing = Closing::Editors(vec![id]),
    }
    if let Some(editor) = s.views.windows.get(&id) {
        editor.view.set_editable(false);
    }
    s.notebook.flush();
    drop(s);
    wake(state);
    glib::Propagation::Stop
}
pub(in crate::app) fn close_library(state: &State) -> glib::Propagation {
    let mut s = state.borrow_mut();
    if matches!(s.closing, Closing::Committed) || matches!(s.notebook.status(), SaveStatus::Saved) {
        if s.views.windows.is_empty() {
            let canvas = s.views.canvas.take();
            drop(s);
            if let Some(canvas) = canvas {
                canvas.destroy();
            }
            return glib::Propagation::Proceed;
        }
        s.views.library.window.set_visible(false);
        return glib::Propagation::Stop;
    }
    let ids = match std::mem::take(&mut s.closing) {
        Closing::Editors(ids) | Closing::Library(ids) => ids,
        _ => Vec::new(),
    };
    s.closing = Closing::Library(ids);
    for editor in s.views.windows.values() {
        editor.view.set_editable(false);
    }
    s.notebook.flush();
    drop(s);
    wake(state);
    glib::Propagation::Stop
}
fn tick(state: &State) -> glib::ControlFlow {
    let now = Instant::now();
    let events = state.borrow_mut().notebook.tick(now);
    let app = state.borrow().views.library.window.application();
    let Some(app) = app else {
        state.borrow_mut().timer_active = false;
        return glib::ControlFlow::Break;
    };
    let received = !events.is_empty();
    for event in events {
        match event {
            NotebookEvent::Created(id) => {
                state.borrow().views.library.add.set_sensitive(true);
                refresh_all(state, &app);
                commands::open_note(state, &app, id);
            }
            NotebookEvent::Deleted(id) => {
                let editor = state.borrow_mut().views.windows.remove(&id);
                if let Some(editor) = editor {
                    editor.window.destroy();
                }
                state.borrow().views.library.add.set_sensitive(true);
                refresh_all(state, &app);
            }
            NotebookEvent::Saved => {
                let s = state.borrow();
                s.views.library.error.set_visible(false);
                for editor in s.views.windows.values() {
                    editor.show_status("저장됨", false);
                }
            }
            NotebookEvent::SaveFailed(error) | NotebookEvent::OperationFailed(error) => {
                let mut s = state.borrow_mut();
                s.views.library.add.set_sensitive(true);
                s.save_failed(&error.to_string());
            }
        }
    }
    if received {
        crate::app::canvas::update_status(state);
    }
    let mut s = state.borrow_mut();
    if matches!(s.notebook.status(), SaveStatus::Saved)
        && !matches!(s.closing, Closing::None | Closing::Committed)
    {
        let closing = std::mem::replace(&mut s.closing, Closing::Committed);
        let windows: Vec<_> = match &closing {
            Closing::Editors(ids) | Closing::Library(ids) => ids
                .iter()
                .filter_map(|id| s.views.windows.get(id).map(|editor| editor.window.clone()))
                .collect(),
            _ => Vec::new(),
        };
        let library = s.views.library.window.clone();
        drop(s);
        for window in windows {
            window.close();
        }
        if matches!(closing, Closing::Library(_)) {
            library.close();
        }
        s = state.borrow_mut();
        for editor in s.views.windows.values() {
            editor.view.set_editable(true);
        }
        s.closing = Closing::None;
    }
    let refresh = s
        .refresh_at
        .is_some_and(|at| now.duration_since(at) >= Duration::from_millis(180));
    if refresh {
        s.refresh_at = None;
    }
    let keep = s.notebook.needs_tick() || s.refresh_at.is_some();
    s.timer_active = keep;
    drop(s);
    if refresh {
        refresh_all(state, &app);
    }
    if keep {
        glib::ControlFlow::Continue
    } else {
        glib::ControlFlow::Break
    }
}

#[cfg(test)]
pub(in crate::app) fn save_002_close_failure_and_idle_timer() {
    use super::state::AppState;
    fn until(done: impl Fn() -> bool) {
        let end = Instant::now() + Duration::from_secs(3);
        let context = glib::MainContext::default();
        while !done() {
            while context.pending() {
                context.iteration(false);
            }
            assert!(Instant::now() < end, "lifecycle timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    let app = gtk::Application::builder()
        .application_id("io.github.lsm.LifecycleTest")
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
    commands::new_note(&state);
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
    commands::new_note(&pending);
    assert_eq!(close_library(&pending), glib::Propagation::Stop);
    until(|| matches!(pending.borrow().notebook.status(), SaveStatus::Saved));
    let created = pending.borrow().notebook.notes()[0].id();
    assert!(pending.borrow().views.windows[&created].view.is_editable());
    assert!(!pending.borrow().views.library.window.is_visible());
    commands::delete_note(&pending, created);
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
