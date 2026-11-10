use super::{display_title, preview};
use crate::app::shared::lazy_menu_for;
use crate::app::shell::{
    commands::{open_canvas_at, open_note},
    state::State,
};
use crate::persistence::Note;
use crate::presentation::icons;
use gtk::{prelude::*, Application, Box as GtkBox, Button, Label, Orientation};

pub(super) enum CardKind {
    Grouped {
        nested: bool,
        children: usize,
        tasks: (usize, usize),
    },
    Flat {
        location: String,
        children: usize,
    },
}

pub(super) fn card(state: &State, app: &Application, note: &Note, kind: CardKind) -> GtkBox {
    let id = note.id();
    let card = GtkBox::new(Orientation::Vertical, 0);
    card.add_css_class("card");
    card.add_css_class(note.color());
    let row = GtkBox::new(Orientation::Horizontal, 0);
    card.append(&row);
    let stripe = GtkBox::new(Orientation::Vertical, 0);
    stripe.add_css_class("card-stripe");
    row.append(&stripe);
    let summary = crate::presentation::markdown::summary(note.text());
    let (nested, children, tasks, place) = match kind {
        CardKind::Grouped {
            nested,
            children,
            tasks,
        } => (nested, children, tasks, String::new()),
        CardKind::Flat { location, children } => (
            false,
            children,
            crate::presentation::markdown::task_progress(note.text()),
            location,
        ),
    };
    if nested {
        card.add_css_class("nested");
    }
    let project = note.canvas().project;
    if project {
        card.add_css_class("project");
    }
    let contents = GtkBox::new(Orientation::Vertical, 4);
    let title = Label::new(Some(display_title(&summary.title, project)));
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
    let mut reveal = None;
    if !project && !summary.preview.is_empty() {
        let (body, toggle) = preview::build(note.text(), &summary.title);
        contents.append(&body);
        reveal = Some(toggle);
    } else if !preview.is_empty() {
        let p = Label::new(Some(&preview));
        p.set_xalign(0.);
        p.set_wrap(true);
        p.set_lines(if nested || project { 1 } else { 2 });
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
    row.append(&open);
    let actions = GtkBox::new(Orientation::Horizontal, 0);
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
    row.append(&actions);
    if let Some(toggle) = reveal {
        card.append(&toggle);
    }
    card
}
