use super::super::{commands, state::State, views::refresh_all};
use crate::persistence::NotebookEvent;
use gtk::prelude::*;

pub(super) fn apply(state: &State, app: &gtk::Application, events: Vec<NotebookEvent>) {
    for event in events {
        match event {
            NotebookEvent::Created(id) => {
                state.borrow().views.library.add.set_sensitive(true);
                let group = state.borrow_mut().pending_group.take();
                if let Some(children) = group {
                    crate::app::canvas::adopt(state, id, children);
                }
                refresh_all(state, app);
                commands::open_note(state, app, id);
            }
            NotebookEvent::Deleted(id) => {
                state.borrow_mut().navigation.forget(id);
                let editor = state.borrow_mut().views.windows.remove(&id);
                if let Some(editor) = editor {
                    editor.window.destroy();
                }
                state.borrow().views.library.add.set_sensitive(true);
                refresh_all(state, app);
                commands::reconcile_attachments(state);
            }
            NotebookEvent::Saved => {
                let s = state.borrow();
                s.views.library.error.set_visible(false);
                for editor in s.views.windows.values() {
                    editor.show_status("저장됨", false);
                }
                drop(s);
                commands::reconcile_attachments(state);
            }
            NotebookEvent::SaveFailed(error) | NotebookEvent::OperationFailed(error) => {
                let mut s = state.borrow_mut();
                s.views.library.add.set_sensitive(true);
                s.pending_group = None;
                // 남은 삭제는 멈춘다. 편집 잠금은 save_failed가 푼다.
                s.delete_queue.clear();
                s.save_failed(&error.to_string());
            }
        }
    }
}
