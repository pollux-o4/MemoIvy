use super::super::{input, port, port::NodeView, CanvasView};
use super::card_root::CardRoot;
use crate::presentation::icons;
use gtk::{prelude::*, Box as GtkBox, Button, Label, Orientation};
use std::{cell::RefCell, rc::Rc};
use uuid::Uuid;

pub(in super::super) const PORT_SIZE: f64 = 18.;

pub(in super::super) struct Card {
    pub root: CardRoot,
    title: Label,
    preview: Label,
    links: GtkBox,
    link_count: Label,
    tasks: Label,
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

        let preview = Label::new(None);
        preview.set_xalign(0.);
        preview.set_yalign(0.);
        preview.set_wrap(true);
        preview.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        preview.set_lines(3);
        preview.set_ellipsize(gtk::pango::EllipsizeMode::End);
        preview.set_max_width_chars(1);
        preview.set_hexpand(true);
        preview.set_vexpand(true);
        preview.add_css_class("card-preview");
        root.append(&preview);

        let meta = GtkBox::new(Orientation::Horizontal, 10);
        meta.add_css_class("card-meta");
        let links = GtkBox::new(Orientation::Horizontal, 3);
        links.append(&icons::image(icons::LINK));
        let link_count = Label::new(None);
        links.append(&link_count);
        meta.append(&links);
        let tasks = Label::new(None);
        meta.append(&tasks);
        root.append(&meta);

        input::card::attach(v, &root, id);
        Self {
            root,
            title,
            preview,
            links,
            link_count,
            tasks,
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
        self.preview.set_text(&node.preview);
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
pub(in super::super) fn title_text(card: &Card) -> String {
    card.title.text().to_string()
}
