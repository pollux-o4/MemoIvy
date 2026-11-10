use super::super::{canvas::valid_position, canvas::CanvasNode, links, nesting, note::Note};
use std::collections::HashSet;
use uuid::Uuid;

/// Transactional editor for canvas-only note state.
pub struct CanvasEdit<'a> {
    pub(super) notes: &'a mut Vec<Note>,
    pub(super) changed: bool,
}

impl CanvasEdit<'_> {
    pub fn set_position(&mut self, id: Uuid, position: [f64; 2]) {
        if !valid_position(position) {
            return;
        }
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            if note.canvas.position != Some(position) {
                note.canvas.position = Some(position);
                self.changed = true;
            }
        }
    }

    pub fn connect(&mut self, from: Uuid, to: Uuid) -> bool {
        let changed = links::connect(self.notes, from, to);
        self.changed |= changed;
        changed
    }

    pub fn disconnect(&mut self, from: Uuid, to: Uuid) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == from) {
            let len = note.canvas.next.len();
            note.canvas.next.retain(|id| *id != to);
            self.changed |= len != note.canvas.next.len();
        }
    }

    /// 하위 메모와 함께 parent 안으로 옮긴다. 순환·깊이 초과면 그대로 두고 거짓을 돌려준다.
    pub fn set_parent(&mut self, id: Uuid, parent: Option<Uuid>) -> bool {
        if !nesting::can_nest(self.notes, id, parent) {
            return false;
        }
        let note = self.notes.iter_mut().find(|n| n.id == id).unwrap();
        let changed = note.canvas.parent != parent;
        note.canvas.parent = parent;
        self.changed |= changed;
        changed
    }

    pub fn set_pinned(&mut self, id: Uuid, pinned: bool) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            self.changed |= note.canvas.pinned != pinned;
            note.canvas.pinned = pinned;
        }
    }

    pub fn set_done(&mut self, id: Uuid, done: bool) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            self.changed |= note.canvas.done != done;
            note.canvas.done = done;
        }
    }

    /// 포함 관계가 깨지는 스냅숏이면 포함 관계만 현재대로 둔다.
    pub fn restore(&mut self, snapshot: &[(Uuid, CanvasNode)]) {
        let ids: HashSet<_> = self.notes.iter().map(|n| n.id).collect();
        let parents: Vec<_> = self.notes.iter().map(|n| n.canvas.parent).collect();
        for note in self.notes.iter_mut() {
            if let Some((_, node)) = snapshot.iter().find(|(id, _)| *id == note.id) {
                if node.position.is_some_and(|p| !valid_position(p)) {
                    continue;
                }
                let mut node = node.clone();
                let mut seen = HashSet::new();
                node.next
                    .retain(|id| *id != note.id && ids.contains(id) && seen.insert(*id));
                self.changed |= note.canvas != node;
                note.canvas = node;
            }
        }
        if !nesting::valid(self.notes) {
            for (note, parent) in self.notes.iter_mut().zip(parents) {
                note.canvas.parent = parent;
            }
        }
    }
}
