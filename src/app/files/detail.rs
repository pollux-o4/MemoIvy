//! 고른 파일의 상세. 미리보기·크기·사용처와 다시 쓰기·삭제 동작을 보인다.
use crate::persistence::{AssetEntry, AssetId};
use crate::presentation::icons;
use gtk::{gdk, glib, prelude::*, Box as GtkBox, Button, Label, Orientation};
use std::{cell::RefCell, rc::Rc};
use uuid::Uuid;

/// 상세 미리보기 한 변(논리 px).
const PREVIEW: i32 = 200;

type OpenNote = Rc<RefCell<Option<Box<dyn Fn(Uuid)>>>>;
type NoteTitle<'a> = dyn Fn(Uuid) -> Option<String> + 'a;

pub(super) struct Detail {
    pub root: GtkBox,
    picture: gtk::Image,
    meta: Label,
    pub(super) heading: Label,
    pub(super) notes: GtkBox,
    expand: Button,
    copy: Button,
    pub(super) delete: Button,
    status: Label,
    entry: Rc<RefCell<Option<AssetEntry>>>,
    on_open: OpenNote,
}

fn action(icon: &'static str, label: &str) -> Button {
    let button = Button::new();
    let row = GtkBox::new(Orientation::Horizontal, 6);
    row.append(&icons::image(icon));
    row.append(&Label::new(Some(label)));
    button.set_child(Some(&row));
    button.add_css_class("memoivy-action");
    button
}

impl Detail {
    pub fn new() -> Self {
        let root = GtkBox::new(Orientation::Vertical, 8);
        root.add_css_class("files-detail");
        root.set_size_request(260, -1);
        let picture = gtk::Image::new();
        picture.set_pixel_size(PREVIEW);
        picture.set_size_request(-1, PREVIEW);
        picture.add_css_class("files-preview");
        let meta = Label::new(None);
        meta.set_xalign(0.);
        meta.add_css_class("menu-hint");
        let heading = Label::new(None);
        heading.set_xalign(0.);
        heading.add_css_class("files-heading");
        let notes = GtkBox::new(Orientation::Vertical, 2);
        let actions = GtkBox::new(Orientation::Vertical, 4);
        let expand = action(icons::EXPAND, "크게 보기");
        let copy = action(icons::COPY, "참조 복사");
        copy.set_tooltip_text(Some("메모에 붙여 넣으면 같은 파일을 다시 써요"));
        let delete = action(icons::TRASH, "삭제…");
        delete.add_css_class("destructive");
        actions.append(&expand);
        actions.append(&copy);
        actions.append(&delete);
        let status = Label::new(None);
        status.set_xalign(0.);
        status.set_wrap(true);
        status.add_css_class("status");
        let spacer = GtkBox::new(Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        for widget in [
            picture.upcast_ref::<gtk::Widget>(),
            meta.upcast_ref(),
            heading.upcast_ref(),
            notes.upcast_ref(),
            spacer.upcast_ref(),
            actions.upcast_ref(),
            status.upcast_ref(),
        ] {
            root.append(widget);
        }
        let detail = Self {
            root,
            picture,
            meta,
            heading,
            notes,
            expand,
            copy,
            delete,
            status,
            entry: Rc::default(),
            on_open: Rc::default(),
        };
        detail.connect();
        detail
    }

    fn connect(&self) {
        let entry = self.entry.clone();
        let status = self.status.clone();
        self.copy.connect_clicked(move |button| {
            if let Some(entry) = entry.borrow().as_ref() {
                let markup =
                    crate::presentation::markdown::Image::markup("", None, &entry.id.target());
                button.clipboard().set_text(&markup);
                status.set_text("복사했어요. 메모에 붙여 넣으세요.");
            }
        });
        let entry = self.entry.clone();
        self.expand.connect_clicked(move |button| {
            let Some(path) = entry.borrow().as_ref().map(|e| e.path.clone()) else {
                return;
            };
            let Some(window) = button.root().and_downcast::<gtk::Window>() else {
                return;
            };
            glib::spawn_future_local(async move {
                let loaded = gtk::gio::spawn_blocking(move || {
                    gdk::Texture::from_filename(&path).map_err(|e| e.to_string())
                })
                .await;
                if let Ok(Ok(texture)) = loaded {
                    crate::app::widgets::show_image(&window, &texture, "");
                }
            });
        });
    }

    pub fn connect_delete(&self, on: impl Fn(AssetId) + 'static) {
        let entry = self.entry.clone();
        self.delete.connect_clicked(move |_| {
            if let Some(id) = entry.borrow().as_ref().map(|e| e.id.clone()) {
                on(id);
            }
        });
    }

    pub fn connect_open_note(&self, on: impl Fn(Uuid) + 'static) {
        *self.on_open.borrow_mut() = Some(Box::new(on));
    }

    pub fn notice(&self, text: &str) {
        self.status.set_text(text);
    }

    pub fn show(&self, entry: Option<&AssetEntry>, title: &NoteTitle<'_>) {
        // 바뀔 때만 짧게 페이드한다(FILE-001).
        if self.entry.borrow().as_ref().map(|e| &e.id) != entry.map(|e| &e.id) {
            self.root.add_css_class("changing");
            self.root.add_tick_callback(|root, _| {
                root.remove_css_class("changing");
                glib::ControlFlow::Break
            });
            self.status.set_text("");
        }
        *self.entry.borrow_mut() = entry.cloned();
        self.root.set_visible(entry.is_some());
        let Some(entry) = entry else {
            return;
        };
        self.load_preview(entry);
        self.meta.set_text(&format!(
            "{} · {} 추가",
            size_text(entry.bytes),
            days_ago(entry.added)
        ));
        while let Some(child) = self.notes.first_child() {
            self.notes.remove(&child);
        }
        let used = !entry.users.is_empty();
        self.delete.set_visible(!used);
        if used {
            self.heading.set_text("사용 중인 메모");
            for id in &entry.users {
                self.notes.append(&self.note_link(*id, title(*id)));
            }
        } else {
            self.heading.set_text(&match entry.unused_since {
                Some(since) => format!("마지막 사용 · {}", days_ago(since)),
                None => "마지막 사용".to_owned(),
            });
            match entry.last_note {
                Some(id) => self.notes.append(&self.note_link(id, title(id))),
                None => self.notes.append(&muted("쓰인 적 없음")),
            }
        }
    }

    /// 고른 파일만 줄여 읽는다. 늦게 끝난 읽기는 그새 고른 파일이 바뀌었으면 버린다.
    fn load_preview(&self, entry: &AssetEntry) {
        self.picture.set_paintable(None::<&gdk::Paintable>);
        let (path, id) = (entry.path.clone(), entry.id.clone());
        let (picture, current) = (self.picture.downgrade(), self.entry.clone());
        glib::spawn_future_local(async move {
            let decoded = gtk::gio::spawn_blocking(move || {
                gtk::gdk_pixbuf::Pixbuf::from_file_at_scale(&path, PREVIEW * 2, PREVIEW * 2, true)
                    .ok()
                    .map(super::grid::raw)
            })
            .await;
            let Ok(Some((bytes, width, height, stride, alpha))) = decoded else {
                return;
            };
            if current.borrow().as_ref().map(|e| &e.id) != Some(&id) {
                return;
            }
            if let Some(picture) = picture.upgrade() {
                picture.set_paintable(Some(&super::grid::texture(
                    bytes, width, height, stride, alpha,
                )));
            }
        });
    }

    fn note_link(&self, id: Uuid, title: Option<String>) -> gtk::Widget {
        let Some(title) = title else {
            return muted("삭제된 메모").upcast();
        };
        let link = Button::with_label(&title);
        link.add_css_class("files-note");
        if let Some(label) = link.child().and_downcast::<Label>() {
            label.set_xalign(0.);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        }
        link.set_tooltip_text(Some("메모 열기"));
        let on_open = self.on_open.clone();
        link.connect_clicked(move |_| {
            if let Some(open) = on_open.borrow().as_ref() {
                open(id);
            }
        });
        link.upcast()
    }
}

fn muted(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_xalign(0.);
    label.add_css_class("menu-hint");
    label
}

fn size_text(bytes: u64) -> String {
    match bytes {
        0..1024 => format!("{bytes} B"),
        1024..1_048_576 => format!("{:.0} KB", bytes as f64 / 1024.),
        _ => format!("{:.1} MB", bytes as f64 / 1_048_576.),
    }
}

fn days_ago(at: u64) -> String {
    let days = crate::persistence::now().saturating_sub(at) / 86_400;
    match days {
        0 => "오늘".into(),
        1 => "어제".into(),
        _ => format!("{days}일 전"),
    }
}
