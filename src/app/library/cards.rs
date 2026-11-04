use crate::app::shared::lazy_menu_for;
use crate::app::shell::{
    commands::{open_canvas_at, open_note},
    state::State,
};
use crate::persistence::Note;
use crate::presentation::icons;
use gtk::{prelude::*, Application, Box as GtkBox, Button, Label, Orientation};
use std::{
    cell::{Cell, RefCell},
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};
use uuid::Uuid;

thread_local! {
    static SHOWN: Cell<u64> = const { Cell::new(0) };
    /// 접어 둔 프로젝트. 화면 상태라 저장하지 않는다.
    static COLLAPSED: RefCell<HashSet<Uuid>> = RefCell::new(HashSet::new());
}

fn list_key(notes: &[Note], query: &str, grouped: bool) -> u64 {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    grouped.hash(&mut hasher);
    COLLAPSED.with(|c| {
        let mut ids: Vec<_> = c.borrow().iter().copied().collect();
        ids.sort();
        ids.hash(&mut hasher);
    });
    for note in notes {
        note.id().hash(&mut hasher);
        note.text().hash(&mut hasher);
        note.color().hash(&mut hasher);
        note.modified().hash(&mut hasher);
        note.canvas().parent.hash(&mut hasher);
    }
    hasher.finish()
}

/// 프로젝트별 보기의 한 줄. 깊이만큼 들여 쓰고, 자식이 있으면 접을 수 있는 머리글이 된다.
struct Row<'a> {
    note: &'a Note,
    depth: usize,
    children: usize,
    /// 자기와 하위 전체의 할 일 진행.
    tasks: (usize, usize),
}

/// 형제끼리는 프로젝트를 먼저, 그 안에서는 하위 전체의 최근 수정 순으로 늘어놓는다.
/// 할 일 파싱은 메모마다 한 번, 하위 합계는 아래에서 위로 한 번만 더한다.
fn tree(notes: &[Note]) -> Vec<Row<'_>> {
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
        depth: usize,
        children: &HashMap<Option<Uuid>, Vec<&'a Note>>,
        totals: &HashMap<Uuid, (usize, usize, u64)>,
        out: &mut Vec<Row<'a>>,
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
        siblings.sort_by_key(|(_, count, t)| (*count == 0, std::cmp::Reverse(t.2)));
        for (note, count, (done, all, _)) in siblings {
            out.push(Row {
                note,
                depth,
                children: count,
                tasks: (done, all),
            });
            let open = !COLLAPSED.with(|c| c.borrow().contains(&note.id()));
            if count > 0 && open {
                walk(Some(note.id()), depth + 1, children, totals, out);
            }
        }
    }
    let mut out = Vec::with_capacity(notes.len());
    walk(None, 0, &children, &totals, &mut out);
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
        names.push(if title.is_empty() {
            "이름 없는 프로젝트".to_owned()
        } else {
            title
        });
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
    let key = list_key(s.notebook.notes(), &query, grouped);
    if SHOWN.with(|shown| shown.replace(key)) == key {
        drop(s);
        return;
    }
    // 지워진 프로젝트의 접힘 상태는 버린다.
    COLLAPSED.with(|c| {
        let ids: HashSet<_> = s.notebook.notes().iter().map(|n| n.id()).collect();
        c.borrow_mut().retain(|id| ids.contains(id));
    });
    let library = &s.views.library;
    while let Some(child) = library.list.first_child() {
        library.list.remove(&child);
    }
    let all = s.notebook.notes();
    let notes = search_notes(all, &query);
    let projects = all
        .iter()
        .filter(|n| all.iter().any(|c| c.canvas().parent == Some(n.id())))
        .count();
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
        for row in tree(all) {
            library.list.append(&card(
                state,
                app,
                row.note,
                CardKind::Tree {
                    depth: row.depth,
                    children: row.children,
                    tasks: row.tasks,
                },
            ));
        }
    } else {
        let by_id: HashMap<_, _> = all.iter().map(|n| (n.id(), n)).collect();
        for note in notes {
            let place = location(&by_id, note);
            library
                .list
                .append(&card(state, app, note, CardKind::Flat { location: place }));
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
        "새 메모를 눌러 생각을 적어 보세요."
    } else {
        "다른 검색어를 입력해 보세요."
    }));
    sub.add_css_class("empty-sub");
    empty.append(&sub);
    empty
}

enum CardKind {
    Tree {
        depth: usize,
        children: usize,
        tasks: (usize, usize),
    },
    Flat {
        location: String,
    },
}

fn card(state: &State, app: &Application, note: &Note, kind: CardKind) -> GtkBox {
    let id = note.id();
    let card = GtkBox::new(Orientation::Horizontal, 0);
    card.add_css_class("card");
    card.add_css_class(note.color());
    let stripe = GtkBox::new(Orientation::Vertical, 0);
    stripe.add_css_class("card-stripe");
    card.append(&stripe);
    let summary = crate::presentation::markdown::summary(note.text());
    let (depth, children, tasks, place) = match kind {
        CardKind::Tree {
            depth,
            children,
            tasks,
        } => (depth, children, tasks, String::new()),
        CardKind::Flat { location } => (
            0,
            0,
            crate::presentation::markdown::task_progress(note.text()),
            location,
        ),
    };
    if depth > 0 {
        card.add_css_class("nested");
        card.set_margin_start(18 * depth as i32);
    }
    let project = children > 0;
    if project {
        card.add_css_class("project");
        let collapsed = COLLAPSED.with(|c| c.borrow().contains(&id));
        let toggle = Button::new();
        toggle.set_child(Some(&icons::image(if collapsed {
            icons::CHEVRON
        } else {
            icons::CHEVRON_DOWN
        })));
        toggle.add_css_class("card-toggle");
        toggle.set_valign(gtk::Align::Center);
        let label = if collapsed { "펼치기" } else { "접기" };
        toggle.set_tooltip_text(Some(label));
        toggle.update_property(&[gtk::accessible::Property::Label(label)]);
        let st = state.clone();
        let a = app.clone();
        toggle.connect_clicked(move |_| {
            COLLAPSED.with(|c| {
                let mut c = c.borrow_mut();
                if !c.remove(&id) {
                    c.insert(id);
                }
            });
            refresh(&st, &a);
        });
        card.append(&toggle);
    }
    let contents = GtkBox::new(Orientation::Vertical, 4);
    let title = Label::new(Some(match summary.title.as_str() {
        "" => "빈 메모",
        title => title,
    }));
    title.set_xalign(0.);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.add_css_class("card-title");
    if !place.is_empty() {
        let location = Label::new(Some(&place));
        location.set_xalign(0.);
        location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        location.add_css_class("card-location");
        contents.append(&location);
    }
    let head = GtkBox::new(Orientation::Horizontal, 8);
    title.set_hexpand(true);
    head.append(&title);
    if tasks.1 > 0 {
        let chip = Label::new(Some(&format!("{}/{}", tasks.0, tasks.1)));
        chip.add_css_class("task-chip");
        if tasks.0 == tasks.1 {
            chip.add_css_class("complete");
        }
        chip.set_tooltip_text(Some(&format!("할 일 {}개 중 {}개 완료", tasks.1, tasks.0)));
        head.append(&chip);
    }
    contents.append(&head);
    let preview = if project {
        format!("메모 {children}개")
    } else if summary.preview.is_empty() && note.text().trim().is_empty() {
        "열어서 내용을 적어 보세요".to_owned()
    } else {
        summary.preview.clone()
    };
    if !preview.is_empty() {
        let p = Label::new(Some(&preview));
        p.set_xalign(0.);
        p.set_wrap(true);
        p.set_lines(if depth > 0 || project { 1 } else { 2 });
        p.set_ellipsize(gtk::pango::EllipsizeMode::End);
        p.add_css_class("preview");
        contents.append(&p);
    }
    if project && tasks.1 > 0 {
        let bar = gtk::ProgressBar::new();
        bar.add_css_class("card-progress");
        bar.set_fraction(tasks.0 as f64 / tasks.1 as f64);
        contents.append(&bar);
    }
    let open = Button::new();
    open.add_css_class("card-open");
    open.set_hexpand(true);
    open.set_child(Some(&contents));
    let st = state.clone();
    let a = app.clone();
    open.connect_clicked(move |_| open_note(&st, &a, id));
    card.append(&open);
    let actions = GtkBox::new(Orientation::Vertical, 0);
    actions.set_valign(gtk::Align::Start);
    if project {
        let canvas = Button::new();
        canvas.set_child(Some(&icons::image(icons::ENTER)));
        canvas.add_css_class("icon-button");
        canvas.set_tooltip_text(Some("캔버스에서 프로젝트 열기"));
        canvas.update_property(&[gtk::accessible::Property::Label("캔버스에서 프로젝트 열기")]);
        let st = state.clone();
        let a = app.clone();
        canvas.connect_clicked(move |_| open_canvas_at(&st, &a, id));
        actions.append(&canvas);
    }
    actions.append(&lazy_menu_for(state, app, id));
    card.append(&actions);
    card
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

    fn note(text: &str, modified: u64, parent: Option<Uuid>) -> Note {
        let mut canvas = serde_json::json!({});
        if let Some(parent) = parent {
            canvas = serde_json::json!({ "parent": parent });
        }
        serde_json::from_value(serde_json::json!({"id": Uuid::new_v4(), "text": text, "color": "yellow", "modified": modified, "canvas": canvas})).unwrap()
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
        .map(|(text, modified)| note(text, modified, None))
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
        let english = note("\n  Sticky Memo\nbody", 0, None);
        assert_eq!(search_notes(&[english], "sTICKY").len(), 1);
    }

    #[test]
    fn project_002_tree_lists_projects_first_with_subtree_progress() {
        let project = note("취업하기", 1, None);
        let pid = project.id();
        let child = note("포트폴리오\n- [x] a\n- [ ] b", 50, Some(pid));
        let loose = note("낱장 메모", 40, None);
        let notes = vec![loose, child, project];
        let rows = tree(&notes);
        let shape: Vec<_> = rows
            .iter()
            .map(|r| (note_title(r.note.text()), r.depth, r.children, r.tasks))
            .collect();
        assert_eq!(
            shape,
            [
                ("취업하기", 0, 1, (1, 2)),
                ("포트폴리오", 1, 0, (1, 2)),
                ("낱장 메모", 0, 0, (0, 0)),
            ]
        );
        let by_id: HashMap<_, _> = notes.iter().map(|n| (n.id(), n)).collect();
        assert_eq!(location(&by_id, &notes[1]), "취업하기");
        COLLAPSED.with(|c| c.borrow_mut().insert(pid));
        assert_eq!(tree(&notes).len(), 2);
        COLLAPSED.with(|c| c.borrow_mut().clear());
    }
}
