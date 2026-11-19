use super::shared::*;
use crate::{
    domain::*,
    error::{err, internal, ApiResult},
    sqlite_store::{notes_referencing_asset, notes_tx, Engine},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

impl Engine {
    pub(super) fn import_asset(&mut self, p: &Value) -> ApiResult {
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

    pub(super) fn read_asset(&self, p: &Value) -> ApiResult {
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

    pub(super) fn delete_asset(&mut self, p: &Value) -> ApiResult {
        let base = base(p)?;
        let id = asset_id(p)?;
        let (_, s) = self.write_tx(Some(base), |tx| {
            let ns = notes_tx(tx)?;
            if notes_referencing_asset(&ns, &id).next().is_some() {
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
