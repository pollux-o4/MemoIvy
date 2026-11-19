use super::shared::*;
use crate::{
    error::{err, internal, ApiResult},
    sqlite_store::{ensure_preference_fresh, serialize_body, Engine},
};
use rusqlite::params;
use serde_json::{json, Value};

impl Engine {
    pub(super) fn set_preference(&mut self, p: &Value) -> ApiResult {
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
        let body = serialize_body(value)?;
        let (_,s)=self.write_tx(Some(base),|tx|{ensure_preference_fresh(tx,&key,base)?;tx.execute("INSERT INTO preferences(key,body) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET body=excluded.body",params![key,body]).map_err(internal)?;Ok(())})?;
        Ok(json!({"snapshot":s}))
    }
}
