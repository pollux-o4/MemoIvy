use super::shared::*;
use crate::{
    domain::*,
    error::{err, ApiError, ApiResult},
    sqlite_store::{ensure_note_fresh, notes_tx, persist_all, Engine},
};
use rusqlite::Transaction;
use serde_json::{json, Value};
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

    pub(super) fn delete(&mut self, p: &Value) -> ApiResult {
        let (_, s) = self.edit_note(p, |ns, id| {
            if !ns.iter().any(|n| n.id == id) {
                return Err(err("NOT_FOUND", "Note not found"));
            }
            let parent = ns
                .iter()
                .find(|n| n.id == id)
                .and_then(|n| n.canvas.parent.clone());
            for n in ns.iter_mut() {
                if n.canvas.parent.as_deref() == Some(id) {
                    n.canvas.parent = parent.clone()
                }
                n.canvas.next.retain(|x| x != id)
            }
            ns.retain(|n| n.id != id);
            Ok(())
        })?;
        Ok(json!({"snapshot":s}))
    }

    pub(super) fn canvas(&mut self, p: &Value) -> ApiResult {
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

/// Derives project flags, validates the hierarchy and persists every note.
fn save_notes(tx: &Transaction<'_>, ns: &mut [Note]) -> Result<(), ApiError> {
    promote_projects(ns);
    validate_notes(ns).map_err(|m| err("INVALID_DATA", m))?;
    persist_all(tx, ns)
}
