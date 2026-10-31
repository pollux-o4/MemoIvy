use super::{card_root::CardRoot, preview};
use crate::app::canvas::{input, port, port::NodeView, CanvasView};
use crate::presentation::{icons, markdown};
use gtk::{prelude::*, Box as GtkBox, Button, Label, Orientation};
use std::{cell::RefCell, rc::Rc};
use uuid::Uuid;

pub(in crate::app::canvas) const PORT_SIZE: f64 = 18.;
const PREVIEW_ROWS: i32 = 3;

pub(in crate::app::canvas) struct Card {
    pub root: CardRoot,
    title_marker: Label,
    title: Label,
    preview: preview::Preview,
    links: GtkBox,
    link_count: Label,
    tasks: Label,
    more: Label,
    done: Button,
    shown: RefCell<Option<NodeView>>,
}

impl Card {
    pub fn new(v: &Rc<CanvasView>, id: Uuid) -> Self {
        let root = CardRoot::new(6);
        root.add_css_class("canvas-card");
        root.set_widget_name(&id.to_string());
        root.set_size_request(port::CARD_WIDTH as i32, port::CARD_HEIGHT as i32);

        let head = GtkBox::new(Orientation::Horizontal, 6);
        let title_marker = preview::marker_label();
        head.append(&title_marker);
        let title = Label::new(None);
        title.set_xalign(0.);
        title.set_hexpand(true);
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title.add_css_class("card-title");
        head.append(&title);
        let done = Button::new();
        done.set_child(Some(&icons::image(icons::CHECK)));
        done.add_css_class("card-done");
        done.set_focusable(false);
        done.set_valign(gtk::Align::Start);
        let weak = Rc::downgrade(v);
        done.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                v.toggle_done(id);
            }
        });
        head.append(&done);
        root.append(&head);

        let preview = preview::Preview::new();
        preview.root.set_hexpand(true);
        preview.root.set_vexpand(true);
        root.append(&preview.root);

        let meta = GtkBox::new(Orientation::Horizontal, 10);
        meta.add_css_class("card-meta");
        let links = GtkBox::new(Orientation::Horizontal, 3);
        links.append(&icons::image(icons::LINK));
        let link_count = Label::new(None);
        links.append(&link_count);
        meta.append(&links);
        let tasks = Label::new(None);
        meta.append(&tasks);
        let more = Label::new(None);
        more.set_hexpand(true);
        more.set_xalign(1.);
        meta.append(&more);
        root.append(&meta);

        input::card::attach(v, &root, id);
        Self {
            root,
            title_marker,
            title,
            preview,
            links,
            link_count,
            tasks,
            more,
            done,
            shown: RefCell::new(None),
        }
    }

    pub fn update(&self, node: &NodeView) {
        let mut shown = self.shown.borrow_mut();
        if shown.as_ref() == Some(node) {
            return;
        }
        if let Some(old) = shown.as_ref() {
            if old.color != node.color {
                self.root.remove_css_class(&old.color);
            }
        }
        self.root.add_css_class(&node.color);
        self.title.set_text(node.display_title());
        preview::set_marker(&self.title_marker, node.title_marker.as_ref(), 0);
        let (rows, unseen) = markdown::fit_preview(&node.lines, PREVIEW_ROWS);
        self.preview.set(&rows);
        let unseen = unseen + node.remaining;
        self.more.set_visible(unseen > 0);
        self.more.set_text(&format!("외 {unseen}줄"));
        if matches!(node.title_marker, Some(markdown::LineMarker::Task(true))) {
            self.title.add_css_class("line-done");
        } else {
            self.title.remove_css_class("line-done");
        }
        if node.title.is_empty() {
            self.title.add_css_class("placeholder");
        } else {
            self.title.remove_css_class("placeholder");
        }
        self.links.set_visible(!node.next.is_empty());
        self.link_count.set_text(&node.next.len().to_string());
        self.tasks.set_visible(node.tasks.1 > 0);
        self.tasks
            .set_text(&format!("할 일 {}/{}", node.tasks.0, node.tasks.1));
        if node.done {
            self.root.add_css_class("done");
        } else {
            self.root.remove_css_class("done");
        }
        let label = if node.done {
            "완료 취소"
        } else {
            "완료로 표시"
        };
        self.done.set_tooltip_text(Some(label));
        self.done
            .update_property(&[gtk::accessible::Property::Label(label)]);
        self.root
            .update_property(&[gtk::accessible::Property::Label(&format!(
                "{} · 선택 및 이동",
                node.display_title()
            ))]);
        *shown = Some(node.clone());
    }

    pub fn set_selected(&self, selected: bool) {
        if selected {
            self.root.add_css_class("selected");
        } else {
            self.root.remove_css_class("selected");
        }
    }
}

#[cfg(test)]
pub(in crate::app::canvas) fn title_text(card: &Card) -> String {
    card.title.text().to_string()
}

#[cfg(test)]
pub(in crate::app::canvas) fn more_text(card: &Card) -> Option<String> {
    card.more.is_visible().then(|| card.more.text().to_string())
}

#[cfg(test)]
pub(in crate::app::canvas) fn title_marker_classes(card: &Card) -> Vec<String> {
    card.title_marker
        .css_classes()
        .iter()
        .map(|c| c.to_string())
        .collect()
}

#[cfg(test)]
pub(in crate::app::canvas) fn preview_rows(card: &Card) -> Vec<(String, String)> {
    card.preview.shown_rows()
}
