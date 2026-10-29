use super::{
    state::{AppState, Operation, State},
    widgets::refresh,
    windows::open_note,
};
use gtk::{glib, prelude::*, Application};
use std::time::{Duration, Instant};

pub(super) fn mark(state: &State) {
    let mut s = state.borrow_mut();
    s.queue.changed();
    let now = Instant::now();
    s.changed_at = Some(now);
    s.first_change.get_or_insert(now);
    s.refresh_at = Some(now);
    for editor in s.windows.values() {
        editor.show_status("저장 중…", false);
    }
}
pub(super) fn dispatch(s: &mut AppState) {
    let notes = &s.store.notes;
    s.queue.dispatch(notes);
    s.changed_at = None;
    s.first_change = None;
}
pub(super) fn start(state: &State, app: &Application) {
    let st = state.clone();
    let a = app.clone();
    glib::timeout_add_local(Duration::from_millis(16), move || tick(&st, &a));
}

fn process_result(st: &State, a: &Application) {
    let mut s = st.borrow_mut();
    let result = s.queue.poll();
    if let Some((transaction, result)) = result {
        if transaction {
            let operation = s.operation.take().map(|pending| pending.0);
            s.add.set_sensitive(true);
            match result {
                Ok(()) => {
                    match operation {
                        Some(Operation::Create(note)) => {
                            let id = note.id;
                            s.store.notes.push(note);
                            drop(s);
                            mark(st);
                            refresh(st, a);
                            open_note(st, a, id);
                            s = st.borrow_mut();
                        }
                        Some(Operation::Delete(id)) => {
                            s.store.notes.retain(|n| n.id != id);
                            let w = s.windows.remove(&id).map(|editor| editor.window);
                            s.bypass = true;
                            drop(s);
                            if let Some(w) = w {
                                w.close();
                            }
                            mark(st);
                            refresh(st, a);
                            s = st.borrow_mut();
                            s.bypass = false;
                        }
                        None => {}
                    }
                    dispatch(&mut s);
                }
                Err(e) => {
                    if let Some(Operation::Delete(id)) = operation {
                        if let Some(editor) = s.windows.get(&id) {
                            editor.view.set_editable(true);
                        }
                    }
                    s.queue.changed();
                    s.queue.record_failure();
                    s.save_failed(&e);
                }
            }
        } else {
            match result {
                Ok(()) => {
                    if s.queue.clean() {
                        s.error.set_visible(false);
                        for editor in s.windows.values() {
                            editor.show_status("자동 저장됨", false);
                        }
                    } else if s.operation.is_none() {
                        dispatch(&mut s);
                    }
                }
                Err(e) => {
                    s.save_failed(&e);
                }
            }
        }
    }
}

fn tick(st: &State, a: &Application) -> glib::ControlFlow {
    process_result(st, a);
    let mut s = st.borrow_mut();
    if !s.queue.in_flight() && s.operation.as_ref().is_some_and(|(_, active)| !active) {
        let mut notes = s.store.notes.clone();
        match &s.operation.as_ref().unwrap().0 {
            Operation::Create(note) => notes.push(note.clone()),
            Operation::Delete(id) => notes.retain(|n| n.id != *id),
        }
        s.operation.as_mut().unwrap().1 = true;
        s.queue.transaction(&notes);
    }
    let now = Instant::now();
    if s.operation.is_none()
        && !s.queue.failed()
        && (s
            .changed_at
            .is_some_and(|t| now.duration_since(t) >= Duration::from_millis(300))
            || s.first_change
                .is_some_and(|t| now.duration_since(t) >= Duration::from_secs(1)))
    {
        dispatch(&mut s);
    }
    if s.operation.is_none() && s.queue.clean() && (!s.closing.is_empty() || s.closing_library) {
        let ids = std::mem::take(&mut s.closing);
        let close_library = std::mem::take(&mut s.closing_library);
        s.bypass = true;
        let windows: Vec<_> = ids
            .iter()
            .filter_map(|id| s.windows.get(id).map(|editor| editor.window.clone()))
            .collect();
        let library = s.library.clone();
        drop(s);
        for w in windows {
            w.close();
        }
        if !close_library && st.borrow().windows.is_empty() {
            library.present();
        }
        if close_library {
            if st.borrow().windows.is_empty() {
                library.close();
            } else {
                library.hide();
                for editor in st.borrow().windows.values() {
                    editor.view.set_editable(true);
                }
            }
        }
        st.borrow_mut().bypass = false;
        s = st.borrow_mut();
    }
    let refresh_now = s
        .refresh_at
        .is_some_and(|t| now.duration_since(t) >= Duration::from_millis(180));
    if refresh_now {
        s.refresh_at = None;
    }
    drop(s);
    if refresh_now {
        refresh(st, a);
    }
    glib::ControlFlow::Continue
}
