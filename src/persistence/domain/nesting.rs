//! 메모 안에 메모를 담는 포함 관계와 프로젝트 정체성.
use super::note::Note;
use std::collections::HashMap;
use uuid::Uuid;

// shortcut: 서랍이 깊어질수록 길을 잃는다. 바로가기로도 부족한 깊이가 필요하면 상한을 다시 본다.
/// 최상위를 0으로 센 최대 깊이. 최상위 아래로 세 단계까지 담는다.
pub const MAX_DEPTH: usize = 3;

fn parents(notes: &[Note]) -> HashMap<Uuid, Option<Uuid>> {
    notes.iter().map(|n| (n.id, n.canvas.parent)).collect()
}

/// 조상 수. 부모가 없거나 순환이면 None이다.
fn depth(parents: &HashMap<Uuid, Option<Uuid>>, id: Uuid) -> Option<usize> {
    let mut current = id;
    for depth in 0..=parents.len() {
        match parents.get(&current)? {
            None => return Some(depth),
            Some(parent) => current = *parent,
        }
    }
    None
}

/// id 아래로 가장 깊은 후손까지의 단계 수. 자식이 없으면 0이다.
fn height(notes: &[Note], id: Uuid) -> usize {
    notes
        .iter()
        .filter(|n| n.canvas.parent == Some(id))
        .map(|n| 1 + height(notes, n.id))
        .max()
        .unwrap_or(0)
}

fn contains(parents: &HashMap<Uuid, Option<Uuid>>, ancestor: Uuid, id: Uuid) -> bool {
    let mut current = Some(id);
    for _ in 0..=parents.len() {
        match current {
            Some(c) if c == ancestor => return true,
            Some(c) => current = parents.get(&c).copied().flatten(),
            None => return false,
        }
    }
    false
}

pub fn valid(notes: &[Note]) -> bool {
    let parents = parents(notes);
    notes.iter().all(|n| {
        n.canvas.parent != Some(n.id) && depth(&parents, n.id).is_some_and(|d| d <= MAX_DEPTH)
    })
}

/// 새 메모를 parent 안에 만들 수 있는가.
pub fn can_create_in(notes: &[Note], parent: Option<Uuid>) -> bool {
    let Some(parent) = parent else { return true };
    depth(&parents(notes), parent).is_some_and(|d| d < MAX_DEPTH)
}

/// 새 프로젝트가 최소 한 단계의 내용을 담을 수 있어야 한다.
pub fn can_create_project_in(notes: &[Note], parent: Option<Uuid>) -> bool {
    parent.is_none_or(|id| depth(&parents(notes), id).is_some_and(|d| d + 1 < MAX_DEPTH))
}

/// 포함 관계·기존 고정 기록을 명시적 프로젝트로 승격하고, 비워져도 유지한다.
pub fn promote_projects(notes: &mut [Note]) -> bool {
    let parents: std::collections::HashSet<_> =
        notes.iter().filter_map(|n| n.canvas.parent).collect();
    let mut changed = false;
    for note in notes {
        if (parents.contains(&note.id) || note.canvas.pinned) && !note.canvas.project {
            note.canvas.project = true;
            changed = true;
        }
    }
    changed
}

/// id를 하위 메모와 함께 parent 안으로 옮길 수 있는가.
pub fn can_nest(notes: &[Note], id: Uuid, parent: Option<Uuid>) -> bool {
    let parents = parents(notes);
    if !parents.contains_key(&id) {
        return false;
    }
    let Some(parent) = parent else { return true };
    parents.contains_key(&parent)
        && !contains(&parents, id, parent)
        && depth(&parents, parent).is_some_and(|d| d + 1 + height(notes, id) <= MAX_DEPTH)
}

/// 지운 메모의 자식은 한 단계 위로 올린다. 상자를 버려도 안의 메모는 남는다.
pub fn lift_children(notes: &mut [Note], removed: Uuid) {
    let Some(parent) = notes
        .iter()
        .find(|n| n.id == removed)
        .map(|n| n.canvas.parent)
    else {
        return;
    };
    for note in notes
        .iter_mut()
        .filter(|n| n.canvas.parent == Some(removed))
    {
        note.canvas.parent = parent;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(len: usize) -> Vec<Note> {
        let mut notes: Vec<_> = (0..len).map(|_| Note::new()).collect();
        for i in 1..len {
            notes[i].canvas.parent = Some(notes[i - 1].id);
        }
        notes
    }

    #[test]
    fn project_001_depth_limit_and_cycles() {
        let notes = chain(MAX_DEPTH + 1);
        assert!(valid(&notes));
        assert!(!can_create_in(&notes, Some(notes[MAX_DEPTH].id)));
        assert!(can_create_in(&notes, Some(notes[MAX_DEPTH - 1].id)));
        assert!(!valid(&chain(MAX_DEPTH + 2)));
        let mut cycle = chain(2);
        cycle[0].canvas.parent = Some(cycle[1].id);
        assert!(!valid(&cycle));
        let mut own = chain(1);
        own[0].canvas.parent = Some(own[0].id);
        assert!(!valid(&own));
        let mut missing = chain(1);
        missing[0].canvas.parent = Some(Uuid::new_v4());
        assert!(!valid(&missing));
    }

    #[test]
    fn project_001_nesting_keeps_subtrees_within_limit() {
        let mut notes = chain(3);
        notes.push(Note::new());
        let (root, mid, leaf, loose) = (notes[0].id, notes[1].id, notes[2].id, notes[3].id);
        assert!(!can_nest(&notes, root, Some(leaf)));
        assert!(!can_nest(&notes, root, Some(root)));
        assert!(can_nest(&notes, loose, Some(leaf)));
        assert!(can_nest(&notes, root, Some(loose)));
        assert!(can_nest(&notes, mid, None));
        assert!(!can_nest(&notes, Uuid::new_v4(), None));
        let mut deep = chain(MAX_DEPTH + 1);
        deep.push(Note::new());
        assert!(!can_nest(&deep, deep[0].id, Some(deep[MAX_DEPTH + 1].id)));
    }

    #[test]
    fn project_001_removing_a_project_lifts_its_children() {
        let mut notes = chain(3);
        let (root, mid, leaf) = (notes[0].id, notes[1].id, notes[2].id);
        lift_children(&mut notes, mid);
        assert_eq!(notes[2].canvas.parent, Some(root));
        assert_eq!(notes[2].id, leaf);
    }
}
