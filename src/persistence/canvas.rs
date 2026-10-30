use super::store::Note;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// shortcut: 로컬 단일 보드 좌표를 제한해 잘못된 데이터와 과도한 영역을 방지, 다중 프로젝트 또는 대형 보드 요구 시 공간 모델 재검토.
pub const LIMIT: f64 = 20_000.;
pub const WIDTH: f64 = 240.;
pub const HEIGHT: f64 = 156.;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
#[serde(default)]
pub struct CanvasNode {
    pub position: Option<[f64; 2]>,
    pub next: Vec<Uuid>,
    pub done: bool,
}

pub fn position(note: &Note, index: usize) -> [f64; 2] {
    note.canvas.position.unwrap_or_else(|| slot(index))
}

fn slot(index: usize) -> [f64; 2] {
    [
        64. + ((index % 4 + (index / 336) * 4) % 60) as f64 * 320.,
        64. + (index / 4 % 84) as f64 * 232.,
    ]
}

pub fn vacant(notes: &[Note]) -> [f64; 2] {
    (0..5040)
        .map(slot)
        .find(|p| {
            notes.iter().enumerate().all(|(i, n)| {
                let q = position(n, i);
                (p[0] - q[0]).abs() >= WIDTH + 24. || (p[1] - q[1]).abs() >= HEIGHT + 24.
            })
        })
        .unwrap_or([64., 64.])
}

pub use super::links::{connect, remove, valid};
