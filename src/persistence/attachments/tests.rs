use super::*;
use files::MAX_BYTES;
use std::os::unix::fs::PermissionsExt;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

fn note(text: &str, modified: u64) -> Note {
    let mut note = Note::new();
    note.text = text.into();
    note.modified = modified;
    note
}

fn open() -> (tempfile::TempDir, Attachments) {
    let dir = tempfile::tempdir().unwrap();
    let attachments = Attachments::beside(&dir.path().join("notes.json"));
    (dir, attachments)
}

#[test]
fn data_003_import_sniffs_format_dedupes_and_is_private() {
    let (dir, attachments) = open();
    let store = attachments.store();
    let id = store.import(PNG).unwrap();
    assert_eq!(store.import(PNG).unwrap(), id);
    let path = store.path(&id).unwrap();
    assert_eq!(path, dir.path().join(format!("assets/{id}.png")));
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    assert_eq!(fs::read_dir(dir.path().join("assets")).unwrap().count(), 1);
    assert!(store.import(b"<svg/>").is_err());
    assert!(store.import(&vec![0; MAX_BYTES as usize + 1]).is_err());
    let jpeg = store.import(&[0xff, 0xd8, 0xff, 0xe0]).unwrap();
    assert!(store.path(&jpeg).unwrap().ends_with(format!("{jpeg}.jpg")));
    let outside = dir.path().join("shot.png");
    fs::write(&outside, b"\x89PNG\r\n\x1a\nother").unwrap();
    let copied = store.import_file(&outside).unwrap();
    fs::remove_file(&outside).unwrap();
    assert!(
        store.path(&copied).is_some(),
        "원본을 지워도 복사본은 남는다"
    );
}

#[test]
fn data_003_usage_follows_text_and_remembers_last_note() {
    let (_dir, mut attachments) = open();
    let id = attachments.store().import(PNG).unwrap();
    let reference = format!("![|320]({})", id.target());
    let old = note(&reference, 1);
    let recent = note(&format!("앞 {} 깨진 문법", id.target()), 5);
    let mut notes = vec![old.clone(), recent.clone()];
    assert!(attachments.reconcile(&notes, 100));
    let entry = &attachments.entries(&notes)[0];
    assert_eq!(entry.users, [recent.id, old.id]);
    assert_eq!(entry.unused_since, None);

    notes.retain(|n| n.id == old.id);
    notes[0].text = "지움".into();
    assert!(attachments.reconcile(&notes, 200));
    assert!(
        !attachments.reconcile(&notes, 300),
        "같은 상태면 다시 쓰지 않는다"
    );
    let entry = &attachments.entries(&notes)[0];
    assert!(entry.users.is_empty());
    assert_eq!(
        entry.last_note,
        Some(recent.id),
        "가장 최근에 고친 사용처를 기억한다"
    );
    assert_eq!(entry.unused_since, Some(200));
}

#[test]
fn data_003_delete_refuses_used_files() {
    let (_dir, mut attachments) = open();
    let store = attachments.store();
    let used = store.import(PNG).unwrap();
    let unused = store.import(b"GIF89a....").unwrap();
    let notes = vec![note(&used.target(), 1)];
    attachments.reconcile(&notes, 10);
    assert!(attachments.delete(&used, &notes).is_err());
    assert!(store.path(&used).is_some());
    assert_eq!(attachments.delete_unused(&notes), Ok(1));
    assert!(store.path(&unused).is_none());
    assert_eq!(attachments.entries(&notes).len(), 1);
}

#[test]
fn data_003_corrupt_index_rebuilds_and_prune_is_opt_in() {
    let dir = tempfile::tempdir().unwrap();
    let notes_path = dir.path().join("notes.json");
    fs::write(dir.path().join("assets.json"), b"{broken").unwrap();
    let mut attachments = Attachments::beside(&notes_path);
    let id = attachments.store().import(PNG).unwrap();
    attachments.reconcile(&[], 0);
    assert_eq!(attachments.prune(&[], PRUNE_AFTER), 0, "기본은 꺼짐");
    attachments.set_auto_prune(true);
    let mut reopened = Attachments::beside(&notes_path);
    assert!(reopened.auto_prune());
    assert_eq!(reopened.prune(&[], PRUNE_AFTER - 1), 0);
    assert_eq!(reopened.prune(&[], PRUNE_AFTER), 1);
    assert!(reopened.store().path(&id).is_none());
}
