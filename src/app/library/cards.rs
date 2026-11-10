use crate::app::shell::state::State;
use crate::persistence::Note;
use crate::presentation::icons;
use gtk::{prelude::*, Application, Box as GtkBox, Label, Orientation};
use std::{
    collections::{hash_map::DefaultHasher, BTreeSet, HashMap},
    hash::{Hash, Hasher},
};
use uuid::Uuid;

mod card;
mod disclosure;
mod grouping;
mod preview;

#[cfg(test)]
pub(in crate::app) use grouping::tests::project_002_sections_toggle_in_place_and_keep_preview;
#[cfg(test)]
pub(in crate::app) use preview::tests::note_004_preview_disclosure;

fn list_key(notes: &[Note], query: &str, grouped: bool, expanded: &BTreeSet<Uuid>) -> u64 {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    grouped.hash(&mut hasher);
    expanded.hash(&mut hasher);
    for note in notes {
        note.id().hash(&mut hasher);
        note.text().hash(&mut hasher);
        note.color().hash(&mut hasher);
        note.modified().hash(&mut hasher);
        note.canvas().parent.hash(&mut hasher);
        note.canvas().project.hash(&mut hasher);
    }
    hasher.finish()
}

/// 프로젝트별 보기의 관계·집계 행. 화면 배치와 펼침 조작은 그룹 뷰가 담당한다.
struct Row<'a> {
    note: &'a Note,
    children: usize,
    /// 자기와 하위 전체의 할 일 진행.
    tasks: (usize, usize),
}

/// 형제끼리는 프로젝트를 먼저, 그 안에서는 하위 전체의 최근 수정 순으로 늘어놓는다.
/// 할 일 파싱은 메모마다 한 번, 하위 합계는 아래에서 위로 한 번만 더한다.
fn tree<'a>(notes: &'a [Note], expanded: &BTreeSet<Uuid>) -> Vec<Row<'a>> {
    let mut children: HashMap<Option<Uuid>, Vec<&Note>> = HashMap::new();
    for note in notes {
        children.entry(note.canvas().parent).or_default().push(note);
    }
    fn total(
        note: &Note,
        children: &HashMap<Option<Uuid>, Vec<&Note>>,
        totals: &mut HashMap<Uuid, (usize, usize, u64)>,
    ) -> (usize, usize, u64) {
        let own = crate::presentation::markdown::task_progress(note.text());
        let mut sum = (own.0, own.1, note.modified());
        for child in children.get(&Some(note.id())).into_iter().flatten() {
            let (d, n, m) = total(child, children, totals);
            sum = (sum.0 + d, sum.1 + n, sum.2.max(m));
        }
        totals.insert(note.id(), sum);
        sum
    }
    let mut totals = HashMap::with_capacity(notes.len());
    for root in children.get(&None).into_iter().flatten() {
        total(root, &children, &mut totals);
    }
    fn walk<'a>(
        parent: Option<Uuid>,
        children: &HashMap<Option<Uuid>, Vec<&'a Note>>,
        totals: &HashMap<Uuid, (usize, usize, u64)>,
        out: &mut Vec<Row<'a>>,
        expanded: &BTreeSet<Uuid>,
    ) {
        let mut siblings: Vec<_> = children
            .get(&parent)
            .into_iter()
            .flatten()
            .map(|note| {
                let count = children.get(&Some(note.id())).map_or(0, Vec::len);
                (*note, count, totals[&note.id()])
            })
            .collect();
        siblings.sort_by_key(|(note, _, t)| (!note.canvas().project, std::cmp::Reverse(t.2)));
        for (note, count, (done, all, _)) in siblings {
            out.push(Row {
                note,
                children: count,
                tasks: (done, all),
            });
            let open = expanded.contains(&note.id());
            if count > 0 && open {
                walk(Some(note.id()), children, totals, out, expanded);
            }
        }
    }
    let mut out = Vec::with_capacity(notes.len());
    walk(None, &children, &totals, &mut out, expanded);
    out
}

/// 검색 결과에 붙이는 위치. 어느 프로젝트의 메모인지 알아야 같은 제목을 구분한다.
fn location(notes: &HashMap<Uuid, &Note>, note: &Note) -> String {
    let mut names = Vec::new();
    let mut current = note.canvas().parent;
    while let Some(id) = current {
        let Some(parent) = notes.get(&id) else {
            break;
        };
        let title = crate::presentation::markdown::summary(note_title(parent.text())).title;
        names.push(display_title(&title, true).to_owned());
        current = parent.canvas().parent;
    }
    names.reverse();
    names.join(" › ")
}

pub(in crate::app) fn refresh(state: &State, app: &Application) {
    let s = state.borrow();
    let query = s.views.library.search.text().to_string();
    let grouped = s.views.library.grouped.is_active() && query.is_empty();
    // 캔버스 배치만 바뀌면 목록은 그대로다. 다시 만들면 스크롤·호버가 끊긴다.
    let key = list_key(
        s.notebook.notes(),
        &query,
        grouped,
        s.views.library.preferences.borrow().expanded(),
    );
    if s.views.library.shown.replace(Some(key)) == Some(key) {
        drop(s);
        return;
    }
    let library = &s.views.library;
    while let Some(child) = library.list.first_child() {
        library.list.remove(&child);
    }
    let all = s.notebook.notes();
    let notes = search_notes(all, &query);
    let projects = all.iter().filter(|n| n.canvas().project).count();
    library
        .count
        .set_visible(!(notes.is_empty() && query.is_empty()));
    library.count.set_text(&if !query.is_empty() {
        format!("검색 결과 {}개", notes.len())
    } else if projects > 0 {
        format!("메모 {}개 · 프로젝트 {projects}개", notes.len())
    } else {
        format!("메모 {}개", notes.len())
    });
    // 프로젝트가 없으면 보기 전환도 필요 없다.
    library
        .grouped
        .parent()
        .inspect(|w| w.set_visible(projects > 0 && query.is_empty()));
    if notes.is_empty() {
        library.list.append(&empty_state(query.is_empty()));
    }
    if grouped {
        grouping::append(&library.list, state, app);
    } else {
        let by_id: HashMap<_, _> = all.iter().map(|n| (n.id(), n)).collect();
        let mut children: HashMap<Uuid, usize> = HashMap::new();
        for note in all {
            if let Some(parent) = note.canvas().parent {
                *children.entry(parent).or_default() += 1;
            }
        }
        for note in notes {
            let place = location(&by_id, note);
            library.list.append(&card::card(
                state,
                app,
                note,
                card::CardKind::Flat {
                    location: place,
                    children: children.get(&note.id()).copied().unwrap_or(0),
                },
            ));
        }
    }
    drop(s);
}

fn empty_state(first: bool) -> GtkBox {
    let empty = GtkBox::new(Orientation::Vertical, 8);
    empty.set_margin_top(80);
    if first {
        empty.append(&icons::illustration(icons::LIBRARY_EMPTY, 96, 80));
    }
    let title = Label::new(Some(if first {
        "첫 메모를 남겨 보세요"
    } else {
        "일치하는 메모가 없어요"
    }));
    title.add_css_class("card-title");
    empty.append(&title);
    let sub = Label::new(Some(if first {
        "+ New에서 메모를 만들어 생각을 적어 보세요."
    } else {
        "다른 검색어를 입력해 보세요."
    }));
    sub.add_css_class("empty-sub");
    empty.append(&sub);
    empty
}

fn display_title(title: &str, project: bool) -> &str {
    match (title.is_empty(), project) {
        (true, true) => "이름 없는 프로젝트",
        (true, false) => "빈 메모",
        (false, _) => title,
    }
}

fn note_title(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
}

fn search_notes<'a>(notes: &'a [Note], query: &str) -> Vec<&'a Note> {
    let query = query.to_lowercase();
    let mut matches: Vec<_> = notes
        .iter()
        .filter(|note| note.text().to_lowercase().contains(&query))
        .collect();
    matches.sort_by_cached_key(|note| {
        (
            !query.is_empty() && !note_title(note.text()).to_lowercase().contains(&query),
            std::cmp::Reverse(note.modified()),
        )
    });
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(text: &str, modified: u64, parent: Option<Uuid>, project: bool) -> Note {
        let mut canvas = serde_json::json!({"project": project});
        if let Some(parent) = parent {
            canvas["parent"] = serde_json::json!(parent);
        }
        serde_json::from_value(serde_json::json!({"id": Uuid::new_v4(), "text": text, "color": "yellow", "modified": modified, "canvas": canvas})).unwrap()
    }

    #[test]
    fn project_002_blank_card_titles_have_contextual_fallbacks() {
        assert_eq!(display_title("", true), "이름 없는 프로젝트");
        assert_eq!(display_title("", false), "빈 메모");
        assert_eq!(display_title("작업", true), "작업");
    }

    #[test]
    fn note_003_title_matches_precede_newer_body_matches() {
        let notes: Vec<_> = [
            ("다른 제목\n스티커 메모", 30),
            ("\n  스티커 메모  \n본문", 10),
            ("스티커 모음\n본문", 20),
            ("일치하지 않음", 40),
        ]
        .into_iter()
        .map(|(text, modified)| note(text, modified, None, false))
        .collect();
        let revisions = |query| {
            search_notes(&notes, query)
                .iter()
                .map(|note| note.modified())
                .collect::<Vec<_>>()
        };
        assert_eq!(revisions("스티커"), [20, 10, 30]);
        assert_eq!(revisions(""), [40, 30, 20, 10]);
        assert!(revisions("없음").is_empty());
        let english = note("\n  Sticky Memo\nbody", 0, None, false);
        assert_eq!(search_notes(&[english], "sTICKY").len(), 1);
    }

    #[test]
    fn project_002_tree_lists_projects_first_with_subtree_progress() {
        let project = note("취업하기", 1, None, true);
        let pid = project.id();
        let child = note("포트폴리오\n- [x] a\n- [ ] b", 50, Some(pid), false);
        let loose = note("낱장 메모", 40, None, false);
        let notes = vec![loose, child, project];
        let rows = tree(&notes, &BTreeSet::from([pid]));
        let shape: Vec<_> = rows
            .iter()
            .map(|r| {
                (
                    note_title(r.note.text()),
                    r.note.canvas().parent,
                    r.children,
                    r.tasks,
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                ("취업하기", None, 1, (1, 2)),
                ("포트폴리오", Some(pid), 0, (1, 2)),
                ("낱장 메모", None, 0, (0, 0)),
            ]
        );
        let by_id: HashMap<_, _> = notes.iter().map(|n| (n.id(), n)).collect();
        assert_eq!(location(&by_id, &notes[1]), "취업하기");
        assert_eq!(tree(&notes, &BTreeSet::new()).len(), 2);
    }

    #[test]
    fn project_003_empty_projects_precede_newer_notes() {
        let notes = vec![
            note("최근 메모", 99, None, false),
            note("빈 프로젝트", 1, None, true),
        ];
        let rows = tree(&notes, &BTreeSet::new());
        assert_eq!(note_title(rows[0].note.text()), "빈 프로젝트");
        assert_eq!(rows[0].children, 0);
        assert_eq!(rows[0].tasks, (0, 0));
        assert_eq!(rows.len(), 2);
    }
}
