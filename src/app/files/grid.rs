//! 썸네일 격자. 썸네일은 줄인 크기로 화면 밖 스레드에서 읽어 캐시하고, 각 칸은 메모로 끌 수 있다.
use crate::persistence::{AssetEntry, AssetId};
use gtk::{gdk, gdk_pixbuf::Pixbuf, glib, prelude::*};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// 정사각 썸네일 한 변(논리 px). 2배 화면에서도 선명하도록 두 배로 읽는다.
const TILE: i32 = 120;

pub(super) struct Grid {
    pub scroll: gtk::ScrolledWindow,
    flow: gtk::FlowBox,
    textures: Rc<RefCell<HashMap<AssetId, gdk::Texture>>>,
    ids: Rc<RefCell<Vec<AssetId>>>,
    syncing: Rc<std::cell::Cell<bool>>,
}

impl Grid {
    pub fn new() -> Self {
        let flow = gtk::FlowBox::new();
        flow.set_selection_mode(gtk::SelectionMode::Single);
        flow.set_homogeneous(true);
        flow.set_halign(gtk::Align::Start);
        flow.set_min_children_per_line(2);
        flow.set_max_children_per_line(8);
        flow.set_valign(gtk::Align::Start);
        flow.add_css_class("files-grid");
        let scroll = gtk::ScrolledWindow::builder()
            .child(&flow)
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        Self {
            scroll,
            flow,
            textures: Rc::default(),
            ids: Rc::default(),
            syncing: Rc::default(),
        }
    }

    pub fn connect_selected(&self, on: impl Fn(AssetId) + 'static) {
        let (ids, syncing) = (self.ids.clone(), self.syncing.clone());
        self.flow.connect_selected_children_changed(move |flow| {
            if syncing.get() {
                return;
            }
            let index = flow.selected_children().first().map(|child| child.index());
            if let Some(id) = index.and_then(|i| ids.borrow().get(i as usize).cloned()) {
                on(id);
            }
        });
    }

    /// 목록을 다시 그린다. 칸 수가 적어 매번 새로 만든다.
    pub fn set(&self, entries: &[AssetEntry], selected: Option<&AssetId>) {
        self.syncing.set(true);
        while let Some(child) = self.flow.first_child() {
            self.flow.remove(&child);
        }
        *self.ids.borrow_mut() = entries.iter().map(|e| e.id.clone()).collect();
        for entry in entries {
            let tile = self.tile(entry);
            self.flow.append(&tile);
            if selected == Some(&entry.id) {
                if let Some(child) = tile.parent().and_downcast::<gtk::FlowBoxChild>() {
                    self.flow.select_child(&child);
                }
            }
        }
        self.syncing.set(false);
    }

    fn tile(&self, entry: &AssetEntry) -> gtk::Widget {
        let picture = gtk::Image::new();
        picture.set_pixel_size(TILE);
        picture.set_overflow(gtk::Overflow::Hidden);
        picture.add_css_class("file-tile");
        picture.update_property(&[gtk::accessible::Property::Label("이미지 파일")]);
        let cached = self.textures.borrow().get(&entry.id).cloned();
        match cached {
            Some(texture) => picture.set_paintable(Some(&texture)),
            None => self.load(entry, &picture),
        }
        picture.add_controller(drag_source(&entry.id, &self.textures));
        picture.set_tooltip_text(Some("메모로 끌어 놓아 다시 쓰기"));
        picture.upcast()
    }

    fn load(&self, entry: &AssetEntry, picture: &gtk::Image) {
        let (path, id) = (entry.path.clone(), entry.id.clone());
        let (textures, picture) = (self.textures.clone(), picture.downgrade());
        glib::spawn_future_local(async move {
            let decoded = gtk::gio::spawn_blocking(move || square(&path, TILE * 2).map(raw)).await;
            let Ok(Some((bytes, width, height, stride, alpha))) = decoded else {
                if let Some(picture) = picture.upgrade() {
                    picture.add_css_class("unreadable");
                    picture.set_tooltip_text(Some("미리보기를 만들 수 없어요"));
                }
                return;
            };
            let texture = texture(bytes, width, height, stride, alpha);
            textures.borrow_mut().insert(id, texture.clone());
            if let Some(picture) = picture.upgrade() {
                picture.set_paintable(Some(&texture));
            }
        });
    }
}

type Raw = (glib::Bytes, i32, i32, i32, bool);

/// 화면 밖 스레드에서 넘길 수 있는 픽셀 묶음.
pub(super) fn raw(pixbuf: Pixbuf) -> Raw {
    (
        pixbuf.read_pixel_bytes(),
        pixbuf.width(),
        pixbuf.height(),
        pixbuf.rowstride(),
        pixbuf.has_alpha(),
    )
}

pub(super) fn texture(
    bytes: glib::Bytes,
    width: i32,
    height: i32,
    stride: i32,
    alpha: bool,
) -> gdk::Texture {
    let format = if alpha {
        gdk::MemoryFormat::R8g8b8a8
    } else {
        gdk::MemoryFormat::R8g8b8
    };
    gdk::MemoryTexture::new(width, height, format, &bytes, stride as usize).upcast()
}

/// 가운데를 잘라 채운 정사각 썸네일.
fn square(path: &std::path::Path, side: i32) -> Option<Pixbuf> {
    let (_, w, h) = Pixbuf::file_info(path)?;
    let scale = f64::from(side) / f64::from(w.min(h).max(1));
    let (sw, sh) = (
        ((f64::from(w) * scale).ceil() as i32).max(side),
        ((f64::from(h) * scale).ceil() as i32).max(side),
    );
    let scaled = Pixbuf::from_file_at_scale(path, sw, sh, false).ok()?;
    scaled
        .new_subpixbuf((sw - side) / 2, (sh - side) / 2, side, side)
        .copy()
}

/// 편집 창은 해시를, 다른 앱은 참조 문법 글자를 받는다.
fn drag_source(
    id: &AssetId,
    textures: &Rc<RefCell<HashMap<AssetId, gdk::Texture>>>,
) -> gtk::DragSource {
    let source = gtk::DragSource::new();
    source.set_actions(gdk::DragAction::COPY);
    let (id, textures) = (id.clone(), textures.clone());
    source.connect_prepare(move |source, _, _| {
        if let Some(texture) = textures.borrow().get(&id) {
            source.set_icon(Some(texture), 24, 24);
        }
        let markup = crate::presentation::markdown::Image::markup("", None, &id.target());
        Some(gdk::ContentProvider::new_union(&[
            gdk::ContentProvider::for_bytes(
                crate::app::widgets::ASSET_MIME,
                &glib::Bytes::from(id.as_str().as_bytes()),
            ),
            gdk::ContentProvider::for_value(&markup.to_value()),
        ]))
    });
    source
}
