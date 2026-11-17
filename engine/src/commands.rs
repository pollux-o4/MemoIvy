use crate::{
    domain::*,
    error::{err, internal, ApiError, ApiResult},
    sqlite_store::Engine,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::{collections::HashSet, path::Path};
use uuid::Uuid;

impl Engine {
    pub(crate) fn dispatch(&mut self, op: &str, p: &Value) -> ApiResult {
        let result = match op {
            "snapshot" | "flush" => self.response_snapshot(),
            "create" => self.create(p),
            "update" => self.update(p),
            "delete" => self.delete(p),
            "canvas" => self.canvas(p),
            "preference" => self.set_preference(p),
            "importLegacy" => {
                let path = p
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err("INVALID_PARAMS", "path is required"))?;
                let s = self.import_legacy(Path::new(path))?;
                Ok(json!({"snapshot":s}))
            }
            "importAsset" => self.import_asset(p),
            "readAsset" => self.read_asset(p),
            "deleteAsset" => self.delete_asset(p),
            _ => Err(err("UNKNOWN_OP", "Unknown operation")),
        };
        match result {
            Err(mut e) => {
                if e.code == "CONFLICT" && e.snapshot.is_none() {
                    e.snapshot = self.snapshot().ok();
                }
                Err(e)
            }
            Ok(value) => Ok(value),
        }
    }
    fn set_preference(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let key = p
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| err("INVALID_PARAMS", "key is required"))?;
        let value = p
            .get("value")
            .ok_or_else(|| err("INVALID_PARAMS", "value is required"))?;
        validate_preference(key, value)?;
        let key = key.to_owned();
        let body = serde_json::to_string(value).map_err(internal)?;
        let (_,s)=self.write_tx(Some(base),|tx|{ensure_preference_fresh(tx,&key,base)?;tx.execute("INSERT INTO preferences(key,body) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET body=excluded.body",params![key,body]).map_err(internal)?;Ok(())})?;
        Ok(json!({"snapshot":s}))
    }
    fn create(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let text = opt_string(p, "text")?.unwrap_or_default().to_owned();
        let color = opt_string(p, "color")?.unwrap_or("yellow").to_owned();
        if !COLORS.contains(&color.as_str()) {
            return Err(err("INVALID_DATA", "Unknown note color"));
        }
        let id = Uuid::new_v4().to_string();
        let mut n = Note {
            id,
            text,
            color,
            modified: now(),
            canvas: Canvas::default(),
        };
        n.canvas.position = position_param(p.get("position"))?;
        n.canvas.parent = nullable_id(p.get("parent"))?;
        n.canvas.project = p.get("project").and_then(Value::as_bool).unwrap_or(false);
        let (_, s) = self.write_tx(Some(base), |tx| {
            let mut ns = notes_tx(tx)?;
            if n.canvas.project {
                let pd = n
                    .canvas
                    .parent
                    .as_ref()
                    .map(|id| note_depth(&ns, id))
                    .unwrap_or(Some(0));
                if pd.is_none_or(|d| d + 1 >= MAX_DEPTH) {
                    return Err(err(
                        "INVALID_DATA",
                        "Project cannot contain a child at this nesting depth",
                    ));
                }
            }
            ns.push(n.clone());
            promote_projects(&mut ns);
            validate_notes(&ns).map_err(|m| err("INVALID_DATA", m))?;
            persist_all(tx, &ns)?;
            Ok(())
        })?;
        Ok(json!({"note":n,"snapshot":s}))
    }
    fn update(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let id = required_id(p, "id")?;
        let (_, s) = self.write_tx(Some(base), |tx| {
            ensure_note_fresh(tx, &id, base)?;
            let mut ns = notes_tx(tx)?;
            let n = ns
                .iter_mut()
                .find(|n| n.id == id)
                .ok_or_else(|| err("NOT_FOUND", "Note not found"))?;
            if let Some(v) = p.get("text") {
                n.text = v
                    .as_str()
                    .ok_or_else(|| err("INVALID_PARAMS", "text must be a string"))?
                    .into();
                n.modified = now()
            }
            if let Some(v) = p.get("color") {
                let c = v
                    .as_str()
                    .ok_or_else(|| err("INVALID_PARAMS", "color must be a string"))?;
                if !COLORS.contains(&c) {
                    return Err(err("INVALID_DATA", "Unknown note color"));
                }
                n.color = c.into();
                n.modified = now()
            }
            if let Some(v) = p.get("canvas") {
                apply_canvas(&mut n.canvas, v)?;
                n.modified = now()
            }
            promote_projects(&mut ns);
            validate_notes(&ns).map_err(|m| err("INVALID_DATA", m))?;
            persist_all(tx, &ns)?;
            Ok(())
        })?;
        let note = s
            .notes
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or_else(|| err("NOT_FOUND", "Note not found"))?;
        Ok(json!({"note":note,"snapshot":s}))
    }
    fn delete(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let id = required_id(p, "id")?;
        let (_, s) = self.write_tx(Some(base), |tx| {
            ensure_note_fresh(tx, &id, base)?;
            let mut ns = notes_tx(tx)?;
            if !ns.iter().any(|n| n.id == id) {
                return Err(err("NOT_FOUND", "Note not found"));
            }
            let parent = ns
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| n.canvas.parent.clone());
            for n in &mut ns {
                if n.canvas.parent.as_deref() == Some(&id) {
                    n.canvas.parent = parent.clone()
                }
                n.canvas.next.retain(|x| x != &id)
            }
            ns.retain(|n| n.id != id);
            promote_projects(&mut ns);
            persist_all(tx, &ns)?;
            Ok(())
        })?;
        Ok(json!({"snapshot":s}))
    }
    fn canvas(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let changes = p
            .get("changes")
            .and_then(Value::as_array)
            .ok_or_else(|| err("INVALID_PARAMS", "changes must be an array"))?;
        let cs = changes.clone();
        let (_, s) = self.write_tx(Some(base), |tx| {
            let mut ns = notes_tx(tx)?;
            for change in cs {
                let id = change
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err("INVALID_PARAMS", "change id required"))?;
                ensure_note_fresh(tx, id, base)?;
                let n = ns
                    .iter_mut()
                    .find(|n| n.id == id)
                    .ok_or_else(|| err("NOT_FOUND", "Note not found"))?;
                let cv = change
                    .get("canvas")
                    .ok_or_else(|| err("INVALID_PARAMS", "canvas required"))?;
                apply_canvas(&mut n.canvas, cv)?;
                n.modified = now()
            }
            promote_projects(&mut ns);
            validate_notes(&ns).map_err(|m| err("INVALID_DATA", m))?;
            persist_all(tx, &ns)?;
            Ok(())
        })?;
        Ok(json!({"snapshot":s}))
    }
    fn import_asset(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let name = opt_string(p, "name")?.unwrap_or("file").to_owned();
        let data = STANDARD
            .decode(
                p.get("data")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err("INVALID_PARAMS", "data is required"))?,
            )
            .map_err(|_| err("INVALID_PARAMS", "data must be base64"))?;
        if data.len() > MAX_ASSET_BYTES {
            return Err(err("INVALID_DATA", "Attachment exceeds 20 MiB"));
        }
        if name.len() > 1024 || name.chars().any(char::is_control) {
            return Err(err("INVALID_PARAMS", "Invalid attachment name"));
        }
        let id = hash(&data);
        let mime = opt_string(p, "mime")?
            .unwrap_or("application/octet-stream")
            .to_owned();
        if mime.len() > 255 || mime.chars().any(char::is_control) {
            return Err(err("INVALID_PARAMS", "Invalid MIME type"));
        }
        if mime.starts_with("image/")
            && !matches!(
                mime.as_str(),
                "image/png" | "image/jpeg" | "image/gif" | "image/webp"
            )
        {
            return Err(err("INVALID_DATA", "Unsupported image type"));
        }
        if mime.starts_with("image/") && !image_matches(&mime[6..], &data) {
            return Err(err(
                "INVALID_DATA",
                "Image bytes do not match the MIME type",
            ));
        }
        let (_,s)=self.write_tx(Some(base),|tx|{tx.execute("INSERT OR IGNORE INTO assets(id,name,mime,data,added,unused_since) VALUES(?1,?2,?3,?4,?5,?5)",params![id,name,mime,data,now()]).map_err(internal)?;Ok(())})?;
        let asset = s.assets.iter().find(|a| a.id == id).cloned().unwrap();
        Ok(json!({"asset":asset,"snapshot":s}))
    }
    fn read_asset(&self, p: &Value) -> ApiResult {
        let id = asset_id(p)?;
        let (data, mime): (Vec<u8>, String) = self
            .db
            .query_row("SELECT data,mime FROM assets WHERE id=?1", [&id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()
            .map_err(internal)?
            .ok_or_else(|| err("NOT_FOUND", "Attachment not found"))?;
        Ok(json!({"data":STANDARD.encode(data),"mime":mime}))
    }
    fn delete_asset(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let id = asset_id(p)?;
        let (_, s) = self.write_tx(Some(base), |tx| {
            let ns = notes_tx(tx)?;
            if ns.iter().any(|n| n.text.contains(&format!("asset:{id}"))) {
                return Err(err("IN_USE", "Attachment is used by a note"));
            }
            let count = tx
                .execute("DELETE FROM assets WHERE id=?1", [&id])
                .map_err(internal)?;
            if count == 0 {
                return Err(err("NOT_FOUND", "Attachment not found"));
            }
            Ok(())
        })?;
        Ok(json!({"snapshot":s}))
    }
}
fn base(p: &Value) -> Result<u64, ApiError> {
    p.get("baseRevision")
        .and_then(Value::as_u64)
        .ok_or_else(|| err("INVALID_PARAMS", "baseRevision is required"))
}
fn ensure_note_fresh(tx: &Transaction<'_>, id: &str, base: u64) -> Result<(), ApiError> {
    let revision: Option<u64> = tx
        .query_row(
            "SELECT revision FROM note_revisions WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(internal)?;
    if revision.is_some_and(|revision| revision > base) {
        return Err(err("CONFLICT", "This note changed in another window."));
    }
    Ok(())
}
fn ensure_preference_fresh(tx: &Transaction<'_>, key: &str, base: u64) -> Result<(), ApiError> {
    let revision: Option<u64> = tx
        .query_row(
            "SELECT revision FROM preference_revisions WHERE key=?1",
            [key],
            |r| r.get(0),
        )
        .optional()
        .map_err(internal)?;
    if revision.is_some_and(|revision| revision > base) {
        return Err(err(
            "CONFLICT",
            "This preference changed in another window.",
        ));
    }
    Ok(())
}
fn asset_id(p: &Value) -> Result<String, ApiError> {
    let s = p
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| err("INVALID_PARAMS", "id is required"))?;
    if !valid_hash(s) {
        return Err(err("INVALID_PARAMS", "Invalid asset id"));
    }
    Ok(s.into())
}
fn required_id(p: &Value, k: &str) -> Result<String, ApiError> {
    let s = p
        .get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| err("INVALID_PARAMS", format!("{k} is required")))?;
    Uuid::parse_str(s).map_err(|_| err("INVALID_PARAMS", format!("{k} must be a UUID")))?;
    Ok(s.into())
}
fn opt_string<'a>(p: &'a Value, k: &str) -> Result<Option<&'a str>, ApiError> {
    match p.get(k) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        _ => Err(err("INVALID_PARAMS", format!("{k} must be a string"))),
    }
}
fn nullable_id(v: Option<&Value>) -> Result<Option<String>, ApiError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let parsed = Uuid::parse_str(s)
                .map_err(|_| err("INVALID_PARAMS", "parent must be a UUID or null"))?;
            if parsed.to_string() != *s {
                return Err(err("INVALID_PARAMS", "parent UUID must be canonical"));
            }
            Ok(Some(s.clone()))
        }
        _ => Err(err("INVALID_PARAMS", "parent must be a UUID or null")),
    }
}
fn position_param(v: Option<&Value>) -> Result<Option<[f64; 2]>, ApiError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let a = v
                .as_array()
                .filter(|a| a.len() == 2)
                .ok_or_else(|| err("INVALID_PARAMS", "position must be a pair or null"))?;
            let x = a[0]
                .as_f64()
                .ok_or_else(|| err("INVALID_PARAMS", "position must be numeric"))?;
            let y = a[1]
                .as_f64()
                .ok_or_else(|| err("INVALID_PARAMS", "position must be numeric"))?;
            Ok(Some([x, y]))
        }
    }
}
fn apply_canvas(c: &mut Canvas, v: &Value) -> Result<(), ApiError> {
    let o = v
        .as_object()
        .ok_or_else(|| err("INVALID_PARAMS", "canvas must be an object"))?;
    if let Some(x) = o.get("position") {
        c.position = position_param(Some(x))?
    }
    if let Some(x) = o.get("next") {
        c.next = serde_json::from_value(x.clone()).map_err(internal)?
    }
    if let Some(x) = o.get("done") {
        c.done = x
            .as_bool()
            .ok_or_else(|| err("INVALID_PARAMS", "done must be boolean"))?
    }
    if let Some(x) = o.get("project") {
        c.project = x
            .as_bool()
            .ok_or_else(|| err("INVALID_PARAMS", "project must be boolean"))?
    }
    if let Some(x) = o.get("parent") {
        c.parent = nullable_id(Some(x))?
    }
    if let Some(x) = o.get("pinned") {
        c.pinned = x
            .as_bool()
            .ok_or_else(|| err("INVALID_PARAMS", "pinned must be boolean"))?
    }
    Ok(())
}
pub(crate) fn notes_tx(tx: &Transaction<'_>) -> Result<Vec<Note>, ApiError> {
    let mut q = tx
        .prepare("SELECT body FROM notes ORDER BY rowid")
        .map_err(internal)?;
    let rows = q
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(internal)?;
    rows.map(|r| serde_json::from_str(&r.map_err(internal)?).map_err(internal))
        .collect()
}
pub(crate) fn persist_all(tx: &Transaction<'_>, notes: &[Note]) -> Result<(), ApiError> {
    tx.execute("DELETE FROM notes", []).map_err(internal)?;
    for n in notes {
        tx.execute(
            "INSERT INTO notes(id,body) VALUES(?1,?2)",
            params![n.id, serde_json::to_string(n).map_err(internal)?],
        )
        .map_err(internal)?;
    }
    Ok(())
}
fn validate_preference(k: &str, v: &Value) -> Result<(), ApiError> {
    match k {
        "expandedProjects" | "expandedNotes" => {
            let a = v.as_array().filter(|a| a.len() <= 10_000).ok_or_else(|| {
                err(
                    "INVALID_PARAMS",
                    "expanded ids must be an array of at most 10000 UUIDs",
                )
            })?;
            let mut seen = HashSet::new();
            for id in a {
                let s = id
                    .as_str()
                    .ok_or_else(|| err("INVALID_PARAMS", "expanded ids must be UUID strings"))?;
                let u = Uuid::parse_str(s)
                    .map_err(|_| err("INVALID_PARAMS", "invalid expanded UUID"))?;
                if u.to_string() != s || !seen.insert(s) {
                    return Err(err(
                        "INVALID_PARAMS",
                        "expanded UUIDs must be unique and canonical",
                    ));
                }
            }
        }
        "navigation" => {
            let o = v
                .as_object()
                .filter(|o| o.len() <= 64)
                .ok_or_else(|| err("INVALID_PARAMS", "navigation must have at most 64 entries"))?;
            for (id, val) in o {
                let u = Uuid::parse_str(id)
                    .map_err(|_| err("INVALID_PARAMS", "invalid navigation UUID"))?;
                if u.to_string() != *id {
                    return Err(err("INVALID_PARAMS", "navigation UUID must be canonical"));
                }
                let count = val
                    .get("count")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| err("INVALID_PARAMS", "navigation count must be an integer"))?;
                let last = val
                    .get("lastVisited")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| err("INVALID_PARAMS", "lastVisited must be an integer"))?;
                if count > u32::MAX as u64 || last > now().saturating_add(86400) {
                    return Err(err("INVALID_PARAMS", "navigation value is out of range"));
                }
            }
        }
        "autoPrune" => {
            if !v.is_boolean() {
                return Err(err("INVALID_PARAMS", "autoPrune must be boolean"));
            }
        }
        _ => return Err(err("INVALID_PARAMS", "Unknown preference key")),
    }
    if v.to_string().len() > 1024 * 1024 {
        return Err(err("INVALID_PARAMS", "Preference value is too large"));
    }
    Ok(())
}
pub(crate) fn reconcile_asset_usage(tx: &Transaction<'_>) -> Result<(), ApiError> {
    let notes = notes_tx(tx)?;
    let mut q = tx
        .prepare("SELECT id,last_note,unused_since FROM assets")
        .map_err(internal)?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<u64>>(2)?,
            ))
        })
        .map_err(internal)?;
    let mut updates = Vec::new();
    for row in rows {
        let (id, last, unused) = row.map_err(internal)?;
        let mut users: Vec<_> = notes
            .iter()
            .filter(|n| n.text.contains(&format!("asset:{id}")))
            .collect();
        users.sort_by_key(|n| std::cmp::Reverse(n.modified));
        if let Some(n) = users.first() {
            updates.push((id, Some(n.id.clone()), None))
        } else {
            updates.push((id, last, unused.or(Some(now()))))
        }
    }
    drop(q);
    for (id, last, unused) in updates {
        tx.execute(
            "UPDATE assets SET last_note=?2,unused_since=?3 WHERE id=?1",
            params![id, last, unused],
        )
        .map_err(internal)?;
    }
    Ok(())
}
