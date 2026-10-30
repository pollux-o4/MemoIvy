use super::{
    canvas::CanvasNode,
    links,
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
        let changed = canvas.changed;
        if changed {
            self.changed(Instant::now());
        }
        changed
    }
    pub fn request_create(&mut self, position: Option<[f64; 2]>) -> bool {
        if self.busy() || position.is_some_and(|p| !valid_position(p)) {
            return false;
        }
        let mut note = Note::new();
        note.canvas.position = position;
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
                        match operation {
                            Some(Operation::Create(note)) => {
                                let id = note.id;
                                self.notes.push(note);
                                events.push(NotebookEvent::Created(id));
                            }
                            Some(Operation::Delete(id)) => {
                                links::remove(&mut self.notes, id);
                                events.push(NotebookEvent::Deleted(id));
                            }
                            None => {}
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
            match &self.operation.as_ref().unwrap().0 {
                Operation::Create(note) => notes.push(note.clone()),
                Operation::Delete(id) => links::remove(&mut notes, *id),
            }
            self.operation.as_mut().unwrap().1 = true;
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
fn valid_position(p: [f64; 2]) -> bool {
    p.iter()
        .all(|v| v.is_finite() && (0. ..=super::canvas::LIMIT).contains(v))
}
pub struct CanvasEdit<'a> {
    notes: &'a mut Vec<Note>,
    changed: bool,
}
impl CanvasEdit<'_> {
    pub fn set_position(&mut self, id: Uuid, position: [f64; 2]) {
        if !valid_position(position) {
            return;
        }
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            if note.canvas.position != Some(position) {
                note.canvas.position = Some(position);
                self.changed = true;
            }
        }
    }
    pub fn connect(&mut self, from: Uuid, to: Uuid) -> bool {
        let changed = links::connect(self.notes, from, to);
        self.changed |= changed;
        changed
    }
    pub fn disconnect(&mut self, from: Uuid, to: Uuid) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == from) {
            let len = note.canvas.next.len();
            note.canvas.next.retain(|id| *id != to);
            self.changed |= len != note.canvas.next.len();
        }
    }
    pub fn set_done(&mut self, id: Uuid, done: bool) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            self.changed |= note.canvas.done != done;
            note.canvas.done = done;
        }
    }
    pub fn restore(&mut self, snapshot: &[(Uuid, CanvasNode)]) {
        let ids: std::collections::HashSet<_> = self.notes.iter().map(|n| n.id).collect();
        for note in self.notes.iter_mut() {
            if let Some((_, node)) = snapshot.iter().find(|(id, _)| *id == note.id) {
                if node.position.is_some_and(|p| !valid_position(p)) {
                    continue;
                }
                let mut node = node.clone();
                let mut seen = std::collections::HashSet::new();
                node.next
                    .retain(|id| *id != note.id && ids.contains(id) && seen.insert(*id));
                self.changed |= note.canvas != node;
                note.canvas = node;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn notebook() -> (tempfile::TempDir, Notebook) {
        let dir = tempfile::tempdir().unwrap();
        let book = Notebook::open(dir.path().join("notes.json")).unwrap();
        (dir, book)
    }
    fn settle(book: &mut Notebook) -> Vec<NotebookEvent> {
        let end = Instant::now() + Duration::from_secs(5);
        let mut events = Vec::new();
        while book.needs_tick() {
            events.extend(book.tick(Instant::now() + Duration::from_secs(2)));
            assert!(Instant::now() < end, "writer timed out");
            std::thread::sleep(Duration::from_millis(1));
        }
        events
    }
    fn create(book: &mut Notebook) -> Uuid {
        assert!(book.request_create(Some([64., 64.])));
        settle(book)
            .into_iter()
            .find_map(|event| match event {
                NotebookEvent::Created(id) => Some(id),
                _ => None,
            })
            .unwrap()
    }
    #[test]
    fn save_001_debounce_boundaries_and_one_second_cap() {
        let (_dir, mut book) = notebook();
        let id = create(&mut book);
        let start = Instant::now();
        assert!(book.set_text(id, "a".into()));
        book.changed_at = Some(start);
        book.first_change = Some(start);
        book.tick(start + Duration::from_millis(299));
        assert_eq!(book.status(), SaveStatus::Pending);
        book.tick(start + Duration::from_millis(300));
        assert_eq!(book.status(), SaveStatus::Saving);
        settle(&mut book);
        assert!(book.set_text(id, "b".into()));
        book.changed_at = Some(start + Duration::from_millis(900));
        book.first_change = Some(start);
        book.tick(start + Duration::from_millis(999));
        assert_eq!(book.status(), SaveStatus::Pending);
        book.tick(start + Duration::from_secs(1));
        assert_eq!(book.status(), SaveStatus::Saving);
        settle(&mut book);
    }
    #[test]
    fn note_002_pending_delete_merges_latest_other_edit() {
        let (dir, mut book) = notebook();
        let deleted = create(&mut book);
        let other = create(&mut book);
        assert!(book.request_delete(deleted));
        assert!(!book.request_create(None));
        book.tick(Instant::now());
        assert!(book.set_text(other, "동시 입력".into()));
        book.flush();
        settle(&mut book);
        let loaded = Notebook::open(dir.path().join("notes.json")).unwrap();
        assert!(loaded.note(deleted).is_none());
        assert_eq!(loaded.note(other).unwrap().text(), "동시 입력");
        assert_eq!(book.status(), SaveStatus::Saved);
    }
    #[test]
    fn save_002_failed_delete_preserves_note_and_retry_restores_file() {
        let (dir, mut book) = notebook();
        let id = create(&mut book);
        let path = dir.path().join("notes.json");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(book.request_delete(id));
        let events = settle(&mut book);
        assert!(events
            .iter()
            .any(|e| matches!(e, NotebookEvent::OperationFailed(_))));
        assert!(book.note(id).is_some());
        assert!(!book.busy());
        assert!(matches!(book.status(), SaveStatus::Failed(_)));
        assert!(!book.needs_tick());
        std::fs::remove_dir(&path).unwrap();
        book.flush();
        settle(&mut book);
        assert_eq!(book.status(), SaveStatus::Saved);
        assert!(Notebook::open(path).unwrap().note(id).is_some());
    }
    #[test]
    fn save_002_snapshot_failure_is_idle_until_explicit_retry() {
        let (dir, mut book) = notebook();
        let id = create(&mut book);
        let path = dir.path().join("notes.json");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        book.set_text(id, "재시도할 내용".into());
        book.flush();
        let events = settle(&mut book);
        assert!(events
            .iter()
            .any(|e| matches!(e, NotebookEvent::SaveFailed(_))));
        assert!(!book.needs_tick());
        std::fs::remove_dir(&path).unwrap();
        book.flush();
        assert!(settle(&mut book)
            .iter()
            .any(|e| matches!(e, NotebookEvent::Saved)));
        assert_eq!(
            Notebook::open(path).unwrap().note(id).unwrap().text(),
            "재시도할 내용"
        );
    }
    #[test]
    fn save_002_new_edit_after_failed_close_keeps_debounce() {
        let (dir, mut book) = notebook();
        let id = create(&mut book);
        let path = dir.path().join("notes.json");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        book.set_text(id, "failed close".into());
        book.flush();
        settle(&mut book);
        std::fs::remove_dir(&path).unwrap();
        book.set_text(id, "new input".into());
        let now = book.changed_at.unwrap();
        book.tick(now + Duration::from_millis(299));
        assert_eq!(book.status(), SaveStatus::Pending);
        book.tick(now + Duration::from_millis(300));
        assert_eq!(book.status(), SaveStatus::Saving);
        settle(&mut book);
    }
    #[test]
    fn note_001_create_is_visible_only_after_transaction_and_preserves_edits() {
        let (dir, mut book) = notebook();
        let existing = create(&mut book);
        assert!(book.request_create(Some([500., 64.])));
        assert_eq!(book.notes().len(), 1);
        book.tick(Instant::now());
        book.set_text(existing, "생성 중 편집".into());
        let events = settle(&mut book);
        let created = events
            .into_iter()
            .find_map(|e| match e {
                NotebookEvent::Created(id) => Some(id),
                _ => None,
            })
            .unwrap();
        let loaded = Notebook::open(dir.path().join("notes.json")).unwrap();
        assert_eq!(loaded.notes().len(), 2);
        assert_eq!(loaded.note(existing).unwrap().text(), "생성 중 편집");
        assert_eq!(
            loaded.note(created).unwrap().canvas().position,
            Some([500., 64.])
        );
    }
    #[test]
    fn canvas_001_edits_validate_and_restore_only_surviving_links() {
        let (_dir, mut book) = notebook();
        let a = create(&mut book);
        let b = create(&mut book);
        assert!(!book.set_color(a, "invalid"));
        assert!(!book.edit_canvas(|edit| edit.set_position(a, [f64::NAN, 0.])));
        assert!(book.edit_canvas(|edit| {
            assert!(edit.connect(a, b));
            edit.set_done(a, true);
        }));
        let snapshot: Vec<_> = book
            .notes()
            .iter()
            .map(|n| (n.id(), n.canvas().clone()))
            .collect();
        assert!(book.request_delete(b));
        settle(&mut book);
        book.edit_canvas(|edit| edit.restore(&snapshot));
        assert!(book.note(a).unwrap().canvas().next.is_empty());
        assert!(book.note(a).unwrap().canvas().done);
        settle(&mut book);
    }
}
