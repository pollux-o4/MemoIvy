//! 사용처 색인. 원본은 메모 본문이고 이 파일은 이력(마지막 메모·사용 안 함 시각)만 더한다.
use super::files::AssetId;
use crate::persistence::note::Note;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct Usage {
    pub added: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_note: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unused_since: Option<u64>,
}

#[derive(Default, Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct Index {
    #[serde(default)]
    pub auto_prune: bool,
    #[serde(default)]
    pub assets: BTreeMap<String, Usage>,
}

/// 본문에 나온 `asset:<해시>`. 문법이 깨진 줄도 사용 중으로 센다(DATA-003).
pub(super) fn references(text: &str) -> BTreeSet<AssetId> {
    text.match_indices(super::SCHEME)
        .filter_map(|(at, scheme)| {
            let start = at + scheme.len();
            AssetId::parse(text.get(start..start + 64)?)
        })
        .collect()
}

/// 해시마다 쓰는 메모. 최근에 고친 메모가 앞이다.
pub(super) fn users(notes: &[Note]) -> BTreeMap<AssetId, Vec<Uuid>> {
    let mut sorted: Vec<_> = notes.iter().collect();
    sorted.sort_by_key(|note| std::cmp::Reverse(note.modified));
    let mut users: BTreeMap<AssetId, Vec<Uuid>> = BTreeMap::new();
    for note in sorted {
        for id in references(&note.text) {
            users.entry(id).or_default().push(note.id);
        }
    }
    users
}

impl Index {
    /// 폴더와 본문에 맞춘다. 바뀌었으면 참.
    pub fn reconcile(
        &mut self,
        files: &[AssetId],
        users: &BTreeMap<AssetId, Vec<Uuid>>,
        now: u64,
    ) -> bool {
        let before = self.assets.clone();
        let present: BTreeSet<_> = files.iter().map(AssetId::as_str).collect();
        self.assets.retain(|id, _| present.contains(id.as_str()));
        for id in files {
            let usage = self.assets.entry(id.to_string()).or_insert(Usage {
                added: now,
                ..Usage::default()
            });
            match users.get(id).and_then(|users| users.first()) {
                Some(note) => {
                    usage.last_note = Some(*note);
                    usage.unused_since = None;
                }
                None => {
                    usage.unused_since.get_or_insert(now);
                }
            }
        }
        before != self.assets
    }
}
