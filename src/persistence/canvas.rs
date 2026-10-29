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

pub fn connect(notes: &mut [Note], from: Uuid, to: Uuid) -> bool {
    if from == to || !notes.iter().any(|n| n.id == to) {
        return false;
    }
    let Some(note) = notes.iter_mut().find(|n| n.id == from) else {
        return false;
    };
    if note.canvas.next.contains(&to) {
        return false;
    }
    note.canvas.next.push(to);
    true
}

pub fn remove(notes: &mut Vec<Note>, id: Uuid) {
    notes.retain(|n| n.id != id);
    for note in notes {
        note.canvas.next.retain(|next| *next != id);
    }
}

pub fn valid(notes: &[Note]) -> bool {
    let ids: std::collections::HashSet<_> = notes.iter().map(|n| n.id).collect();
    notes.iter().all(|n| {
        let mut seen = std::collections::HashSet::new();
        n.canvas
            .position
            .is_none_or(|p| p.iter().all(|v| v.is_finite() && (0. ..=LIMIT).contains(v)))
            && n.canvas
                .next
                .iter()
                .all(|id| *id != n.id && ids.contains(id) && seen.insert(*id))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_001_identity_edges_and_delete() {
        let mut notes = vec![Note::new(), Note::new(), Note::new()];
        let (a, b, c) = (notes[0].id, notes[1].id, notes[2].id);
        assert!(!connect(&mut notes, a, a));
        assert!(!connect(&mut notes, a, Uuid::new_v4()));
        assert!(connect(&mut notes, a, b));
        assert!(!connect(&mut notes, a, b));
        assert!(connect(&mut notes, b, a));
        assert!(connect(&mut notes, c, b));
        assert!(valid(&notes));
        remove(&mut notes, b);
        assert_eq!(notes.len(), 2);
        assert!(notes.iter().all(|n| n.canvas.next.is_empty()));
        assert!(valid(&notes));
    }

    #[test]
    fn canvas_001_rejects_invalid_graph_and_positions() {
        let mut notes = vec![Note::new()];
        for p in [[f64::NAN, 0.], [-1., 0.], [LIMIT + 1., 0.]] {
            notes[0].canvas.position = Some(p);
            assert!(!valid(&notes));
        }
        notes[0].canvas.position = Some([0., LIMIT]);
        assert!(valid(&notes));
        notes[0].canvas.next.push(Uuid::new_v4());
        assert!(!valid(&notes));
    }
}
