use crate::{
    domain::*,
    error::{err, internal, ApiError},
};
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

pub(super) fn base(p: &Value) -> Result<u64, ApiError> {
    p.get("baseRevision")
        .and_then(Value::as_u64)
        .ok_or_else(|| err("INVALID_PARAMS", "baseRevision is required"))
}

pub(super) fn asset_id(p: &Value) -> Result<String, ApiError> {
    let s = p
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| err("INVALID_PARAMS", "id is required"))?;
    if !valid_hash(s) {
        return Err(err("INVALID_PARAMS", "Invalid asset id"));
    }
    Ok(s.into())
}

pub(super) fn required_id(p: &Value, k: &str) -> Result<String, ApiError> {
    let s = p
        .get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| err("INVALID_PARAMS", format!("{k} is required")))?;
    Uuid::parse_str(s).map_err(|_| err("INVALID_PARAMS", format!("{k} must be a UUID")))?;
    Ok(s.into())
}

pub(super) fn opt_string<'a>(p: &'a Value, k: &str) -> Result<Option<&'a str>, ApiError> {
    match p.get(k) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        _ => Err(err("INVALID_PARAMS", format!("{k} must be a string"))),
    }
}

pub(super) fn nullable_id(v: Option<&Value>) -> Result<Option<String>, ApiError> {
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

pub(super) fn position_param(v: Option<&Value>) -> Result<Option<[f64; 2]>, ApiError> {
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

pub(super) fn apply_canvas(c: &mut Canvas, v: &Value) -> Result<(), ApiError> {
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

/// 설정 키와 기본값의 단일 출처. 새 키는 여기와 validate_preference에 함께 추가한다.
pub(crate) fn preference_defaults() -> Value {
    serde_json::json!({
        "expandedProjects": [],
        "expandedNotes": [],
        "navigation": {},
        "autoPrune": false,
        "appearance": { "theme": "system", "motion": "system" },
    })
}

pub(super) fn validate_preference(k: &str, v: &Value) -> Result<(), ApiError> {
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
        "appearance" => {
            let o = v
                .as_object()
                .filter(|o| o.len() == 2)
                .ok_or_else(|| err("INVALID_PARAMS", "appearance must be {theme, motion}"))?;
            let theme = o.get("theme").and_then(Value::as_str);
            let motion = o.get("motion").and_then(Value::as_str);
            if !matches!(theme, Some("system" | "light" | "dark"))
                || !matches!(motion, Some("system" | "reduce" | "full"))
            {
                return Err(err("INVALID_PARAMS", "appearance value is out of range"));
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
