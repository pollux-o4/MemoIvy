use super::Engine;
use crate::{
    domain::*,
    error::{err, internal, ApiError},
    sqlite_store::{notes_tx, reconcile_asset_usage},
};
use rusqlite::{params, OptionalExtension, Transaction};
use std::collections::HashSet;

pub(crate) fn ensure_note_fresh(tx: &Transaction<'_>, id: &str, base: u64) -> Result<(), ApiError> {
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

pub(crate) fn ensure_preference_fresh(
    tx: &Transaction<'_>,
    key: &str,
    base: u64,
) -> Result<(), ApiError> {
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

pub(super) fn revision(engine: &Engine) -> Result<u64, String> {
    engine
        .db
        .query_row("SELECT value FROM meta WHERE key='revision'", [], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())
}

pub(super) fn write_tx<T>(
    engine: &mut Engine,
    base: Option<u64>,
    op: impl FnOnce(&Transaction<'_>) -> Result<T, ApiError>,
) -> Result<(T, Snapshot), ApiError> {
    let current = engine.revision().map_err(internal)?;
    if base.is_some_and(|base| base > current) {
        return Err(err(
            "INVALID_PARAMS",
            "baseRevision is newer than the current revision.",
        ));
    }
    let tx = engine.db.transaction().map_err(internal)?;
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
            set_note_revision(&tx, &note.id, next)?;
        }
    }
    for old in &before_notes {
        if !after_ids.contains(old.id.as_str()) {
            set_note_revision(&tx, &old.id, next)?;
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
    let snapshot = engine.snapshot().map_err(internal)?;
    Ok((result, snapshot))
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

fn set_note_revision(tx: &Transaction<'_>, id: &str, revision: u64) -> Result<(), ApiError> {
    tx.execute(
        "INSERT INTO note_revisions(id,revision) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision",
        params![id, revision],
    )
    .map_err(internal)?;
    Ok(())
}
