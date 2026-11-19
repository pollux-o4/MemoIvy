use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const COLORS: &[&str] = &["yellow", "pink", "blue", "green", "purple"];
/// 좌표 절댓값 상한. desktop/canvas_bounds.json과 같아야 한다(테스트로 확인).
const MAX_POSITION: f64 = 20_000.0;
pub(crate) const MAX_ASSET_BYTES: usize = 20 * 1024 * 1024;
pub(crate) const MAX_DEPTH: usize = 3;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct Canvas {
    pub(crate) position: Option<[f64; 2]>,
    pub(crate) next: Vec<String>,
    pub(crate) done: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) project: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parent: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) pinned: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub(crate) struct Note {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) color: String,
    pub(crate) modified: u64,
    #[serde(default)]
    pub(crate) canvas: Canvas,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Asset {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) mime: String,
    pub(crate) bytes: usize,
    #[serde(rename = "usedBy")]
    pub(crate) used_by: Vec<String>,
    pub(crate) added: u64,
    #[serde(rename = "lastNote")]
    pub(crate) last_note: Option<String>,
    #[serde(rename = "unusedSince")]
    pub(crate) unused_since: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Snapshot {
    pub(crate) notes: Vec<Note>,
    pub(crate) assets: Vec<Asset>,
    pub(crate) revision: u64,
    #[serde(rename = "savedRevision")]
    pub(crate) saved_revision: u64,
    pub(crate) preferences: Value,
    #[serde(rename = "noteRevisions")]
    pub(crate) note_revisions: std::collections::BTreeMap<String, u64>,
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(crate) fn hash(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(crate) fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub(crate) fn mime_from_ext(e: &str) -> &'static str {
    match e {
        "png" => "image/png",
        "jpg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}
pub(crate) fn image_matches(kind: &str, b: &[u8]) -> bool {
    match kind {
        "png" => b.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => b.starts_with(&[0xff, 0xd8, 0xff]),
        "gif" => b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a"),
        "webp" => b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP",
        _ => false,
    }
}
pub(crate) fn promote_projects(ns: &mut [Note]) {
    let parents: HashSet<String> = ns.iter().filter_map(|n| n.canvas.parent.clone()).collect();
    for n in ns {
        if (parents.contains(&n.id) || n.canvas.pinned) && !n.canvas.project {
            n.canvas.project = true
        }
    }
}
pub(crate) fn note_depth(ns: &[Note], id: &str) -> Option<usize> {
    let mut cur = Some(id);
    let mut d = 0;
    let mut seen = HashSet::new();
    while let Some(x) = cur {
        if !seen.insert(x) {
            return None;
        }
        let n = ns.iter().find(|n| n.id == x)?;
        cur = n.canvas.parent.as_deref();
        if cur.is_some() {
            d += 1
        }
    }
    Some(d)
}
pub(crate) fn validate_notes(ns: &[Note]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for n in ns {
        let parsed = uuid::Uuid::parse_str(&n.id).map_err(|_| "invalid note id".to_string())?;
        if parsed.to_string() != n.id {
            return Err("noncanonical note id".into());
        }
        if !ids.insert(n.id.as_str()) {
            return Err("duplicate note id".into());
        }
        if !COLORS.contains(&n.color.as_str()) {
            return Err("unknown note color".into());
        }
        if n.canvas.position.is_some_and(|p| {
            p.iter()
                .any(|x| !x.is_finite() || x.abs() > MAX_POSITION)
        }) {
            return Err("invalid canvas position".into());
        }
    }
    let by_id: HashMap<&str, &Note> = ns.iter().map(|n| (n.id.as_str(), n)).collect();
    for n in ns {
        let mut seen = HashSet::new();
        for edge in &n.canvas.next {
            if uuid::Uuid::parse_str(edge)
                .ok()
                .is_none_or(|u| u.to_string() != *edge)
                || edge == &n.id
                || !by_id.contains_key(edge.as_str())
                || !seen.insert(edge)
            {
                return Err("invalid canvas edge".into());
            }
        }
        if let Some(parent) = &n.canvas.parent {
            if uuid::Uuid::parse_str(parent)
                .ok()
                .is_none_or(|u| u.to_string() != *parent)
                || parent == &n.id
                || !by_id.contains_key(parent.as_str())
            {
                return Err("invalid project parent".into());
            }
        }
    }
    for n in ns {
        let mut cur = Some(n.id.as_str());
        let mut depth = 0;
        let mut chain = HashSet::new();
        while let Some(id) = cur {
            if !chain.insert(id) {
                return Err("project cycle".into());
            }
            depth += 1;
            if depth > MAX_DEPTH + 1 {
                return Err("project nesting exceeds depth limit".into());
            }
            cur = by_id.get(id).and_then(|x| x.canvas.parent.as_deref())
        }
    }
    Ok(())
}

#[cfg(test)]
mod bounds_tests {
    use super::MAX_POSITION;

    #[test]
    fn position_limit_matches_shared_bounds_file() {
        let shared: serde_json::Value =
            serde_json::from_str(include_str!("../../desktop/canvas_bounds.json")).unwrap();
        assert_eq!(shared["limit"].as_f64(), Some(MAX_POSITION));
    }
}
