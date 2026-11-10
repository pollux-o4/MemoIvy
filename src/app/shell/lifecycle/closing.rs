use super::super::state::{Closing, State};
use super::wake;
use crate::persistence::SaveStatus;
use gtk::{glib, prelude::*};
use uuid::Uuid;

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
pub(super) fn settle(state: &State) {
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
        let mut s = state.borrow_mut();
        for editor in s.views.windows.values() {
            editor.view.set_editable(true);
        }
        s.closing = Closing::None;
    }
}
