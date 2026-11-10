//! 프로젝트를 실제 부모·자식 위젯으로 묶고, 해당 섹션만 지연 생성·전환한다.
use super::{list_key, note_title, tree, Note, State};
use gtk::{prelude::*, Application, Box as GtkBox, Orientation};
use std::{collections::HashMap, rc::Rc};
use uuid::Uuid;

struct Entry {
    id: Uuid,
    children: usize,
    tasks: (usize, usize),
}
type Outline = Rc<HashMap<Option<Uuid>, Vec<Entry>>>;

#[cfg(test)]
pub(in crate::app::library) mod tests;

pub(super) fn append(list: &GtkBox, state: &State, app: &Application) {
    let s = state.borrow();
    let notes = s.notebook.notes();
    let expanded = notes
        .iter()
        .filter(|n| n.canvas().project)
        .map(Note::id)
        .collect();
    let mut outline: HashMap<_, Vec<_>> = HashMap::new();
    for row in tree(notes, &expanded) {
        outline
            .entry(row.note.canvas().parent)
            .or_default()
            .push(Entry {
                id: row.note.id(),
                children: row.children,
                tasks: row.tasks,
            });
    }
    let index = notes.iter().map(|n| (n.id(), n)).collect();
    branch(list, state, app, &Rc::new(outline), &index, None);
}

fn branch(
    list: &GtkBox,
    state: &State,
    app: &Application,
    outline: &Outline,
    notes: &HashMap<Uuid, &Note>,
    parent: Option<Uuid>,
) {
    for entry in outline.get(&parent).into_iter().flatten() {
        let Some(note) = notes.get(&entry.id) else {
            continue;
        };
        let card = super::card::card(
            state,
            app,
            note,
            super::card::CardKind::Grouped {
                nested: parent.is_some(),
                children: entry.children,
                tasks: entry.tasks,
            },
        );
        let inherited_color = parent
            .and_then(|id| notes.get(&id))
            .is_some_and(|owner| owner.color() == note.color());
        if !note.canvas().project {
            if inherited_color {
                card.add_css_class("inherited-color");
            }
            list.append(&card);
            continue;
        }
        let section = GtkBox::new(Orientation::Vertical, 0);
        section.add_css_class("project-section");
        section.add_css_class(note.color());
        if inherited_color {
            section.add_css_class("inherited-color");
        }
        section.set_widget_name(&format!("project-section-{}", entry.id));
        section.append(&card);
        list.append(&section);
        if entry.children == 0 {
            continue;
        }
        let revealer = gtk::Revealer::new();
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(240);
        section.append(&revealer);
        let expanded = state
            .borrow()
            .views
            .library
            .preferences
            .borrow()
            .expanded()
            .contains(&entry.id);
        if expanded {
            populate(&revealer, state, app, outline, notes, entry.id);
            section.add_css_class("expanded");
        }
        revealer.set_reveal_child(expanded);
        let header = &super::disclosure::button("프로젝트 내용 더 보기·접기");
        header.set_widget_name(&entry.id.to_string());
        section.append(header);
        header.update_relation(&[gtk::accessible::Relation::Controls(
            &[revealer.upcast_ref()],
        )]);
        let title = crate::presentation::markdown::summary(note_title(note.text())).title;
        let title = super::display_title(&title, true).to_owned();
        super::disclosure::reflect(header, expanded, &title);
        let (weak_state, weak_app) = (Rc::downgrade(state), app.downgrade());
        let (weak_revealer, weak_section) = (revealer.downgrade(), section.downgrade());
        let outline = outline.clone();
        let id = entry.id;
        header.connect_clicked(move |header| {
            let (Some(state), Some(app), Some(revealer), Some(section)) =
                (weak_state.upgrade(), weak_app.upgrade(), weak_revealer.upgrade(), weak_section.upgrade()) else { return; };
            let s = state.borrow();
            let result = s.views.library.preferences.borrow_mut().toggle(id, |other| {
                s.notebook.note(*other).is_some_and(|note| note.canvas().project)
            });
            let expanded = s.views.library.preferences.borrow().expanded().contains(&id);
            let query = s.views.library.search.text();
            s.views.library.shown.set(Some(list_key(s.notebook.notes(), &query,
                s.views.library.grouped.is_active() && query.is_empty(),
                s.views.library.preferences.borrow().expanded())));
            if let Err(error) = result {
                crate::app::shared::message(&s.views.library.window, "목록 상태 저장 오류",
                    &format!("이번 펼침 상태를 저장하지 못했어요. 다시 실행하면 이전 상태로 돌아갑니다.\n{error}"));
            }
            drop(s);
            if expanded {
                let s = state.borrow();
                let notes = s.notebook.notes().iter().map(|n| (n.id(), n)).collect();
                populate(&revealer, &state, &app, &outline, &notes, id);
                section.add_css_class("expanded");
            } else {
                section.remove_css_class("expanded");
            }
            super::disclosure::reflect(header, expanded, &title);
            revealer.set_reveal_child(expanded);
        });
    }
}

fn populate(
    revealer: &gtk::Revealer,
    state: &State,
    app: &Application,
    outline: &Outline,
    notes: &HashMap<Uuid, &Note>,
    parent: Uuid,
) {
    if revealer.child().is_some() {
        return;
    }
    let children = GtkBox::new(Orientation::Vertical, 0);
    children.add_css_class("project-children");
    branch(&children, state, app, outline, notes, Some(parent));
    revealer.set_child(Some(&children));
}
