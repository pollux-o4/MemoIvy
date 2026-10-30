//! 캔버스가 저장 계층을 만나는 유일한 경계. 저장 형식이 바뀌어도 이 파일만 고친다.
use super::model::layout::{Moves, Rect};
use crate::{
    app::state::AppState,
    persistence::{
        canvas::{self as backend, CanvasNode},
        store::Note,
    },
    presentation::markdown,
};
use std::collections::HashMap;
use uuid::Uuid;

pub(super) const CARD_WIDTH: f64 = backend::WIDTH;
pub(super) const CARD_HEIGHT: f64 = backend::HEIGHT;
pub(super) const LIMIT: f64 = backend::LIMIT;

pub(super) type Notes = Vec<Note>;
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
        .retain(|id, _| state.store.notes.iter().any(|note| note.id == *id));
    state
        .store
        .notes
        .iter()
        .enumerate()
        .map(|(index, note)| {
            let entry = cache.0.entry(note.id).or_default();
            if entry.0 != note.text || entry.0.is_empty() {
                *entry = (
                    note.text.clone(),
                    markdown::summary(&note.text),
                    markdown::task_progress(&note.text),
                );
            }
            NodeView {
                id: note.id,
                title: entry.1.title.clone(),
                preview: entry.1.preview.clone(),
                color: note.color.clone(),
                position: backend::position(note, index),
                next: note.canvas.next.clone(),
                done: note.canvas.done,
                tasks: entry.2,
            }
        })
        .collect()
}

pub(super) fn snapshot(state: &AppState) -> Snapshot {
    state
        .store
        .notes
        .iter()
        .map(|n| (n.id, n.canvas.clone()))
        .collect()
}

/// 배치·연결·완료만 복원한다. 그사이 삭제된 메모와 그 메모로 가는 연결은 되살리지 않는다.
pub(super) fn restore(notes: &mut Notes, snapshot: &Snapshot) {
    let ids: Vec<Uuid> = notes.iter().map(|n| n.id).collect();
    for note in notes.iter_mut() {
        if let Some((_, node)) = snapshot.iter().find(|(id, _)| *id == note.id) {
            note.canvas = node.clone();
        }
        note.canvas.next.retain(|id| ids.contains(id));
    }
}

pub(super) fn move_nodes(notes: &mut Notes, moves: &Moves) {
    for note in notes.iter_mut() {
        if let Some((_, p)) = moves.iter().find(|(id, _)| *id == note.id) {
            note.canvas.position = Some([p[0].clamp(0., LIMIT), p[1].clamp(0., LIMIT)]);
        }
    }
}

pub(super) fn connect(notes: &mut Notes, from: Uuid, to: Uuid) -> bool {
    backend::connect(notes, from, to)
}

pub(super) fn can_connect(nodes: &[NodeView], from: Uuid, to: Uuid) -> bool {
    from != to
        && nodes
            .iter()
            .find(|n| n.id == from)
            .is_some_and(|n| !n.next.contains(&to))
        && nodes.iter().any(|n| n.id == to)
}

pub(super) fn disconnect(notes: &mut Notes, from: Uuid, to: Uuid) {
    if let Some(note) = notes.iter_mut().find(|n| n.id == from) {
        note.canvas.next.retain(|id| *id != to);
    }
}

pub(super) fn set_done(notes: &mut Notes, id: Uuid, done: bool) {
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        note.canvas.done = done;
    }
}
