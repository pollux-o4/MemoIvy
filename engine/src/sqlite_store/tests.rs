use super::legacy::backup_path;
use super::*;
use crate::domain::hash;
use crate::sqlite_store::Engine;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use uuid::Uuid;
fn open() -> (tempfile::TempDir, Engine) {
    let d = tempfile::tempdir().unwrap();
    let e = Engine::open(d.path(), None).unwrap();
    (d, e)
}
fn req(e: &mut Engine, op: &str, p: Value) -> ApiResult {
    e.dispatch(op, &p)
}
#[test]
fn create_conflict_and_reopen_persist_committed_revision() {
    let (d, mut e) = open();
    let s = e.snapshot().unwrap();
    let out = req(
        &mut e,
        "create",
        json!({"baseRevision":s.revision,"text":"persist me"}),
    )
    .unwrap();
    assert_eq!(out["snapshot"]["revision"], 1);
    let note_id = out["note"]["id"].as_str().unwrap();
    let conflict = req(
        &mut e,
        "update",
        json!({"baseRevision":0,"id":note_id,"text":"stale"}),
    )
    .unwrap_err();
    assert_eq!(conflict.code, "CONFLICT");
    assert!(conflict.snapshot.is_some());
    drop(e);
    let e = Engine::open(d.path(), None).unwrap();
    let s = e.snapshot().unwrap();
    assert_eq!(s.revision, 1);
    assert_eq!(s.notes.len(), 1);
    assert_eq!(s.notes[0].text, "persist me");
}
#[test]
fn unrelated_note_and_preference_writes_do_not_conflict() {
    let (_d, mut e) = open();
    let a = req(&mut e, "create", json!({"baseRevision":0,"text":"a"})).unwrap();
    let a_id = a["note"]["id"].as_str().unwrap().to_owned();
    let b = req(&mut e, "create", json!({"baseRevision":1,"text":"b"})).unwrap();
    let b_id = b["note"]["id"].as_str().unwrap().to_owned();
    req(
        &mut e,
        "preference",
        json!({"baseRevision":2,"key":"autoPrune","value":true}),
    )
    .unwrap();
    let updated = req(
        &mut e,
        "update",
        json!({"baseRevision":1,"id":a_id,"text":"a updated"}),
    )
    .unwrap();
    assert_eq!(updated["note"]["text"], "a updated");
    let other_pref = req(
        &mut e,
        "preference",
        json!({"baseRevision":2,"key":"expandedNotes","value":[b_id]}),
    )
    .unwrap();
    assert_eq!(
        other_pref["snapshot"]["preferences"]["expandedNotes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let stale_pref = req(
        &mut e,
        "preference",
        json!({"baseRevision":2,"key":"autoPrune","value":false}),
    )
    .unwrap_err();
    assert_eq!(stale_pref.code, "CONFLICT");
    assert!(stale_pref.snapshot.is_some());
    let stale = req(
        &mut e,
        "update",
        json!({"baseRevision":1,"id":a_id,"text":"lost draft"}),
    )
    .unwrap_err();
    assert_eq!(stale.code, "CONFLICT");
    assert!(stale.snapshot.is_some());
}
#[test]
fn schema_v1_migrates_existing_note_revision_conservatively() {
    let (d, mut e) = open();
    let out = req(&mut e, "create", json!({"baseRevision":0,"text":"stored"})).unwrap();
    let id = out["note"]["id"].as_str().unwrap().to_owned();
    e.db.execute_batch(
        "DROP TABLE note_revisions; DROP TABLE preference_revisions; PRAGMA user_version=1;",
    )
    .unwrap();
    drop(e);
    let e = Engine::open(d.path(), None).unwrap();
    let s = e.snapshot().unwrap();
    assert_eq!(s.note_revisions[&id], 1);
    assert_eq!(
        e.db.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
            .unwrap(),
        2
    );
}
#[test]
fn legacy_import_backup_and_invalid_import_preserve_source() {
    let (d, mut e) = open();
    let legacy = d.path().join("notes.json");
    let id = Uuid::new_v4();
    let source = format!(r#"[{{"id":"{id}","text":"old","color":"yellow","modified":1}}]"#);
    fs::write(&legacy, &source).unwrap();
    let s = e.import_legacy(&legacy).unwrap();
    assert_eq!(s.notes.len(), 1);
    assert_eq!(fs::read(backup_path(&legacy)).unwrap(), source.as_bytes());
    assert_eq!(fs::read(&legacy).unwrap(), source.as_bytes());
    assert_eq!(
        e.import_legacy(&legacy).unwrap_err().code,
        "IMPORT_REJECTED"
    );
}
#[test]
fn legacy_import_auto_and_manual_preserve_notes_attachments_and_preferences_on_reopen() {
    let d = tempfile::tempdir().unwrap();
    let legacy = d.path().join("notes.json");
    let note_id = Uuid::new_v4();
    let bytes = b"\x89PNG\r\n\x1a\nsynthetic";
    let id = hash(bytes);
    let text = format!("![x](asset:{id})");
    let source =
        format!(r#"[{{"id":"{note_id}","text":"{text}","color":"yellow","modified":42}}]"#);
    fs::write(&legacy, &source).unwrap();
    fs::create_dir(d.path().join("assets")).unwrap();
    let attachment = d.path().join("assets").join(format!("{id}.png"));
    fs::write(&attachment, bytes).unwrap();
    fs::write(
        d.path().join("assets.json"),
        format!(
            r#"{{"auto_prune":true,"assets":{{"{id}":{{"added":11,"last_note":"{note_id}"}}}}}}"#
        ),
    )
    .unwrap();
    fs::write(d.path().join("library.json"), format!(r#"["{note_id}"]"#)).unwrap();
    fs::write(
        d.path().join("navigation.json"),
        format!(r#"{{"{note_id}":{{"count":3,"last":12}}}}"#),
    )
    .unwrap();
    for auto_import in [true, false] {
        let data = tempfile::tempdir().unwrap();
        let discovery = auto_import.then(|| legacy.clone());
        let mut e = Engine::open(data.path(), discovery.clone()).unwrap();
        if !auto_import {
            let result = req(&mut e, "importLegacy", json!({"path":legacy})).unwrap();
            assert_eq!(result["snapshot"]["notes"][0]["text"], text);
        }
        assert_eq!(e.snapshot().unwrap().revision, 1);
        drop(e);

        let mut e = Engine::open(data.path(), discovery).unwrap();
        let s = e.snapshot().unwrap();
        assert_eq!(s.revision, 1);
        assert_eq!(s.notes[0].text, text);
        assert_eq!(s.assets.len(), 1);
        assert_eq!(s.assets[0].added, 11);
        assert_eq!(s.assets[0].used_by, vec![note_id.to_string()]);
        assert_eq!(s.preferences["autoPrune"], true);
        assert_eq!(s.preferences["expandedProjects"][0], note_id.to_string());
        assert_eq!(
            s.preferences["navigation"][note_id.to_string()]["lastVisited"],
            12
        );
        let asset = req(&mut e, "readAsset", json!({"id":id})).unwrap();
        assert_eq!(
            asset,
            json!({"mime":"image/png", "data":STANDARD.encode(bytes)})
        );
        assert_eq!(fs::read(&legacy).unwrap(), source.as_bytes());
        assert_eq!(fs::read(backup_path(&legacy)).unwrap(), source.as_bytes());
        assert_eq!(fs::read(&attachment).unwrap(), bytes);
    }
}
#[test]
fn failed_legacy_transaction_keeps_empty_database_and_verified_backup() {
    let (d, mut e) = open();
    let legacy = d.path().join("notes.json");
    let id = Uuid::new_v4();
    let source =
        format!(r#"[{{"id":"{id}","text":"fail storage","color":"yellow","modified":1}}]"#);
    fs::write(&legacy, &source).unwrap();
    e.db.execute_batch("CREATE TRIGGER simulated_migration_failure BEFORE INSERT ON notes BEGIN SELECT RAISE(ABORT,'simulated migration failure'); END;").unwrap();
    assert!(e.import_legacy(&legacy).is_err());
    assert_eq!(e.snapshot().unwrap().revision, 0);
    assert!(e.snapshot().unwrap().notes.is_empty());
    assert_eq!(fs::read(&legacy).unwrap(), source.as_bytes());
    assert_eq!(fs::read(backup_path(&legacy)).unwrap(), source.as_bytes());
}
#[test]
fn legacy_import_retries_wait_for_source_lock_and_keep_it_until_close() {
    let data = tempfile::tempdir().unwrap();
    let source_dir = tempfile::tempdir().unwrap();
    let legacy = source_dir.path().join("notes.json");
    fs::write(&legacy, "[]").unwrap();
    let writer = Engine::open(source_dir.path(), None).unwrap();
    let mut e = Engine::open(data.path(), None).unwrap();
    for _ in 0..2 {
        assert_eq!(e.import_legacy(&legacy).unwrap_err().code, "LOCKED");
    }
    assert!(!backup_path(&legacy).exists());
    drop(writer);

    e.import_legacy(&legacy).unwrap();
    assert!(Engine::open(source_dir.path(), None).is_err());
}
#[test]
fn failed_validation_rolls_back_notes_and_revision() {
    let (_d, mut e) = open();
    let s = e.snapshot().unwrap();
    let id = Uuid::new_v4().to_string();
    let bad = req(
        &mut e,
        "create",
        json!({"baseRevision":s.revision,"parent":id}),
    )
    .unwrap_err();
    assert_eq!(bad.code, "INVALID_DATA");
    let s = e.snapshot().unwrap();
    assert_eq!(s.revision, 0);
    assert!(s.notes.is_empty());
}
#[test]
fn active_writer_lock_blocks_second_engine() {
    let (d, _e) = open();
    assert!(Engine::open(d.path(), None).is_err());
}
#[test]
fn sqlite_write_failure_rolls_back_deleted_rows_and_revision() {
    let (_d, mut e) = open();
    let base = e.snapshot().unwrap().revision;
    req(
        &mut e,
        "create",
        json!({"baseRevision":base,"text":"keep me"}),
    )
    .unwrap();
    e.db.execute_batch("CREATE TRIGGER simulated_storage_failure BEFORE INSERT ON notes WHEN json_extract(NEW.body,'$.text')='fail storage' BEGIN SELECT RAISE(ABORT,'simulated storage failure'); END;").unwrap();
    let revision = e.snapshot().unwrap().revision;
    assert!(req(
        &mut e,
        "create",
        json!({"baseRevision":revision,"text":"fail storage"})
    )
    .is_err());
    let s = e.snapshot().unwrap();
    assert_eq!(s.revision, revision);
    assert_eq!(s.notes.len(), 1);
    assert_eq!(s.notes[0].text, "keep me");
}
#[test]
fn preference_updates_are_revisioned_and_bounded() {
    let (_d, mut e) = open();
    let id = Uuid::new_v4().to_string();
    let s = e.snapshot().unwrap();
    let out = req(
        &mut e,
        "preference",
        json!({"baseRevision":s.revision,"key":"expandedProjects","value":[id]}),
    )
    .unwrap();
    assert_eq!(out["snapshot"]["revision"], 1);
    assert_eq!(out["snapshot"]["preferences"]["expandedProjects"][0], id);
    let bad = req(
        &mut e,
        "preference",
        json!({"baseRevision":1,"key":"unknown","value":true}),
    );
    assert_eq!(bad.unwrap_err().code, "INVALID_PARAMS");
}
#[test]
fn canvas_link_style_preference_accepts_known_shapes_and_rejects_others() {
    let (_d, mut e) = open();
    let saved = req(
        &mut e,
        "preference",
        json!({"baseRevision":0,"key":"canvasLinkStyle","value":"straight"}),
    )
    .unwrap();
    assert_eq!(
        saved["snapshot"]["preferences"]["canvasLinkStyle"],
        "straight"
    );
    let error = req(
        &mut e,
        "preference",
        json!({"baseRevision":1,"key":"canvasLinkStyle","value":"zigzag"}),
    )
    .unwrap_err();
    assert_eq!(error.code, "INVALID_PARAMS");
}
#[test]
fn appearance_preference_accepts_known_choices_and_rejects_others() {
    let (_d, mut e) = open();
    let saved = req(
        &mut e,
        "preference",
        json!({"baseRevision":0,"key":"appearance","value":{"theme":"dark","motion":"reduce"}}),
    )
    .unwrap();
    assert_eq!(
        saved["snapshot"]["preferences"]["appearance"]["theme"],
        "dark"
    );
    for bad in [
        json!({"theme":"sepia","motion":"system"}),
        json!({"theme":"dark"}),
        json!({"theme":"dark","motion":"system","extra":1}),
    ] {
        let error = req(
            &mut e,
            "preference",
            json!({"baseRevision":1,"key":"appearance","value":bad}),
        )
        .unwrap_err();
        assert_eq!(error.code, "INVALID_PARAMS");
    }
}

#[test]
fn deleted_note_is_absent_from_result_reload_and_reopen() {
    let (d, mut e) = open();
    let created = req(
        &mut e,
        "create",
        json!({"baseRevision":0,"text":"delete me"}),
    )
    .unwrap();
    let id = created["note"]["id"].clone();
    let deleted = req(&mut e, "delete", json!({"baseRevision":1,"id":id})).unwrap();
    assert_eq!(deleted["snapshot"]["notes"], json!([]));
    assert_eq!(deleted["snapshot"]["revision"], 2);
    assert_eq!(
        req(&mut e, "snapshot", json!({})).unwrap()["snapshot"],
        deleted["snapshot"]
    );
    drop(e);
    let mut reopened = Engine::open(d.path(), None).unwrap();
    assert_eq!(
        req(&mut reopened, "snapshot", json!({})).unwrap()["snapshot"],
        deleted["snapshot"]
    );
}
#[test]
fn restore_brings_back_deleted_note_with_links_and_children() {
    let (_d, mut e) = open();
    let make = |e: &mut Engine, extra: Value| {
        let rev = e.snapshot().unwrap().revision;
        let mut p = json!({"baseRevision":rev});
        p.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        req(e, "create", p).unwrap()["note"].clone()
    };
    let parent = make(&mut e, json!({"text":"p","project":true}));
    let pid = parent["id"].as_str().unwrap().to_owned();
    let target = make(&mut e, json!({"text":"t","parent":pid}));
    let tid = target["id"].as_str().unwrap().to_owned();
    let source = make(&mut e, json!({"text":"s"}));
    let sid = source["id"].as_str().unwrap().to_owned();
    let rev = e.snapshot().unwrap().revision;
    req(
        &mut e,
        "canvas",
        json!({"baseRevision":rev,"changes":[{"id":sid,"canvas":{"next":[pid]}}]}),
    )
    .unwrap();
    let before = e.snapshot().unwrap();
    let deleted: Value =
        serde_json::to_value(before.notes.iter().find(|n| n.id == pid).unwrap()).unwrap();
    req(
        &mut e,
        "delete",
        json!({"baseRevision":before.revision,"id":pid}),
    )
    .unwrap();
    let rev = e.snapshot().unwrap().revision;
    let out = req(
        &mut e,
        "restore",
        json!({"baseRevision":rev,"notes":[deleted],"changes":[
        {"id":sid,"canvas":{"next":[pid]}},{"id":tid,"canvas":{"parent":pid}}]}),
    )
    .unwrap();
    let notes = out["snapshot"]["notes"].as_array().unwrap();
    let find = |id: &str| notes.iter().find(|n| n["id"] == id).unwrap().clone();
    assert_eq!(find(&pid)["text"], "p");
    assert_eq!(find(&sid)["canvas"]["next"], json!([pid]));
    assert_eq!(find(&tid)["canvas"]["parent"], json!(pid));
    let rev = out["snapshot"]["revision"].as_u64().unwrap();
    let again = req(
        &mut e,
        "restore",
        json!({"baseRevision":rev,"notes":[find(&pid)],"changes":[]}),
    )
    .unwrap_err();
    assert_eq!(again.code, "CONFLICT");
}
#[test]
fn batch_delete_lifts_children_past_removed_ancestors() {
    let (_d, mut e) = open();
    let mut make = |parent: Option<&str>, project: bool| {
        let rev = e.snapshot().unwrap().revision;
        let out = req(
            &mut e,
            "create",
            json!({"baseRevision":rev,"parent":parent,"project":project}),
        )
        .unwrap();
        out["note"]["id"].as_str().unwrap().to_owned()
    };
    let top = make(None, true);
    let mid = make(Some(&top), true);
    let leaf = make(Some(&mid), false);
    let rev = e.snapshot().unwrap().revision;
    let out = req(
        &mut e,
        "delete",
        json!({"baseRevision":rev,"ids":[mid, top]}),
    )
    .unwrap();
    let notes = out["snapshot"]["notes"].as_array().unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0]["id"], json!(leaf));
    assert!(notes[0]["canvas"].get("parent").is_none());
    let rev = out["snapshot"]["revision"].as_u64().unwrap();
    let missing = req(&mut e, "delete", json!({"baseRevision":rev,"ids":[top]})).unwrap_err();
    assert_eq!(missing.code, "NOT_FOUND");
}
