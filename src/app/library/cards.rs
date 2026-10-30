use crate::app::shared::lazy_menu_for;
use crate::app::shell::{commands::open_note, state::State};
use crate::persistence::Note;
use crate::presentation::icons;
use gtk::{prelude::*, Application, Box as GtkBox, Button, Label, Orientation};
use std::{
    cell::Cell,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

thread_local! { static SHOWN: Cell<u64> = const { Cell::new(0) }; }

fn list_key(notes: &[Note], query: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    for note in notes {
        note.id().hash(&mut hasher);
        note.text().hash(&mut hasher);
        note.color().hash(&mut hasher);
        note.modified().hash(&mut hasher);
    }
    hasher.finish()
}

pub(in crate::app) fn refresh(state: &State, app: &Application) {
    let s = state.borrow();
    let query = s.views.library.search.text().to_string();
    // 캔버스 배치만 바뀌면 목록은 그대로다. 다시 만들면 스크롤·호버가 끊긴다.
    let key = list_key(s.notebook.notes(), &query);
    if SHOWN.with(|shown| shown.replace(key)) == key {
        drop(s);
        return;
    }
    while let Some(child) = s.views.library.list.first_child() {
        s.views.library.list.remove(&child);
    }
    let notes = search_notes(s.notebook.notes(), &query);
    s.views
        .library
        .count
        .set_visible(!(notes.is_empty() && query.is_empty()));
    s.views.library.count.set_text(&format!(
        "{} {}개",
        if query.is_empty() {
            "메모"
        } else {
            "검색 결과"
        },
        notes.len()
    ));
    if notes.is_empty() {
        let empty = GtkBox::new(Orientation::Vertical, 8);
        empty.set_margin_top(80);
        if query.is_empty() {
            empty.append(&icons::illustration(icons::LIBRARY_EMPTY, 96, 80));
        }
        let title = Label::new(Some(if query.is_empty() {
            "첫 메모를 남겨 보세요"
        } else {
            "일치하는 메모가 없어요"
        }));
        title.add_css_class("card-title");
        empty.append(&title);
        let sub = Label::new(Some(if query.is_empty() {
            "새 메모를 눌러 생각을 적어 보세요."
        } else {
            "다른 검색어를 입력해 보세요."
        }));
        sub.add_css_class("empty-sub");
        empty.append(&sub);
        s.views.library.list.append(&empty);
    }
    for note in notes {
        let card = GtkBox::new(Orientation::Vertical, 0);
        card.add_css_class("card");
        card.add_css_class(note.color());
        let row = GtkBox::new(Orientation::Horizontal, 0);
        let contents = GtkBox::new(Orientation::Vertical, 6);
        let summary = crate::presentation::markdown::summary(note.text());
        let title = match summary.title.as_str() {
            "" => "빈 메모",
            title => title,
        };
        let preview = summary.preview;
        let t = Label::new(Some(title));
        t.set_xalign(0.);
        t.set_ellipsize(gtk::pango::EllipsizeMode::End);
        t.add_css_class("card-title");
        contents.append(&t);
        if !preview.is_empty() || note.text().trim().is_empty() {
            let p = Label::new(Some(if preview.is_empty() {
                "열어서 내용을 적어 보세요"
            } else {
                &preview
            }));
            p.set_xalign(0.);
            p.set_wrap(true);
            p.set_lines(2);
            p.set_ellipsize(gtk::pango::EllipsizeMode::End);
            p.add_css_class("preview");
            contents.append(&p);
        }
        let open = Button::new();
        open.add_css_class("card-open");
        open.set_hexpand(true);
        open.set_child(Some(&contents));
        let st = state.clone();
        let a = app.clone();
        let id = note.id();
        open.connect_clicked(move |_| open_note(&st, &a, id));
        row.append(&open);
        let menu = lazy_menu_for(state, app, note.id());
        menu.set_valign(gtk::Align::Start);
        row.append(&menu);
        card.append(&row);
        s.views.library.list.append(&card);
    }
    drop(s);
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

    #[test]
    fn note_003_title_matches_precede_newer_body_matches() {
        let notes: Vec<_> = [
            ("다른 제목\n스티커 메모", 30),
            ("\n  스티커 메모  \n본문", 10),
            ("스티커 모음\n본문", 20),
            ("일치하지 않음", 40),
        ]
        .into_iter()
        .map(|(text, modified)| {
            let mut note = Note::new();
            note = serde_json::from_value(serde_json::json!({"id": note.id(), "text":text,"color":"yellow","modified":modified})).unwrap();
            note
        })
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
        let mut english = Note::new();
        english = serde_json::from_value(serde_json::json!({"id":english.id(),"text":"\n  Sticky Memo\nbody","color":"yellow","modified":0})).unwrap();
        assert_eq!(search_notes(&[english], "sTICKY").len(), 1);
    }
}
