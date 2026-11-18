use super::Engine;
use crate::{
    domain::*,
    error::{internal, ApiError, ApiResult},
};
use rusqlite::{params, Transaction};
use serde::Serialize;
use serde_json::{json, Value};

pub(super) fn snapshot(engine: &Engine) -> Result<Snapshot, String> {
    let mut st = engine
        .db
        .prepare("SELECT body FROM notes ORDER BY rowid")
        .map_err(|e| e.to_string())?;
    let rows = st
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut notes = Vec::new();
    for row in rows {
        notes.push(
            serde_json::from_str(&row.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?,
        )
    }
    let assets = assets(engine, &notes)?;
    let revision = super::revision::revision(engine)?;
    let preferences = preferences(engine)?;
    let note_revisions = {
        let mut q = engine
            .db
            .prepare("SELECT nr.id,nr.revision FROM note_revisions nr JOIN notes n ON n.id=nr.id")
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

fn preferences(engine: &Engine) -> Result<Value, String> {
    let mut out = crate::commands::shared::preference_defaults();
    let mut q = engine
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

fn assets(engine: &Engine, notes: &[Note]) -> Result<Vec<Asset>, String> {
    let mut s=engine.db.prepare("SELECT id,name,mime,length(data),added,last_note,unused_since FROM assets ORDER BY name,id").map_err(|e|e.to_string())?;
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

pub(super) fn response_snapshot(engine: &Engine) -> ApiResult {
    snapshot(engine)
        .map(|s| json!({"snapshot":s}))
        .map_err(internal)
}

pub(super) fn prune_expired(engine: &mut Engine) -> ApiResult {
    let prefs = preferences(engine).map_err(internal)?;
    if prefs.get("autoPrune").and_then(Value::as_bool) != Some(true) {
        return Ok(json!({}));
    }
    let cutoff = now().saturating_sub(30 * 24 * 60 * 60);
    let expired: Vec<String> = {
        let mut q = engine
            .db
            .prepare("SELECT id FROM assets WHERE unused_since IS NOT NULL AND unused_since<=?1")
            .map_err(internal)?;
        let rows = q.query_map([cutoff], |r| r.get(0)).map_err(internal)?;
        rows.collect::<Result<_, _>>().map_err(internal)?
    };
    if expired.is_empty() {
        return Ok(json!({}));
    }
    let (_, snapshot) = super::revision::write_tx(engine, None, |tx| {
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
            params![n.id, serialize_body(n)?],
        )
        .map_err(internal)?;
    }
    Ok(())
}

pub(crate) fn serialize_body(value: &impl Serialize) -> Result<String, ApiError> {
    serde_json::to_string(value).map_err(internal)
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
