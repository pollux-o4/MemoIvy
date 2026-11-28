use super::shared::*;
use crate::{
    domain::*,
    error::{err, ApiError, ApiResult},
    sqlite_store::{ensure_note_fresh, notes_tx, persist_all, Engine},
};
use rusqlite::Transaction;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

impl Engine {
    pub(super) fn create(&mut self, p: &Value) -> ApiResult {
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
            save_notes(tx, &mut ns)
        })?;
        Ok(json!({"note":n,"snapshot":s}))
    }

    pub(super) fn update(&mut self, p: &Value) -> ApiResult {
        let (id, s) = self.edit_note(p, |ns, id| {
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

    /// Deletes `id` or every note in `ids` in one transaction (see `remove_notes`).
    pub(super) fn delete(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let ids: HashSet<String> = match p.get("ids") {
            Some(v) => v
                .as_array()
                .filter(|a| !a.is_empty())
                .ok_or_else(|| err("INVALID_PARAMS", "ids must be a non-empty array"))?
                .iter()
                .map(|x| required_id(&json!({ "id": x }), "id"))
                .collect::<Result<_, _>>()?,
            None => HashSet::from([required_id(p, "id")?]),
        };
        let (_, s) = self.write_tx(Some(base), |tx| {
            let mut ns = notes_tx(tx)?;
            let existing: HashSet<&str> = ns.iter().map(|n| n.id.as_str()).collect();
            for id in &ids {
                ensure_note_fresh(tx, id, base)?;
                if !existing.contains(id.as_str()) {
                    return Err(err("NOT_FOUND", "Note not found"));
                }
            }
            remove_notes(&mut ns, &ids);
            save_notes(tx, &mut ns)
        })?;
        Ok(json!({"snapshot":s}))
    }

    pub(super) fn canvas(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let changes = p
            .get("changes")
            .and_then(Value::as_array)
            .ok_or_else(|| err("INVALID_PARAMS", "changes must be an array"))?;
        let (_, s) = self.write_tx(Some(base), |tx| {
            let mut ns = notes_tx(tx)?;
            apply_changes(tx, &mut ns, changes, base)?;
            save_notes(tx, &mut ns)
        })?;
        Ok(json!({"snapshot":s}))
    }

    /// Puts deleted notes back with their original ids and fields, then reapplies the
    /// canvas fields (parent, next) that deletion rewrote on surviving notes. One transaction,
    /// so a restore either brings back the whole deletion or nothing.
    pub(super) fn restore(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let notes: Vec<Note> = p
            .get("notes")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| err("INVALID_PARAMS", "notes must be an array of notes"))?
            .ok_or_else(|| err("INVALID_PARAMS", "notes must be an array of notes"))?;
        let changes = p
            .get("changes")
            .and_then(Value::as_array)
            .ok_or_else(|| err("INVALID_PARAMS", "changes must be an array"))?;
        if notes.iter().any(|n| !COLORS.contains(&n.color.as_str())) {
            return Err(err("INVALID_DATA", "Unknown note color"));
        }
        let (_, s) = self.write_tx(Some(base), |tx| {
            let mut ns = notes_tx(tx)?;
            let mut known: HashSet<String> = ns.iter().map(|n| n.id.clone()).collect();
            for n in &notes {
                if !known.insert(n.id.clone()) {
                    return Err(err("CONFLICT", "Note already exists"));
                }
            }
            ns.extend(notes.iter().cloned());
            apply_changes(tx, &mut ns, changes, base)?;
            save_notes(tx, &mut ns)
        })?;
        Ok(json!({"snapshot":s}))
    }

    /// Loads notes for the note named by `p.id`, applies `edit`, then saves them.
    fn edit_note(
        &mut self,
        p: &Value,
        edit: impl FnOnce(&mut Vec<Note>, &str) -> Result<(), ApiError>,
    ) -> Result<(String, Snapshot), ApiError> {
        let base = base(p)?;
        let id = required_id(p, "id")?;
        let (_, s) = self.write_tx(Some(base), |tx| {
            ensure_note_fresh(tx, &id, base)?;
            let mut ns = notes_tx(tx)?;
            edit(&mut ns, &id)?;
            save_notes(tx, &mut ns)
        })?;
        Ok((id, s))
    }
}

/// Removes `ids`, lifts each surviving child to its nearest surviving ancestor and drops
/// links into the removed set. Same result as deleting one by one, in O(notes + links)
/// time: ancestor lookups are memoised so each chain is walked once.
fn remove_notes(ns: &mut Vec<Note>, ids: &HashSet<String>) {
    let parents: HashMap<String, Option<String>> = ns
        .iter()
        .filter(|n| ids.contains(&n.id))
        .map(|n| (n.id.clone(), n.canvas.parent.clone()))
        .collect();
    let mut lifted: HashMap<String, Option<String>> = HashMap::new();
    fn survivor(
        id: &str,
        parents: &HashMap<String, Option<String>>,
        lifted: &mut HashMap<String, Option<String>>,
    ) -> Option<String> {
        let Some(up) = parents.get(id) else {
            return Some(id.to_owned());
        };
        if let Some(hit) = lifted.get(id) {
            return hit.clone();
        }
        let found = up.as_deref().and_then(|u| survivor(u, parents, lifted));
        lifted.insert(id.to_owned(), found.clone());
        found
    }
    ns.retain(|n| !ids.contains(&n.id));
    for n in ns.iter_mut() {
        if let Some(p) = n.canvas.parent.as_deref().filter(|p| ids.contains(*p)) {
            n.canvas.parent = survivor(p, &parents, &mut lifted);
        }
        n.canvas.next.retain(|x| !ids.contains(x))
    }
}

/// Applies `{id, canvas}` patches to fresh notes.
fn apply_changes(
    tx: &Transaction<'_>,
    ns: &mut [Note],
    changes: &[Value],
    base: u64,
) -> Result<(), ApiError> {
    let index: HashMap<String, usize> = ns
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();
    for change in changes {
        let id = change
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| err("INVALID_PARAMS", "change id required"))?;
        ensure_note_fresh(tx, id, base)?;
        let &idx = index
            .get(id)
            .ok_or_else(|| err("NOT_FOUND", "Note not found"))?;
        let n = &mut ns[idx];
        let cv = change
            .get("canvas")
            .ok_or_else(|| err("INVALID_PARAMS", "canvas required"))?;
        apply_canvas(&mut n.canvas, cv)?;
        n.modified = now()
    }
    Ok(())
}

/// Derives project flags, validates the hierarchy and persists every note.
fn save_notes(tx: &Transaction<'_>, ns: &mut [Note]) -> Result<(), ApiError> {
    promote_projects(ns);
    validate_notes(ns).map_err(|m| err("INVALID_DATA", m))?;
    persist_all(tx, ns)
}
