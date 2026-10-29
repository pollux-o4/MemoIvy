use super::menu_for;
use crate::app::{state::State, windows::open_note};
use gtk::{prelude::*, Application, Box as GtkBox, Button, Label, Orientation};

pub(in crate::app) fn refresh(state: &State, app: &Application) {
    let s = state.borrow();
    while let Some(child) = s.list.first_child() {
        s.list.remove(&child);
    }
    let query = s.search.text().to_lowercase();
    let mut notes = s.store.notes.clone();
    notes.sort_by_key(|n| std::cmp::Reverse(n.modified));
    notes.retain(|n| n.text.to_lowercase().contains(&query));
    s.count.set_text(&format!(
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
        let image = gtk::Image::from_icon_name("document-new-symbolic");
        image.set_pixel_size(36);
        empty.append(&image);
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
        s.list.append(&empty);
    }
    for note in notes {
        let card = GtkBox::new(Orientation::Vertical, 0);
        card.add_css_class("card");
        card.add_css_class(&note.color);
        let strip = GtkBox::new(Orientation::Horizontal, 0);
        strip.add_css_class("strip");
        card.append(&strip);
        let row = GtkBox::new(Orientation::Horizontal, 0);
        let contents = GtkBox::new(Orientation::Vertical, 6);
        let mut lines = note.text.lines().skip_while(|l| l.trim().is_empty());
        let title = lines
            .next()
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .unwrap_or("빈 메모");
        let preview = lines
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let t = Label::new(Some(title));
        t.set_xalign(0.);
        t.set_ellipsize(gtk::pango::EllipsizeMode::End);
        t.add_css_class("card-title");
        contents.append(&t);
        if !preview.is_empty() || note.text.trim().is_empty() {
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
        let id = note.id;
        open.connect_clicked(move |_| open_note(&st, &a, id));
        row.append(&open);
        let menu = menu_for(state, app, note.id);
        menu.set_valign(gtk::Align::Start);
        row.append(&menu);
        card.append(&row);
        s.list.append(&card);
    }
}
