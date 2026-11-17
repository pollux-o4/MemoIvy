use crate::commands::{notes_tx, reconcile_asset_usage};
use crate::domain::*;
use crate::error::{err, internal, ApiError, ApiResult};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
pub(crate) struct Engine {
    pub(crate) db: Connection,
    _locks: Vec<File>,
    lock_paths: HashSet<PathBuf>,
    legacy: Option<PathBuf>,
}
impl Engine {
    pub(crate) fn open(dir: &Path, legacy: Option<PathBuf>) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
        let lock_path = fs::canonicalize(dir)
            .map_err(|e| e.to_string())?
            .join(".memoivy.lock");
        let lock = lock_file(&lock_path)?;
        let mut locks = vec![lock];
        let mut lock_paths = HashSet::from([lock_path.clone()]);
        if let Some(p) = &legacy {
            let lp = lock_path_for(p);
            if lock_paths.insert(lp.clone()) {
                locks.push(lock_file(&lp)?);
            }
        }
        let mut db = Connection::open(dir.join("memoivy.sqlite3")).map_err(|e| e.to_string())?;
        let schema: i64 = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if schema > 2 {
            return Err(format!(
                "database schema {schema} is newer than supported schema 2"
            ));
        }
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
   CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
   INSERT OR IGNORE INTO meta(key,value) VALUES ('revision',0);
   CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY, body TEXT NOT NULL);
   CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, name TEXT NOT NULL, mime TEXT NOT NULL, data BLOB NOT NULL, added INTEGER NOT NULL, last_note TEXT, unused_since INTEGER);
   CREATE TABLE IF NOT EXISTS preferences (key TEXT PRIMARY KEY, body TEXT NOT NULL);
   CREATE TABLE IF NOT EXISTS note_revisions (id TEXT PRIMARY KEY, revision INTEGER NOT NULL);
   CREATE TABLE IF NOT EXISTS preference_revisions (key TEXT PRIMARY KEY, revision INTEGER NOT NULL);").map_err(|e|e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        let revision: i64 = tx
            .query_row("SELECT value FROM meta WHERE key='revision'", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT OR IGNORE INTO note_revisions(id,revision) SELECT id,?1 FROM notes",
            [revision],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("INSERT OR IGNORE INTO preference_revisions(key,revision) SELECT key,?1 FROM preferences",[revision]).map_err(|e|e.to_string())?;
        tx.pragma_update(None, "user_version", 2)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::set_permissions(
            dir.join("memoivy.sqlite3"),
            fs::Permissions::from_mode(0o600),
        )
        .map_err(|e| e.to_string())?;
        let mut s = Self {
            db,
            _locks: locks,
            lock_paths,
            legacy,
        };
        let imported: i64 =
            s.db.query_row(
                "SELECT value FROM meta WHERE key='legacy_imported'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or(0);
        if imported == 0 && s.empty_for_import()? {
            if let Some(p) = s.legacy.clone() {
                if p.exists() {
                    let _ = s.import_legacy(&p).map_err(|e| e.message)?;
                }
            }
        }
        s.prune_expired().map_err(|e| e.message)?;
        Ok(s)
    }
    fn count_notes(&self) -> Result<usize, String> {
        self.db
            .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }
    fn empty_for_import(&self) -> Result<bool, String> {
        let revision = self.revision()?;
        let notes = self.count_notes()?;
        let assets: i64 = self
            .db
            .query_row("SELECT count(*) FROM assets", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let prefs: i64 = self
            .db
            .query_row("SELECT count(*) FROM preferences", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(revision == 0 && notes == 0 && assets == 0 && prefs == 0)
    }
    fn revision(&self) -> Result<u64, String> {
        self.db
            .query_row("SELECT value FROM meta WHERE key='revision'", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())
    }
    pub(crate) fn snapshot(&self) -> Result<Snapshot, String> {
        let mut st = self
            .db
            .prepare("SELECT body FROM notes ORDER BY rowid")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut notes = Vec::new();
        for row in rows {
            notes.push(
                serde_json::from_str(&row.map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            )
        }
        let assets = self.assets(&notes)?;
        let revision = self.revision()?;
        let preferences = self.preferences()?;
        let note_revisions = {
            let mut q = self
                .db
                .prepare(
                    "SELECT nr.id,nr.revision FROM note_revisions nr JOIN notes n ON n.id=nr.id",
                )
                .map_err(|e| e.to_string())?;
            let rows = q
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<std::collections::BTreeMap<_, _>, _>>()
                .map_err(|e| e.to_string())?
        };
        Ok(Snapshot {
            notes,
            assets,
            revision,
            saved_revision: revision,
            preferences,
            note_revisions,
        })
    }
    fn preferences(&self) -> Result<Value, String> {
        let mut out =
            json!({"expandedProjects":[],"expandedNotes":[],"navigation":{},"autoPrune":false});
        let mut q = self
            .db
            .prepare("SELECT key,body FROM preferences")
            .map_err(|e| e.to_string())?;
        let rows = q
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (k, v) = row.map_err(|e| e.to_string())?;
            if let Some(dst) = out.get_mut(&k) {
                *dst = serde_json::from_str(&v).map_err(|e| e.to_string())?
            }
        }
        Ok(out)
    }
    fn assets(&self, notes: &[Note]) -> Result<Vec<Asset>, String> {
        let mut s=self.db.prepare("SELECT id,name,mime,length(data),added,last_note,unused_since FROM assets ORDER BY name,id").map_err(|e|e.to_string())?;
        let rows = s
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, usize>(3)?,
                    r.get::<_, u64>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<u64>>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = vec![];
        for row in rows {
            let (id, name, mime, bytes, added, last_note, unused_since) =
                row.map_err(|e| e.to_string())?;
            let used_by = notes
                .iter()
                .filter(|n| n.text.contains(&format!("asset:{id}")))
                .map(|n| n.id.clone())
                .collect();
            out.push(Asset {
                id,
                name,
                mime,
                bytes,
                used_by,
                added,
                last_note,
                unused_since,
            })
        }
        Ok(out)
    }
    pub(crate) fn response_snapshot(&self) -> ApiResult {
        self.snapshot()
            .map(|s| json!({"snapshot":s}))
            .map_err(internal)
    }
    fn prune_expired(&mut self) -> ApiResult {
        let prefs = self.preferences().map_err(internal)?;
        if prefs.get("autoPrune").and_then(Value::as_bool) != Some(true) {
            return Ok(json!({}));
        }
        let cutoff = now().saturating_sub(30 * 24 * 60 * 60);
        let expired: Vec<String> = {
            let mut q = self
                .db
                .prepare(
                    "SELECT id FROM assets WHERE unused_since IS NOT NULL AND unused_since<=?1",
                )
                .map_err(internal)?;
            let rows = q.query_map([cutoff], |r| r.get(0)).map_err(internal)?;
            rows.collect::<Result<_, _>>().map_err(internal)?
        };
        if expired.is_empty() {
            return Ok(json!({}));
        }
        let (_, snapshot) = self.write_tx(None, |tx| {
            let notes = notes_tx(tx)?;
            for id in &expired {
                if notes
                    .iter()
                    .any(|n| n.text.contains(&format!("asset:{id}")))
                {
                    continue;
                }
                tx.execute("DELETE FROM assets WHERE id=?1", [id])
                    .map_err(internal)?;
            }
            Ok(())
        })?;
        Ok(json!({"snapshot":snapshot}))
    }
    pub(crate) fn write_tx<T>(
        &mut self,
        base: Option<u64>,
        op: impl FnOnce(&Transaction<'_>) -> Result<T, ApiError>,
    ) -> Result<(T, Snapshot), ApiError> {
        let current = self.revision().map_err(internal)?;
        if base.is_some_and(|base| base > current) {
            return Err(err(
                "INVALID_PARAMS",
                "baseRevision is newer than the current revision.",
            ));
        }
        let tx = self.db.transaction().map_err(internal)?;
        let before_notes = notes_tx(&tx)?;
        let before_preferences = preference_rows(&tx)?;
        let result = op(&tx)?;
        reconcile_asset_usage(&tx)?;
        let next = current + 1;
        let after_notes = notes_tx(&tx)?;
        let before_by_id: std::collections::HashMap<_, _> =
            before_notes.iter().map(|n| (n.id.as_str(), n)).collect();
        let after_ids: HashSet<_> = after_notes.iter().map(|n| n.id.as_str()).collect();
        for note in &after_notes {
            if before_by_id
                .get(note.id.as_str())
                .is_none_or(|old| **old != *note)
            {
                tx.execute("INSERT INTO note_revisions(id,revision) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision",params![note.id,next]).map_err(internal)?;
            }
        }
        for old in &before_notes {
            if !after_ids.contains(old.id.as_str()) {
                tx.execute("INSERT INTO note_revisions(id,revision) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision",params![old.id,next]).map_err(internal)?;
            }
        }
        let after_preferences = preference_rows(&tx)?;
        for (key, body) in &after_preferences {
            if before_preferences.get(key) != Some(body) {
                tx.execute("INSERT INTO preference_revisions(key,revision) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET revision=excluded.revision",params![key,next]).map_err(internal)?;
            }
        }
        for key in before_preferences.keys() {
            if !after_preferences.contains_key(key) {
                tx.execute("DELETE FROM preference_revisions WHERE key=?1", [key])
                    .map_err(internal)?;
            }
        }
        tx.execute("UPDATE meta SET value=?1 WHERE key='revision'", [next])
            .map_err(internal)?;
        tx.commit().map_err(internal)?;
        let snapshot = self.snapshot().map_err(internal)?;
        Ok((result, snapshot))
    }
    pub(crate) fn import_legacy(&mut self, path: &Path) -> Result<Snapshot, ApiError> {
        let lp = lock_path_for(path);
        if self.lock_paths.insert(lp.clone()) {
            self._locks
                .push(lock_file(&lp).map_err(|e| err("LOCKED", e))?);
        }
        if !self.empty_for_import().map_err(internal)? {
            return Err(err(
                "IMPORT_REJECTED",
                "Legacy import is allowed only into an empty database.",
            ));
        }
        if self
            .db
            .query_row::<i64, _, _>(
                "SELECT value FROM meta WHERE key='legacy_imported'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(internal)?
            .unwrap_or(0)
            != 0
        {
            return Err(err(
                "IMPORT_REJECTED",
                "Legacy data has already been imported.",
            ));
        }
        let metadata = fs::symlink_metadata(path).map_err(|e| err("IO", e.to_string()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(err(
                "INVALID_DATA",
                "Legacy notes path must be a regular file.",
            ));
        }
        let bytes = fs::read(path).map_err(|e| err("IO", e.to_string()))?;
        let mut notes: Vec<Note> =
            serde_json::from_slice(&bytes).map_err(|e| err("INVALID_DATA", e.to_string()))?;
        validate_notes(&notes).map_err(|m| err("INVALID_DATA", m))?;
        promote_projects(&mut notes);
        let backup = backup_path(path);
        if let Ok(meta) = fs::symlink_metadata(&backup) {
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(err("IO", "Legacy backup path must be a regular file."));
            }
        }
        if backup.exists() {
            if fs::read(&backup).map_err(|e| err("IO", e.to_string()))? != bytes {
                return Err(err("IO", "Existing legacy backup differs from the source."));
            }
        } else {
            write_new_synced(&backup, &bytes)
                .map_err(|e| err("IO", format!("Could not create legacy backup: {e}")))?
        }
        let check = fs::read(&backup).map_err(|e| err("IO", e.to_string()))?;
        if check != bytes {
            return Err(err("IO", "Legacy backup verification failed."));
        }
        let _: Vec<Note> =
            serde_json::from_slice(&check).map_err(|e| err("INVALID_DATA", e.to_string()))?;
        let assets_index: Value = read_json_default(
            &path.with_file_name("assets.json"),
            json!({"auto_prune":false,"assets":{}}),
        );
        let index_assets = assets_index
            .get("assets")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let auto_prune = assets_index
            .get("auto_prune")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut imported_assets = vec![];
        let asset_dir = path.with_file_name("assets");
        if let Ok(meta) = fs::symlink_metadata(&asset_dir) {
            if meta.file_type().is_symlink() {
                return Err(err(
                    "INVALID_DATA",
                    "Legacy assets directory must not be a symlink.",
                ));
            }
        }
        if asset_dir.is_dir() {
            for ent in fs::read_dir(&asset_dir).map_err(|e| err("IO", e.to_string()))? {
                let ent = ent.map_err(|e| err("IO", e.to_string()))?;
                let p = ent.path();
                let meta = fs::symlink_metadata(&p).map_err(|e| err("IO", e.to_string()))?;
                if meta.file_type().is_symlink() {
                    return Err(err(
                        "INVALID_DATA",
                        format!("Legacy attachment symlink is not imported: {}", p.display()),
                    ));
                }
                if !meta.is_file() {
                    continue;
                }
                let Some(ext) = p.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                if !["png", "jpg", "gif", "webp"].contains(&ext) {
                    continue;
                }
                let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                if !valid_hash(stem) {
                    continue;
                }
                if meta.len() > MAX_ASSET_BYTES as u64 {
                    return Err(err("INVALID_DATA", "Legacy attachment exceeds 20 MiB"));
                }
                let data = fs::read(&p).map_err(|e| err("IO", e.to_string()))?;
                if data.len() > MAX_ASSET_BYTES || hash(&data) != stem || !image_matches(ext, &data)
                {
                    return Err(err(
                        "INVALID_DATA",
                        format!(
                            "Legacy attachment failed hash/format validation: {}",
                            p.display()
                        ),
                    ));
                }
                let ix = index_assets.get(stem).cloned().unwrap_or_else(|| json!({}));
                let added = ix.get("added").and_then(Value::as_u64).unwrap_or_else(now);
                let last = ix
                    .get("last_note")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let unused = ix.get("unused_since").and_then(Value::as_u64);
                let mime = mime_from_ext(ext).to_owned();
                imported_assets.push((
                    stem.to_owned(),
                    p.file_name().unwrap().to_string_lossy().to_string(),
                    mime,
                    data,
                    added,
                    last,
                    unused,
                ));
            }
        }
        let mut prefs = vec![("autoPrune".to_string(), json!(auto_prune))];
        let lib: Value = read_json_default(&path.with_file_name("library.json"), json!([]));
        if lib.is_array() {
            prefs.push(("expandedProjects".to_string(), lib));
        }
        let nav: Value = read_json_default(&path.with_file_name("navigation.json"), json!({}));
        let mut visits = json!({});
        if let Some(obj) = nav.as_object() {
            for (id, v) in obj {
                if Uuid::parse_str(id).is_ok() {
                    visits[id] = json!({"count":v.get("count").and_then(Value::as_u64).unwrap_or(0),"lastVisited":v.get("last").and_then(Value::as_u64).unwrap_or(0)});
                }
            }
        }
        prefs.push(("navigation".into(), visits));
        let (_,snapshot)=self.write_tx(None,|tx|{for n in &notes{tx.execute("INSERT INTO notes(id,body) VALUES(?1,?2)",params![n.id,serde_json::to_string(n).map_err(internal)?]).map_err(internal)?;}for(id,name,mime,data,added,last,unused)in &imported_assets{tx.execute("INSERT INTO assets(id,name,mime,data,added,last_note,unused_since) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,name,mime,data,added,last,unused]).map_err(internal)?;}for(k,v)in &prefs{tx.execute("INSERT INTO preferences(key,body) VALUES(?1,?2)",params![k,serde_json::to_string(v).map_err(internal)?]).map_err(internal)?;}tx.execute("INSERT OR REPLACE INTO meta(key,value) VALUES('legacy_imported',1)",[]).map_err(internal)?;Ok(())})?;
        Ok(snapshot)
    }
}

fn preference_rows(
    tx: &Transaction<'_>,
) -> Result<std::collections::HashMap<String, String>, ApiError> {
    let mut q = tx
        .prepare("SELECT key,body FROM preferences")
        .map_err(internal)?;
    let rows = q
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(internal)?;
    rows.collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(internal)
}

fn lock_path_for(p: &Path) -> PathBuf {
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    fs::canonicalize(parent)
        .unwrap_or_else(|_| parent.to_path_buf())
        .join(".memoivy.lock")
}
fn lock_file(path: &Path) -> Result<File, String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?
    }
    let mut opts = OpenOptions::new();
    opts.create(true).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let f = opts.open(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    fs2::FileExt::try_lock_exclusive(&f)
        .map_err(|_| format!("another MemoIvy writer holds {}", path.display()))?;
    Ok(f)
}
fn backup_path(p: &Path) -> PathBuf {
    let name = p
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or("notes.json");
    p.with_file_name(format!("{name}.memoivy-backup"))
}
fn write_new_synced(p: &Path, data: &[u8]) -> io::Result<()> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(p)?;
    f.write_all(data)?;
    f.sync_all()?;
    if let Some(dir) = p.parent() {
        File::open(dir)?.sync_all()?
    }
    Ok(())
}
fn read_json_default(path: &Path, default: Value) -> Value {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite_store::Engine;
    use serde_json::json;
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
        let stale_pref=req(&mut e,"preference",json!({"baseRevision":2,"key":"autoPrune","value":false})).unwrap_err();
        assert_eq!(stale_pref.code,"CONFLICT");assert!(stale_pref.snapshot.is_some());
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
    fn legacy_import_preserves_attachment_history_and_library_preferences() {
        let (d, mut e) = open();
        let legacy = d.path().join("notes.json");
        let note_id = Uuid::new_v4();
        let bytes = b"\x89PNG\r\n\x1a\nsynthetic";
        let id = hash(bytes);
        let source = format!(
            r#"[{{"id":"{note_id}","text":"![x](asset:{id})","color":"yellow","modified":42}}]"#
        );
        fs::write(&legacy, source).unwrap();
        fs::create_dir(d.path().join("assets")).unwrap();
        fs::write(d.path().join("assets").join(format!("{id}.png")), bytes).unwrap();
        fs::write(d.path().join("assets.json"), format!(r#"{{"auto_prune":true,"assets":{{"{id}":{{"added":11,"last_note":"{note_id}"}}}}}}"#)).unwrap();
        fs::write(d.path().join("library.json"), format!(r#"["{note_id}"]"#)).unwrap();
        fs::write(
            d.path().join("navigation.json"),
            format!(r#"{{"{note_id}":{{"count":3,"last":12}}}}"#),
        )
        .unwrap();
        let s = e.import_legacy(&legacy).unwrap();
        assert_eq!(s.assets.len(), 1);
        assert_eq!(s.assets[0].added, 11);
        assert_eq!(s.assets[0].used_by, vec![note_id.to_string()]);
        assert_eq!(s.preferences["autoPrune"], true);
        assert_eq!(s.preferences["expandedProjects"][0], note_id.to_string());
        assert_eq!(
            s.preferences["navigation"][note_id.to_string()]["lastVisited"],
            12
        );
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
    fn imported_path_lock_is_retained_for_engine_lifetime() {
        let data = tempfile::tempdir().unwrap();
        let source_dir = tempfile::tempdir().unwrap();
        let legacy = source_dir.path().join("notes.json");
        fs::write(&legacy, "[]").unwrap();
        let mut e = Engine::open(data.path(), None).unwrap();
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
}
