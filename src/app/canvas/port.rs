//! 캔버스가 저장 계층을 만나는 유일한 경계. 저장 형식이 바뀌어도 이 파일만 고친다.
use super::model::layout::{Moves, Rect};
pub(in crate::app) use crate::persistence::Notebook;
use crate::{
    app::shell::state::AppState,
    persistence::{CanvasEdit, CanvasNode},
    presentation::markdown,
};
use std::{collections::HashMap, rc::Rc};
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
    /// 다른 프로젝트 안쪽 메모로 이어지는 연결을 이 단계의 카드끼리로 투영한 것. 편집할 수 없다.
    pub through: Vec<Uuid>,
    pub done: bool,
    pub pinned: bool,
    /// 자식이 있으면 프로젝트다. 하위 전체의 메모 수와 할 일 진행.
    pub project: Option<ProjectSummary>,
    pub tasks: (usize, usize),
    pub title_marker: Option<markdown::LineMarker>,
    pub lines: Rc<Vec<markdown::PreviewLine>>,
    /// 요약에 담지 않은 나머지 내용 줄 수.
    pub remaining: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct ProjectSummary {
    pub notes: usize,
    pub tasks: (usize, usize),
}

/// 경로·바로가기에 쓰는 프로젝트 이름표.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Place {
    pub id: Uuid,
    pub title: String,
    pub color: String,
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
pub(super) struct SummaryCache(HashMap<Uuid, CachedSummary>);

#[derive(Default)]
struct CachedSummary {
    text: String,
    summary: markdown::Summary,
    tasks: (usize, usize),
    lines: Rc<Vec<markdown::PreviewLine>>,
}

/// scope 단계에 보이는 카드. 하위 메모의 연결은 그 메모를 담은 이 단계 카드로 투영한다.
pub(super) fn nodes(
    state: &AppState,
    cache: &mut SummaryCache,
    scope: Option<Uuid>,
) -> Vec<NodeView> {
    let notes = state.notebook.notes();
    let ids: std::collections::HashSet<_> = notes.iter().map(|note| note.id()).collect();
    cache.0.retain(|id, _| ids.contains(id));
    for note in notes {
        let entry = cache.0.entry(note.id()).or_default();
        if entry.text != note.text() || entry.text.is_empty() {
            let summary = markdown::summary(note.text());
            *entry = CachedSummary {
                text: note.text().to_owned(),
                lines: Rc::new(summary.lines.clone()),
                summary,
                tasks: markdown::task_progress(note.text()),
            };
        }
    }
    let parents: HashMap<Uuid, Option<Uuid>> =
        notes.iter().map(|n| (n.id(), n.canvas().parent)).collect();
    // 이 단계에서 메모를 대표하는 카드. scope 바깥 메모는 None이다.
    let shown_as = |id: Uuid| -> Option<Uuid> {
        let mut current = id;
        for _ in 0..=parents.len() {
            let parent = *parents.get(&current)?;
            if parent == scope {
                return Some(current);
            }
            current = parent?;
        }
        None
    };
    let mut projects: HashMap<Uuid, ProjectSummary> = HashMap::new();
    let mut through: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for note in notes {
        let Some(card) = shown_as(note.id()) else {
            continue;
        };
        if card != note.id() {
            let tasks = cache.0[&note.id()].tasks;
            let project = projects.entry(card).or_insert(ProjectSummary {
                notes: 0,
                tasks: (0, 0),
            });
            project.notes += 1;
            project.tasks.0 += tasks.0;
            project.tasks.1 += tasks.1;
        }
        for target in &note.canvas().next {
            let Some(to) = shown_as(*target) else {
                continue;
            };
            let direct = card == note.id() && to == *target;
            let list = through.entry(card).or_default();
            if !direct && to != card && !list.contains(&to) {
                list.push(to);
            }
        }
    }
    notes
        .iter()
        .enumerate()
        .filter(|(_, note)| note.canvas().parent == scope)
        .map(|(index, note)| {
            let entry = &cache.0[&note.id()];
            let next = note.canvas().next.clone();
            let mut through = through.remove(&note.id()).unwrap_or_default();
            through.retain(|id| !next.contains(id));
            NodeView {
                id: note.id(),
                title: entry.summary.title.clone(),
                preview: entry.summary.preview.clone(),
                color: note.color().to_owned(),
                position: note.canvas().position.unwrap_or_else(|| slot(index)),
                next,
                through,
                done: note.canvas().done,
                pinned: note.canvas().pinned,
                project: projects.get(&note.id()).copied(),
                tasks: entry.tasks,
                title_marker: entry.summary.title_marker.clone(),
                lines: entry.lines.clone(),
                remaining: entry.summary.remaining,
            }
        })
        .collect()
}

/// 메모 → 부모. 경로·깊이·프로젝트 판별을 메모 수만큼 한 번에 읽는다(`note()`는 선형 탐색이다).
fn parents(state: &AppState) -> HashMap<Uuid, Option<Uuid>> {
    state
        .notebook
        .notes()
        .iter()
        .map(|n| (n.id(), n.canvas().parent))
        .collect()
}

fn projects(parents: &HashMap<Uuid, Option<Uuid>>) -> std::collections::HashSet<Uuid> {
    parents.values().filter_map(|p| *p).collect()
}

fn place(state: &AppState, id: Uuid) -> Option<Place> {
    let note = state.notebook.note(id)?;
    // 제목은 첫 줄에서 나온다. 본문 전체를 파싱하지 않는다.
    let first = note
        .text()
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let title = markdown::summary(first).title;
    Some(Place {
        id,
        title: if title.is_empty() {
            "이름 없는 프로젝트".into()
        } else {
            title
        },
        color: note.color().to_owned(),
    })
}

fn ancestors(parents: &HashMap<Uuid, Option<Uuid>>, id: Uuid) -> impl Iterator<Item = Uuid> + '_ {
    // 저장 계층이 순환을 거부하지만 화면은 깊이 상한으로 한 번 더 막는다.
    std::iter::successors(Some(id), |id| parents.get(id).copied().flatten())
        .take(crate::persistence::MAX_DEPTH + 2)
}

/// 최상위에서 scope까지의 경로. scope가 사라졌으면 남은 가장 가까운 조상까지만 돌려준다.
pub(super) fn path(state: &AppState, scope: Option<Uuid>) -> Vec<Place> {
    let Some(scope) = scope else {
        return Vec::new();
    };
    let parents = parents(state);
    let mut path: Vec<_> = ancestors(&parents, scope)
        .map_while(|id| place(state, id))
        .collect();
    path.reverse();
    path
}

pub(super) fn note_ids(state: &AppState) -> std::collections::HashSet<Uuid> {
    state.notebook.notes().iter().map(|n| n.id()).collect()
}

pub(super) fn exists(state: &AppState, id: Uuid) -> bool {
    state.notebook.note(id).is_some()
}

/// 고정한 프로젝트가 먼저, 그다음 자주·최근 들어간 프로젝트. 현재 경로에 있는 곳은 뺀다.
pub(super) fn shortcuts(state: &AppState, scope: Option<Uuid>, limit: usize) -> Vec<(Place, bool)> {
    let parents = parents(state);
    let projects = projects(&parents);
    let here: Vec<_> = scope.map_or_else(Vec::new, |s| ancestors(&parents, s).collect());
    let mut out: Vec<(Place, bool)> = state
        .notebook
        .notes()
        .iter()
        .filter(|n| n.canvas().pinned && !here.contains(&n.id()))
        .take(limit)
        .filter_map(|n| place(state, n.id()))
        .map(|p| (p, true))
        .collect();
    let now = crate::app::shared::unix_now();
    for id in state
        .navigation
        .ranked(now, |id| projects.contains(&id) && !here.contains(&id))
    {
        if out.len() >= limit {
            break;
        }
        if out.iter().all(|(p, _)| p.id != id) {
            out.extend(place(state, id).map(|p| (p, false)));
        }
    }
    out
}

/// 선택 메모를 옮길 수 있는 프로젝트 후보. 자기 자신·하위·깊이 초과는 빠진다.
pub(super) fn move_targets(state: &AppState, ids: &[Uuid]) -> Vec<Place> {
    let parents = parents(state);
    let projects = projects(&parents);
    state
        .notebook
        .notes()
        .iter()
        .filter(|n| projects.contains(&n.id()))
        .filter(|n| ancestors(&parents, n.id()).count() <= crate::persistence::MAX_DEPTH)
        .filter(|n| {
            ids.iter().all(|id| {
                !ancestors(&parents, n.id()).any(|a| a == *id)
                    && parents.get(id).is_some_and(|p| *p != Some(n.id()))
            })
        })
        .filter_map(|n| place(state, n.id()))
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

pub(super) fn set_parent(notes: &mut Notes<'_>, id: Uuid, parent: Option<Uuid>) -> bool {
    notes.set_parent(id, parent)
}

pub(super) fn set_pinned(notes: &mut Notes<'_>, id: Uuid, pinned: bool) {
    notes.set_pinned(id, pinned);
}

pub(super) fn parent_of(state: &AppState, id: Uuid) -> Option<Uuid> {
    state.notebook.note(id).and_then(|n| n.canvas().parent)
}

pub(super) fn pinned(state: &AppState, id: Uuid) -> bool {
    state.notebook.note(id).is_some_and(|n| n.canvas().pinned)
}

fn slot(index: usize) -> [f64; 2] {
    super::model::placement::slot(index)
}

/// 같은 단계 카드만 피한다. 다른 프로젝트 안의 카드는 이 화면에 겹쳐 보이지 않는다.
pub(super) fn vacant(notebook: &Notebook, scope: Option<Uuid>) -> [f64; 2] {
    super::model::placement::vacant(
        notebook
            .notes()
            .iter()
            .enumerate()
            .filter(|(_, n)| n.canvas().parent == scope)
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
