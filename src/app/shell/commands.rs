use super::{lifecycle, state::State};
use gtk::{prelude::*, Application};
use uuid::Uuid;

pub(in crate::app) fn new_note(state: &State) {
    let mut s = state.borrow_mut();
    let position = crate::app::canvas::vacant_position(&s.notebook);
    if s.notebook.request_create(Some(position)) {
        s.views.library.add.set_sensitive(false);
    }
    drop(s);
    lifecycle::wake(state);
}
pub(in crate::app) fn delete_note(state: &State, id: Uuid) {
    let mut s = state.borrow_mut();
    if s.notebook.request_delete(id) {
        if let Some(editor) = s.views.windows.get(&id) {
            editor.view.set_editable(false);
        }
        s.views.library.add.set_sensitive(false);
    }
    drop(s);
    lifecycle::wake(state);
}
pub(in crate::app) fn open_note(state: &State, app: &Application, id: Uuid) {
    crate::app::editor::open_note(state, app, id);
}
pub(in crate::app) fn open_canvas(state: &State, app: &Application, id: Option<Uuid>) {
    crate::app::canvas::open(state, app, id);
}

pub(in crate::app) fn initialize_positions(notebook: &mut crate::persistence::Notebook) {
    crate::app::canvas::initialize_positions(notebook);
}
