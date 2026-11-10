use super::{
    lifecycle,
    state::{AppState, State},
};
use gtk::{prelude::*, Application};
use uuid::Uuid;

/// parent 프로젝트 안에 새 메모를 만든다. None이면 최상위다.
pub(in crate::app) fn new_note(state: &State, parent: Option<Uuid>) {
    create(state, parent, false);
}

pub(in crate::app) fn new_project(state: &State, parent: Option<Uuid>) {
    create(state, parent, true);
}

fn create(state: &State, parent: Option<Uuid>, project: bool) {
    let mut s = state.borrow_mut();
    let position = crate::app::canvas::vacant_position(&s.notebook, parent);
    let requested = if project {
        s.notebook.request_create_project(Some(position), parent)
    } else {
        s.notebook.request_create(Some(position), parent)
    };
    if requested {
        s.views.library.add.set_sensitive(false);
    }
    drop(s);
    lifecycle::wake(state);
}
/// 단축키를 받은 창 기준. 캔버스에서는 지금 보고 있는 프로젝트 안에 만든다.
pub(in crate::app) fn new_note_from(state: &State, window: &gtk::Window) {
    let parent = crate::app::canvas::scope_for(state, window);
    new_note(state, parent);
}
/// 메모들을 새 프로젝트로 묶는다. 프로젝트 메모가 만들어진 뒤 lifecycle이 안으로 옮긴다.
pub(in crate::app) fn group_into_project(
    state: &State,
    ids: Vec<Uuid>,
    position: [f64; 2],
    parent: Option<Uuid>,
) {
    let mut s = state.borrow_mut();
    if ids.is_empty() || s.pending_group.is_some() {
        return;
    }
    if s.notebook.request_create_project(Some(position), parent) {
        s.pending_group = Some(ids);
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
pub(in crate::app) fn open_files(state: &State, app: &Application) {
    crate::app::files::open(state, app);
}
/// 저장·삭제로 본문이 바뀐 뒤 첨부 사용처를 다시 맞춘다. 바뀌었으면 파일 화면을 갱신한다.
pub(in crate::app) fn reconcile_attachments(state: &State) {
    let mut guard = state.borrow_mut();
    let s = &mut *guard;
    let changed = s
        .attachments
        .reconcile(s.notebook.notes(), crate::persistence::now());
    let files = s.views.files.clone();
    drop(guard);
    if let (true, Some(files)) = (changed, files) {
        files.refresh(state);
    }
}
pub(in crate::app) fn open_canvas(state: &State, app: &Application, id: Option<Uuid>) {
    crate::app::canvas::open(state, app, id);
}
/// 캔버스를 열고 그 프로젝트 안으로 들어간다.
pub(in crate::app) fn open_canvas_at(state: &State, app: &Application, project: Uuid) {
    crate::app::canvas::open(state, app, None);
    crate::app::canvas::show_project(state, project);
}

pub(in crate::app) fn initialize_positions(notebook: &mut crate::persistence::Notebook) {
    crate::app::canvas::initialize_positions(notebook);
}

pub(in crate::app) fn creation_allowed(state: &AppState, scope: Option<Uuid>) -> (bool, bool) {
    let book = &state.notebook;
    (
        !book.busy() && book.can_create_note(scope),
        !book.busy() && book.can_create_project(scope),
    )
}
