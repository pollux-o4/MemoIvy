//! 메모 첨부 파일(DATA-003, ADR-0011). 파일은 내용 해시로 한 번만 저장하고,
//! 사용 여부는 메모 본문에서 다시 계산한다. 색인은 파생 데이터라 손상되면 다시 만든다.
mod files;
mod usage;

pub use files::{AssetId, AssetStore, ImportError};

use super::{note::Note, store::write_atomic};
use std::{fs, path::Path, path::PathBuf};
use usage::Index;
use uuid::Uuid;

/// 본문 참조 표기의 앞부분.
const SCHEME: &str = "asset:";
/// 자동 정리를 켰을 때 사용 안 함 파일을 남겨 두는 기간.
pub const PRUNE_AFTER: u64 = 30 * 24 * 60 * 60;

/// 파일 화면이 보여 줄 읽기 모델.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetEntry {
    pub id: AssetId,
    pub path: PathBuf,
    pub bytes: u64,
    pub added: u64,
    /// 지금 쓰는 메모. 최근에 고친 메모가 앞이다. 비어 있으면 사용 안 함이다.
    pub users: Vec<Uuid>,
    /// 사용 안 함이면 마지막으로 쓴 메모. 이미 지워졌을 수 있다.
    pub last_note: Option<Uuid>,
    pub unused_since: Option<u64>,
}

pub struct Attachments {
    store: AssetStore,
    path: PathBuf,
    index: Index,
}

impl Attachments {
    /// 메모 파일 옆의 assets 폴더와 색인을 연다.
    pub fn beside(notes: &Path) -> Self {
        let path = notes.with_file_name("assets.json");
        let index = fs::read(&path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        Self {
            store: AssetStore::new(notes.with_file_name("assets")),
            path,
            index,
        }
    }

    pub fn store(&self) -> AssetStore {
        self.store.clone()
    }

    /// 메모 저장·삭제·시작 때 부른다. 사용처가 바뀌었으면 참.
    pub fn reconcile(&mut self, notes: &[Note], now: u64) -> bool {
        let files: Vec<_> = self.store.list().into_iter().map(|(id, _)| id).collect();
        let changed = self.index.reconcile(&files, &usage::users(notes), now);
        if changed {
            self.save();
        }
        changed
    }

    /// 사용 중이 앞, 그 안에서는 최근 순이다.
    pub fn entries(&self, notes: &[Note]) -> Vec<AssetEntry> {
        let users = usage::users(notes);
        let modified = |id: &Uuid| {
            notes
                .iter()
                .find(|note| note.id == *id)
                .map_or(0, |note| note.modified)
        };
        let mut entries: Vec<_> = self
            .store
            .list()
            .into_iter()
            .filter_map(|(id, bytes)| {
                let usage = self
                    .index
                    .assets
                    .get(id.as_str())
                    .copied()
                    .unwrap_or_default();
                let users = users.get(&id).cloned().unwrap_or_default();
                Some(AssetEntry {
                    path: self.store.path(&id)?,
                    unused_since: if users.is_empty() {
                        usage.unused_since
                    } else {
                        None
                    },
                    last_note: users.first().copied().or(usage.last_note),
                    id,
                    bytes,
                    added: usage.added,
                    users,
                })
            })
            .collect();
        entries.sort_by_key(|entry| {
            let recent = entry
                .users
                .first()
                .map_or(entry.unused_since.unwrap_or(entry.added), modified);
            (entry.users.is_empty(), std::cmp::Reverse(recent))
        });
        entries
    }

    /// 사용 안 함인 파일만 지운다. 확인창 사이에 다시 쓰였으면 거부한다.
    pub fn delete(&mut self, id: &AssetId, notes: &[Note]) -> Result<(), String> {
        if usage::users(notes).contains_key(id) {
            return Err("메모에서 쓰고 있어 지우지 않았어요".into());
        }
        self.store
            .remove(id)
            .map_err(|e| format!("파일을 지우지 못했어요: {e}"))?;
        if self.index.assets.remove(id.as_str()).is_some() {
            self.save();
        }
        Ok(())
    }

    /// 사용 안 함 전부. 지운 개수를 돌려준다.
    pub fn delete_unused(&mut self, notes: &[Note]) -> Result<usize, String> {
        let unused: Vec<_> = self
            .entries(notes)
            .into_iter()
            .filter(|entry| entry.users.is_empty())
            .map(|entry| entry.id)
            .collect();
        for id in &unused {
            self.delete(id, notes)?;
        }
        Ok(unused.len())
    }

    pub fn auto_prune(&self) -> bool {
        self.index.auto_prune
    }

    pub fn set_auto_prune(&mut self, on: bool) {
        if self.index.auto_prune != on {
            self.index.auto_prune = on;
            self.save();
        }
    }

    /// 자동 정리가 켜져 있으면 PRUNE_AFTER가 지난 사용 안 함 파일을 지운다. 앱 시작 때 부른다.
    pub fn prune(&mut self, notes: &[Note], now: u64) -> usize {
        if !self.index.auto_prune {
            return 0;
        }
        let expired: Vec<_> = self
            .entries(notes)
            .into_iter()
            .filter(|entry| {
                entry
                    .unused_since
                    .is_some_and(|since| now.saturating_sub(since) >= PRUNE_AFTER)
            })
            .map(|entry| entry.id)
            .collect();
        expired
            .iter()
            .filter(|id| self.delete(id, notes).is_ok())
            .count()
    }

    fn save(&self) {
        let result = write_atomic(&self.path, |file| {
            serde_json::to_writer(&mut *file, &self.index)?;
            Ok(())
        });
        if let Err(error) = result {
            eprintln!("첨부 색인을 저장하지 못했습니다: {error}");
        }
    }
}

#[cfg(test)]
mod tests;
