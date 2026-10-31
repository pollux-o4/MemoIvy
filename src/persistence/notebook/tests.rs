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
