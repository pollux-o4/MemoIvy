use super::canvas::valid_position;
use super::{
    links, nesting,
    note::{now, Note},
    save_queue::SaveQueue,
    store::Store,
};
use std::{
    fmt, io,
    path::PathBuf,
    time::{Duration, Instant},
};
use uuid::Uuid;

mod canvas_edit;
pub(crate) use canvas_edit::CanvasEdit;

#[derive(Debug)]
pub struct LoadError(io::Error);
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveError(String);
impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for SaveError {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    Saved,
    Pending,
    Saving,
    Failed(super::SaveError),
}
pub enum NotebookEvent {
    Saved,
    SaveFailed(SaveError),
    Created(Uuid),
    Deleted(Uuid),
    OperationFailed(SaveError),
}
enum Operation {
    Create(Note),
    Delete(Uuid),
}
impl Operation {
    /// Apply the pending operation to either a transaction snapshot or committed state.
    fn apply(&self, notes: &mut Vec<Note>) {
        match self {
            Self::Create(note) => notes.push(note.clone()),
            Self::Delete(id) => links::remove(notes, *id),
        }
        nesting::promote_projects(notes);
    }
}
pub struct Notebook {
    notes: Vec<Note>,
    queue: SaveQueue,
    changed_at: Option<Instant>,
    first_change: Option<Instant>,
    operation: Option<(Operation, bool)>,
    error: Option<SaveError>,
    flush_requested: bool,
}
impl Notebook {
    pub fn open(path: PathBuf) -> Result<Self, super::LoadError> {
        let notes = Store::load(path.clone()).map_err(LoadError)?.notes;
        Ok(Self {
            notes,
            queue: SaveQueue::new(path),
            changed_at: None,
            first_change: None,
            operation: None,
            error: None,
            flush_requested: false,
        })
    }
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }
    pub fn note(&self, id: Uuid) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }
    pub fn status(&self) -> SaveStatus {
        if let Some(error) = &self.error {
            SaveStatus::Failed(error.clone())
        } else if self.queue.in_flight() {
            SaveStatus::Saving
        } else if self.busy() || !self.queue.clean() {
            SaveStatus::Pending
        } else {
            SaveStatus::Saved
        }
    }
    pub fn busy(&self) -> bool {
        self.operation.is_some()
    }
    fn changed(&mut self, at: Instant) {
        self.queue.changed();
        self.error = None;
        self.changed_at = Some(at);
        self.first_change.get_or_insert(at);
    }
    pub fn set_text(&mut self, id: Uuid, text: String) -> bool {
        let Some(note) = self.notes.iter_mut().find(|n| n.id == id) else {
            return false;
        };
        if note.text == text {
            return false;
        }
        note.text = text;
        note.modified = now();
        self.changed(Instant::now());
        true
    }
    pub fn set_color(&mut self, id: Uuid, color: &str) -> bool {
        if !super::note::COLORS.contains(&color) {
            return false;
        }
        let Some(note) = self.notes.iter_mut().find(|n| n.id == id) else {
            return false;
        };
        if note.color == color {
            return false;
        }
        note.color = color.into();
        note.modified = now();
        self.changed(Instant::now());
        true
    }
    pub fn edit_canvas(&mut self, edit: impl FnOnce(&mut CanvasEdit<'_>)) -> bool {
        let mut canvas = CanvasEdit {
            notes: &mut self.notes,
            changed: false,
        };
        edit(&mut canvas);
        let changed = nesting::promote_projects(canvas.notes) | canvas.changed;
        if changed {
            self.changed(Instant::now());
        }
        changed
    }
    /// parent 안에 새 메모를 만든다. 깊이 상한을 넘거나 없는 부모면 거절한다.
    pub fn request_create(&mut self, position: Option<[f64; 2]>, parent: Option<Uuid>) -> bool {
        self.request_create_kind(position, parent, false)
    }

    pub fn can_create_note(&self, parent: Option<Uuid>) -> bool {
        nesting::can_create_in(&self.notes, parent)
    }

    pub fn can_create_project(&self, parent: Option<Uuid>) -> bool {
        nesting::can_create_project_in(&self.notes, parent)
    }

    /// 프로젝트 종류도 최초 생성 트랜잭션에 포함해 원자 저장한다.
    pub fn request_create_project(
        &mut self,
        position: Option<[f64; 2]>,
        parent: Option<Uuid>,
    ) -> bool {
        self.can_create_project(parent) && self.request_create_kind(position, parent, true)
    }

    fn request_create_kind(
        &mut self,
        position: Option<[f64; 2]>,
        parent: Option<Uuid>,
        project: bool,
    ) -> bool {
        if self.busy()
            || position.is_some_and(|p| !valid_position(p))
            || !self.can_create_note(parent)
        {
            return false;
        }
        let mut note = Note::new();
        note.canvas.position = position;
        note.canvas.parent = parent;
        note.canvas.project = project;
        self.operation = Some((Operation::Create(note), false));
        true
    }
    pub fn request_delete(&mut self, id: Uuid) -> bool {
        if self.busy() || self.note(id).is_none() {
            return false;
        }
        self.operation = Some((Operation::Delete(id), false));
        true
    }
    fn dispatch(&mut self) {
        self.queue.dispatch(&self.notes);
        self.changed_at = None;
        self.first_change = None;
    }
    pub fn flush(&mut self) {
        self.queue.retry();
        self.error = None;
        self.flush_requested = true;
        if !self.busy() {
            self.dispatch();
        }
    }
    pub fn needs_tick(&self) -> bool {
        self.busy() || self.queue.in_flight() || (!self.queue.failed() && !self.queue.clean())
    }
    pub fn tick(&mut self, now: Instant) -> Vec<NotebookEvent> {
        let mut events = Vec::new();
        if let Some((transaction, result)) = self.queue.poll() {
            if transaction {
                let operation = self.operation.take().map(|pending| pending.0);
                match result {
                    Ok(()) => {
                        if let Some(operation) = operation {
                            let event = match &operation {
                                Operation::Create(note) => NotebookEvent::Created(note.id),
                                Operation::Delete(id) => NotebookEvent::Deleted(*id),
                            };
                            operation.apply(&mut self.notes);
                            events.push(event);
                        }
                        self.changed(now);
                        self.dispatch();
                    }
                    Err(message) => {
                        self.queue.changed();
                        self.queue.record_failure();
                        self.flush_requested = false;
                        let error = SaveError(message);
                        self.error = Some(error.clone());
                        events.push(NotebookEvent::OperationFailed(error));
                    }
                }
            } else {
                match result {
                    Ok(()) => {
                        self.error = None;
                        if self.queue.clean() && !self.busy() {
                            self.flush_requested = false;
                            events.push(NotebookEvent::Saved);
                        } else if !self.busy() {
                            self.dispatch();
                        }
                    }
                    Err(message) => {
                        self.flush_requested = false;
                        let error = SaveError(message);
                        self.error = Some(error.clone());
                        events.push(NotebookEvent::SaveFailed(error));
                    }
                }
            }
        }
        if !self.queue.in_flight() && self.operation.as_ref().is_some_and(|(_, active)| !active) {
            let mut notes = self.notes.clone();
            let pending = self.operation.as_mut().unwrap();
            pending.0.apply(&mut notes);
            pending.1 = true;
            self.queue.transaction(&notes);
        }
        if !self.busy()
            && !self.queue.failed()
            && (self.flush_requested
                || self.changed_at.is_some_and(|at| {
                    now.saturating_duration_since(at) >= Duration::from_millis(300)
                })
                || self
                    .first_change
                    .is_some_and(|at| now.saturating_duration_since(at) >= Duration::from_secs(1)))
        {
            self.dispatch();
        }
        events
    }
}

#[cfg(test)]
mod tests;
