use super::Engine;
use crate::{
    domain::*,
    error::{err, internal, ApiError},
    sqlite_store::queries::serialize_body,
};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub(super) fn import(engine: &mut Engine, path: &Path) -> Result<Snapshot, ApiError> {
    let lp = lock_path_for(path);
    if engine.lock_paths.insert(lp.clone()) {
        engine
            ._locks
            .push(lock_file(&lp).map_err(|e| err("LOCKED", e))?);
    }
    if !engine.empty_for_import().map_err(internal)? {
        return Err(err(
            "IMPORT_REJECTED",
            "Legacy import is allowed only into an empty database.",
        ));
    }
    if engine
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
            if data.len() > MAX_ASSET_BYTES || hash(&data) != stem || !image_matches(ext, &data) {
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
    let (_,snapshot)=engine.write_tx(None,|tx|{for n in &notes{tx.execute("INSERT INTO notes(id,body) VALUES(?1,?2)",params![n.id,serialize_body(n)?]).map_err(internal)?;}for(id,name,mime,data,added,last,unused)in &imported_assets{tx.execute("INSERT INTO assets(id,name,mime,data,added,last_note,unused_since) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,name,mime,data,added,last,unused]).map_err(internal)?;}for(k,v)in &prefs{tx.execute("INSERT INTO preferences(key,body) VALUES(?1,?2)",params![k,serialize_body(v)?]).map_err(internal)?;}tx.execute("INSERT OR REPLACE INTO meta(key,value) VALUES('legacy_imported',1)",[]).map_err(internal)?;Ok(())})?;
    Ok(snapshot)
}

pub(super) fn lock_path_for(p: &Path) -> PathBuf {
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    fs::canonicalize(parent)
        .unwrap_or_else(|_| parent.to_path_buf())
        .join(".memoivy.lock")
}

pub(super) fn lock_file(path: &Path) -> Result<File, String> {
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

pub(super) fn backup_path(p: &Path) -> PathBuf {
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
