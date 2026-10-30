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
fn delete_note(state: &State, id: Uuid) {
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
/// 저장 계층은 한 번에 하나만 삭제한다. 나머지는 줄을 세워 앞선 삭제가 끝나면 이어 간다.
pub(in crate::app) fn delete_notes(state: &State, ids: Vec<Uuid>) {
    let mut s = state.borrow_mut();
    for id in &ids {
        if let Some(editor) = s.views.windows.get(id) {
            editor.view.set_editable(false);
        }
    }
    s.delete_queue.extend(ids);
    drop(s);
    next_delete(state);
}
/// 대기 중인 다음 삭제를 시작한다. 진행 중이면 그대로 두고, 그사이 사라진 메모는 건너뛴다.
pub(in crate::app) fn next_delete(state: &State) {
    loop {
        let mut s = state.borrow_mut();
        if s.delete_queue.is_empty() || s.notebook.busy() {
            return;
        }
        let id = s.delete_queue.remove(0);
        drop(s);
        delete_note(state, id);
        if state.borrow().notebook.busy() {
            return;
        }
    }
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
