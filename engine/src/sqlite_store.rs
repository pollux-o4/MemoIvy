mod legacy;
mod queries;
mod revision;
mod schema;
#[cfg(test)]
mod tests;

use crate::{domain::Snapshot, error::ApiResult};
pub(crate) use queries::{notes_tx, persist_all, reconcile_asset_usage, serialize_body};
pub(crate) use revision::{ensure_note_fresh, ensure_preference_fresh};
use rusqlite::{Connection, OptionalExtension};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::HashSet,
    fs::{self, File},
    path::{Path, PathBuf},
};

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
        let lock = legacy::lock_file(&lock_path)?;
        let mut locks = vec![lock];
        let mut lock_paths = HashSet::from([lock_path.clone()]);
        if let Some(path) = &legacy {
            let legacy_lock = legacy::lock_path_for(path);
            if lock_paths.insert(legacy_lock.clone()) {
                locks.push(legacy::lock_file(&legacy_lock)?);
            }
        }
        let mut db = Connection::open(dir.join("memoivy.sqlite3")).map_err(|e| e.to_string())?;
        schema::initialize(&mut db)?;
        #[cfg(unix)]
        fs::set_permissions(
            dir.join("memoivy.sqlite3"),
            fs::Permissions::from_mode(0o600),
        )
        .map_err(|e| e.to_string())?;
        let mut engine = Self {
            db,
            _locks: locks,
            lock_paths,
            legacy,
        };
        let imported: i64 = engine
            .db
            .query_row(
                "SELECT value FROM meta WHERE key='legacy_imported'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .unwrap_or(0);
        if imported == 0 && engine.empty_for_import()? {
            if let Some(path) = engine.legacy.clone() {
                if path.exists() {
                    let _ = engine.import_legacy(&path).map_err(|e| e.message)?;
                }
            }
        }
        engine.prune_expired().map_err(|e| e.message)?;
        Ok(engine)
    }

    fn empty_for_import(&self) -> Result<bool, String> {
        let revision = self.revision()?;
        let notes: i64 = self
            .db
            .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let assets: i64 = self
            .db
            .query_row("SELECT count(*) FROM assets", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let preferences: i64 = self
            .db
            .query_row("SELECT count(*) FROM preferences", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(revision == 0 && notes == 0 && assets == 0 && preferences == 0)
    }
}

impl Engine {
    pub(crate) fn import_legacy(
        &mut self,
        path: &Path,
    ) -> Result<Snapshot, crate::error::ApiError> {
        legacy::import(self, path)
    }
    fn revision(&self) -> Result<u64, String> {
        revision::revision(self)
    }
    pub(crate) fn snapshot(&self) -> Result<Snapshot, String> {
        queries::snapshot(self)
    }
    pub(crate) fn response_snapshot(&self) -> ApiResult {
        queries::response_snapshot(self)
    }
    fn prune_expired(&mut self) -> ApiResult {
        queries::prune_expired(self)
    }
    pub(crate) fn write_tx<T>(
        &mut self,
        base: Option<u64>,
        op: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, crate::error::ApiError>,
    ) -> Result<(T, Snapshot), crate::error::ApiError> {
        revision::write_tx(self, base, op)
    }
}
