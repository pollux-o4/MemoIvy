//! 프로젝트 방문 기록과 바로가기 순위. 메모 원본과 분리된 파생 데이터라
//! 읽기 실패·손상 시 빈 기록으로 시작하고, 저장 실패는 메모 저장을 막지 않는다.
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const DAY: u64 = 24 * 60 * 60;
/// 오래된 방문을 끝없이 쌓지 않는다. 순위에 필요한 만큼만 남긴다.
const KEEP: usize = 64;

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Debug)]
struct Visit {
    count: u32,
    last: u64,
}

pub struct Navigation {
    path: PathBuf,
    visits: HashMap<Uuid, Visit>,
}

/// 브라우저 주소창식 frecency: 방문 수에 최근성 구간 가중치를 곱한다.
fn score(visit: Visit, now: u64) -> f64 {
    let age = now.saturating_sub(visit.last) / DAY;
    let weight = match age {
        0..4 => 1.0,
        4..14 => 0.7,
        14..31 => 0.5,
        31..90 => 0.3,
        _ => 0.1,
    };
    f64::from(visit.count) * weight
}

impl Navigation {
    /// 메모 파일 옆의 방문 기록을 연다.
    pub fn beside(notes: &Path) -> Self {
        let path = notes.with_file_name("navigation.json");
        let visits = fs::read(&path)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        Self { path, visits }
    }

    pub fn record(&mut self, id: Uuid, now: u64) {
        let visit = self.visits.entry(id).or_default();
        visit.count = visit.count.saturating_add(1);
        visit.last = now;
        if self.visits.len() > KEEP {
            let weakest = self
                .visits
                .iter()
                .filter(|(other, _)| **other != id)
                .min_by(|a, b| score(*a.1, now).total_cmp(&score(*b.1, now)))
                .map(|(id, _)| *id);
            if let Some(weakest) = weakest {
                self.visits.remove(&weakest);
            }
        }
        if let Err(error) = self.save() {
            eprintln!("방문 기록을 저장하지 못했습니다: {error}");
        }
    }

    pub fn forget(&mut self, id: Uuid) {
        if self.visits.remove(&id).is_some() {
            let _ = self.save();
        }
    }

    /// 점수가 높은 순. 같은 점수면 최근 방문이 앞이다. keep이 거짓인 항목은 건너뛴다.
    pub fn ranked(&self, now: u64, keep: impl Fn(Uuid) -> bool) -> Vec<Uuid> {
        let mut ranked: Vec<_> = self
            .visits
            .iter()
            .filter(|(id, _)| keep(**id))
            .map(|(id, visit)| (*id, score(*visit, now), visit.last))
            .collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(b.2.cmp(&a.2)));
        ranked.into_iter().map(|(id, ..)| id).collect()
    }

    fn save(&self) -> std::io::Result<()> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| std::io::Error::other("저장 폴더가 없습니다"))?;
        fs::create_dir_all(dir)?;
        let temp = dir.join(format!(".navigation-{}.tmp", Uuid::new_v4()));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temp)?;
            serde_json::to_writer(&mut file, &self.visits)?;
            file.flush()?;
            fs::rename(&temp, &self.path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_001_frequent_recent_projects_rank_first_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes.json");
        let (old, recent, gone) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let now = 100 * DAY;
        let mut nav = Navigation::beside(&notes);
        for _ in 0..5 {
            nav.record(old, now - 60 * DAY);
        }
        for _ in 0..2 {
            nav.record(recent, now - DAY);
        }
        nav.record(gone, now);
        let reopened = Navigation::beside(&notes);
        assert_eq!(reopened.ranked(now, |id| id != gone), [recent, old]);
        let mut reopened = reopened;
        reopened.forget(recent);
        assert_eq!(
            Navigation::beside(&notes).ranked(now, |_| true),
            [old, gone]
        );
    }

    #[test]
    fn nav_001_corrupt_history_starts_empty_and_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes.json");
        fs::write(dir.path().join("navigation.json"), b"{broken").unwrap();
        let mut nav = Navigation::beside(&notes);
        assert!(nav.ranked(0, |_| true).is_empty());
        nav.record(Uuid::new_v4(), 0);
        let mode = fs::metadata(dir.path().join("navigation.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0);
    }
}
