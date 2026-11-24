mod attachments;
mod notes;
mod preferences;
pub(crate) mod shared;

use crate::{
    error::{err, ApiResult},
    sqlite_store::Engine,
};
use serde_json::{json, Value};
use std::path::Path;

impl Engine {
    pub(crate) fn dispatch(&mut self, op: &str, p: &Value) -> ApiResult {
        let result = match op {
            "snapshot" | "flush" => self.response_snapshot(),
            "create" => self.create(p),
            "update" => self.update(p),
            "delete" => self.delete(p),
            "canvas" => self.canvas(p),
            "restore" => self.restore(p),
            "preference" => self.set_preference(p),
            "importLegacy" => {
                let path = p
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err("INVALID_PARAMS", "path is required"))?;
                let snapshot = self.import_legacy(Path::new(path))?;
                Ok(json!({"snapshot":snapshot}))
            }
            "importAsset" => self.import_asset(p),
            "readAsset" => self.read_asset(p),
            "deleteAsset" => self.delete_asset(p),
            _ => Err(err("UNKNOWN_OP", "Unknown operation")),
        };
        match result {
            Err(mut e) => {
                if e.code == "CONFLICT" && e.snapshot.is_none() {
                    e.snapshot = self.snapshot().ok().map(Box::new);
                }
                Err(e)
            }
            Ok(value) => Ok(value),
        }
    }
}
