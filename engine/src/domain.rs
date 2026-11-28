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
/// Scans every note text once in O(sum(|text|)) and maps each referenced valid asset hash
/// to the notes that contain `asset:<hash>` (preserving note order, deduplicated per note).
pub(crate) fn build_asset_index<'a>(notes: &'a [Note]) -> HashMap<&'a str, Vec<&'a Note>> {
    let mut index: HashMap<&'a str, Vec<&'a Note>> = HashMap::new();
    for note in notes {
        if !note.text.contains("asset:") {
            continue;
        }
        let mut seen = HashSet::new();
        for (pos, _) in note.text.match_indices("asset:") {
            let rest = &note.text[pos + 6..];
            if let Some(hash_str) = rest.get(..64).filter(|h| valid_hash(h) && seen.insert(*h)) {
                index.entry(hash_str).or_default().push(note);
            }
        }
    }
    index
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
    let mut by_id: HashMap<&str, usize> = HashMap::with_capacity(ns.len());
    for (idx, n) in ns.iter().enumerate() {
        let parsed = uuid::Uuid::parse_str(&n.id).map_err(|_| "invalid note id".to_string())?;
        if parsed.to_string() != n.id {
            return Err("noncanonical note id".into());
        }
        if by_id.insert(n.id.as_str(), idx).is_some() {
            return Err("duplicate note id".into());
        }
        if !COLORS.contains(&n.color.as_str()) {
            return Err("unknown note color".into());
        }
        if n.canvas
            .position
            .is_some_and(|p| p.iter().any(|x| !x.is_finite() || x.abs() > MAX_POSITION))
        {
            return Err("invalid canvas position".into());
        }
    }
    let mut parents: Vec<Option<usize>> = Vec::with_capacity(ns.len());
    for (idx, n) in ns.iter().enumerate() {
        let mut seen = HashSet::with_capacity(n.canvas.next.len());
        for edge in &n.canvas.next {
            if edge == &n.id || !by_id.contains_key(edge.as_str()) || !seen.insert(edge.as_str()) {
                return Err("invalid canvas edge".into());
            }
        }
        let parent_idx = match n.canvas.parent.as_deref() {
            Some(parent) => {
                let &p = by_id
                    .get(parent)
                    .filter(|&&p| p != idx)
                    .ok_or_else(|| "invalid project parent".to_string())?;
                Some(p)
            }
            None => None,
        };
        parents.push(parent_idx);
    }
    // 0 = unvisited, 1 = visiting, 2 = done (single O(N) pass)
    let mut state = vec![0u8; ns.len()];
    let mut depths = vec![0usize; ns.len()];
    let mut chain = Vec::new();
    for start in 0..ns.len() {
        if state[start] == 2 {
            continue;
        }
        chain.clear();
        let mut cur = Some(start);
        let base_depth = loop {
            let Some(i) = cur else { break 0 };
            match state[i] {
                1 => return Err("project cycle".into()),
                2 => break depths[i],
                _ => {
                    state[i] = 1;
                    chain.push(i);
                    cur = parents[i];
                }
            }
        };
        for (offset, &i) in chain.iter().rev().enumerate() {
            let depth = base_depth + offset + 1;
            if depth > MAX_DEPTH + 1 {
                return Err("project nesting exceeds depth limit".into());
            }
            depths[i] = depth;
            state[i] = 2;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_asset_index, validate_notes, Canvas, Note, MAX_DEPTH};

    fn make_note(id: &str, text: &str, parent: Option<&str>) -> Note {
        Note {
            id: id.into(),
            text: text.into(),
            color: "yellow".into(),
            modified: 1,
            canvas: Canvas {
                parent: parent.map(str::to_owned),
                ..Canvas::default()
            },
        }
    }

    #[test]
    fn asset_index_handles_multibyte_utf8_duplicates_and_invalid_hashes() {
        let h1 = "a".repeat(64);
        let h2 = "b".repeat(64);
        let upper = "C".repeat(64);
        let n1 = make_note(
            "00000000-0000-4000-8000-000000000001",
            &format!("![x](asset:{h1})한글바로뒤 ![dup](asset:{h1}) asset:{upper} asset:short"),
            None,
        );
        let n2 = make_note(
            "00000000-0000-4000-8000-000000000002",
            &format!("asset:{h1} and asset:{h2}"),
            None,
        );
        let notes = [n1, n2];
        let index = build_asset_index(&notes);
        assert_eq!(index.len(), 2);
        assert_eq!(
            index[h1.as_str()]
                .iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "00000000-0000-4000-8000-000000000001",
                "00000000-0000-4000-8000-000000000002",
            ]
        );
        assert_eq!(
            index[h2.as_str()]
                .iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>(),
            vec!["00000000-0000-4000-8000-000000000002"]
        );
    }

    #[test]
    fn validate_notes_checks_cycles_depth_and_edges_in_linear_pass() {
        let ids = [
            "00000000-0000-4000-8000-000000000001",
            "00000000-0000-4000-8000-000000000002",
            "00000000-0000-4000-8000-000000000003",
            "00000000-0000-4000-8000-000000000004",
            "00000000-0000-4000-8000-000000000005",
        ];
        // Depth MAX_DEPTH + 1 (4 nodes: 0 <- 1 <- 2 <- 3) is allowed.
        let allowed = vec![
            make_note(ids[0], "", None),
            make_note(ids[1], "", Some(ids[0])),
            make_note(ids[2], "", Some(ids[1])),
            make_note(ids[3], "", Some(ids[2])),
        ];
        assert_eq!(allowed.len(), MAX_DEPTH + 1);
        assert!(validate_notes(&allowed).is_ok());

        // Depth MAX_DEPTH + 2 exceeds limit.
        let mut too_deep = allowed.clone();
        too_deep.push(make_note(ids[4], "", Some(ids[3])));
        assert_eq!(
            validate_notes(&too_deep),
            Err("project nesting exceeds depth limit".into())
        );

        // Cycle detection.
        let cycle = vec![
            make_note(ids[0], "", Some(ids[1])),
            make_note(ids[1], "", Some(ids[0])),
        ];
        assert_eq!(validate_notes(&cycle), Err("project cycle".into()));

        // Invalid edge.
        let mut bad_edge = vec![make_note(ids[0], "", None)];
        bad_edge[0].canvas.next.push(ids[1].into());
        assert_eq!(validate_notes(&bad_edge), Err("invalid canvas edge".into()));
    }
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
