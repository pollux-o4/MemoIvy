//! 캔버스가 저장 계층을 만나는 유일한 경계. 저장 형식이 바뀌어도 이 파일만 고친다.
use super::model::layout::{Moves, Rect};
pub(in crate::app) use crate::persistence::Notebook;
use crate::{
    app::shell::state::AppState,
    persistence::{CanvasEdit, CanvasNode},
    presentation::markdown,
};
use std::collections::HashMap;
use uuid::Uuid;

pub(super) use super::model::placement::{CARD_HEIGHT, CARD_WIDTH};
pub(super) const LIMIT: f64 = crate::persistence::LIMIT;

pub(super) type Notes<'a> = CanvasEdit<'a>;
pub(super) type Snapshot = Vec<(Uuid, CanvasNode)>;

/// 화면이 읽는 메모 표현. 원문·저장 구조를 뷰에 노출하지 않는다.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct NodeView {
    pub id: Uuid,
    pub title: String,
    pub preview: String,
    pub color: String,
    pub position: [f64; 2],
    pub next: Vec<Uuid>,
    pub done: bool,
    pub tasks: (usize, usize),
}

impl NodeView {
    pub fn rect(&self) -> Rect {
        rect_at(self.position)
    }
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            "빈 메모"
        } else {
            &self.title
        }
    }
}

pub(super) fn rect_at(position: [f64; 2]) -> Rect {
    Rect::new(position[0], position[1], CARD_WIDTH, CARD_HEIGHT)
}

/// 본문이 그대로면 요약을 다시 파싱하지 않는다. 입력마다 전체 메모를 읽지 않기 위한 캐시다.
#[derive(Default)]
pub(super) struct SummaryCache(HashMap<Uuid, (String, markdown::Summary, (usize, usize))>);

pub(super) fn nodes(state: &AppState, cache: &mut SummaryCache) -> Vec<NodeView> {
    cache
        .0
        .retain(|id, _| state.notebook.notes().iter().any(|note| note.id() == *id));
    state
        .notebook
        .notes()
        .iter()
        .enumerate()
        .map(|(index, note)| {
            let entry = cache.0.entry(note.id()).or_default();
            if entry.0 != note.text() || entry.0.is_empty() {
                *entry = (
                    note.text().to_owned(),
                    markdown::summary(note.text()),
                    markdown::task_progress(note.text()),
                );
            }
            NodeView {
                id: note.id(),
                title: entry.1.title.clone(),
                preview: entry.1.preview.clone(),
                color: note.color().to_owned(),
                position: note.canvas().position.unwrap_or_else(|| slot(index)),
                next: note.canvas().next.clone(),
                done: note.canvas().done,
                tasks: entry.2,
            }
        })
        .collect()
}

pub(super) fn snapshot(state: &AppState) -> Snapshot {
    state
        .notebook
        .notes()
        .iter()
        .map(|n| (n.id(), n.canvas().clone()))
        .collect()
}

/// 배치·연결·완료만 복원한다. 그사이 삭제된 메모와 그 메모로 가는 연결은 되살리지 않는다.
pub(super) fn restore(notes: &mut Notes<'_>, snapshot: &Snapshot) {
    notes.restore(snapshot);
}

pub(super) fn move_nodes(notes: &mut Notes<'_>, moves: &Moves) {
    for (id, p) in moves {
        notes.set_position(*id, [p[0].clamp(0., LIMIT), p[1].clamp(0., LIMIT)]);
    }
}

pub(super) fn connect(notes: &mut Notes<'_>, from: Uuid, to: Uuid) -> bool {
    notes.connect(from, to)
}

pub(super) fn can_connect(nodes: &[NodeView], from: Uuid, to: Uuid) -> bool {
    from != to
        && nodes
            .iter()
            .find(|n| n.id == from)
            .is_some_and(|n| !n.next.contains(&to))
        && nodes.iter().any(|n| n.id == to)
}

pub(super) fn disconnect(notes: &mut Notes<'_>, from: Uuid, to: Uuid) {
    notes.disconnect(from, to);
}

pub(super) fn set_done(notes: &mut Notes<'_>, id: Uuid, done: bool) {
    notes.set_done(id, done);
}

fn slot(index: usize) -> [f64; 2] {
    super::model::placement::slot(index)
}

pub(super) fn vacant(notebook: &Notebook) -> [f64; 2] {
    super::model::placement::vacant(
        notebook
            .notes()
            .iter()
            .enumerate()
            .map(|(i, n)| n.canvas().position.unwrap_or_else(|| slot(i))),
    )
}

pub(super) fn initialize_positions(notebook: &mut Notebook) {
    let missing: Vec<_> = notebook
        .notes()
        .iter()
        .enumerate()
        .filter(|(_, note)| note.canvas().position.is_none())
        .map(|(i, note)| (note.id(), slot(i)))
        .collect();
    notebook.edit_canvas(|canvas| {
        for (id, position) in missing {
            canvas.set_position(id, position);
        }
    });
}

pub(super) fn saved(state: &AppState) -> bool {
    matches!(
        state.notebook.status(),
        crate::persistence::SaveStatus::Saved
    )
}
pub(super) fn failed(state: &AppState) -> bool {
    matches!(
        state.notebook.status(),
        crate::persistence::SaveStatus::Failed(_)
    )
}
