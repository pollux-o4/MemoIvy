//! FMT-004 메모 이미지. 참조 줄 위에 줄 간격으로 자리를 만들고 그 자리에 이미지 위젯을 올린다.
//! 원문은 그대로 두고 너비·빼기만 원문 편집으로 반영해 undo·저장 계약을 그대로 따른다.
mod block;
mod import;

use super::*;
use crate::persistence::{AssetId, AssetStore};
use block::Block;
pub(super) use import::connect_import;

/// 편집 창이 넘겨주는 저장소와 알림 통로. 테스트 편집기는 없이 만든다.
pub(in crate::app) struct MediaHost {
    pub store: AssetStore,
    /// 거부 이유 등 사용자에게 보일 한 줄.
    pub notice: Box<dyn Fn(&str)>,
}

/// 앞 줄과 이미지 사이, 이미지와 원문 줄 사이 간격.
const TOP: i32 = 6;
const GAP: i32 = 4;
/// 아직 읽지 못했거나 찾을 수 없는 이미지의 자리 높이.
const PLACEHOLDER: i32 = 72;

#[derive(Clone)]
enum Texture {
    Loading,
    Ready(gdk::Texture),
    Missing,
}

#[derive(Default)]
pub(super) struct MediaState {
    host: Option<MediaHost>,
    blocks: RefCell<Vec<Rc<Block>>>,
    textures: RefCell<HashMap<String, Texture>>,
    /// 이 메모 안에서 끌고 있는 이미지 줄의 시작. 놓으면 복사가 아니라 이동이다.
    moving: Cell<Option<i32>>,
    /// 끌기 중 넣을 자리를 알리는 가로선.
    drop_line: RefCell<Option<gtk::Box>>,
}

impl MediaState {
    pub(super) fn new(host: Option<MediaHost>) -> Self {
        Self {
            host,
            ..Self::default()
        }
    }
}

impl LiveEditor {
    /// 이미지 줄의 위젯·간격·원문 태그를 맞춘다. refresh 끝에서 부른다.
    pub(super) fn sync_images(self: &Rc<Self>, view: &TextView, document: &Document) {
        let images: &[markdown::Image] = if self.source.get() || self.media.host.is_none() {
            &[]
        } else {
            &document.images
        };
        let buffer = view.buffer();
        let mut blocks = self.media.blocks.borrow_mut();
        while blocks.len() < images.len() {
            let block = Block::new(self);
            view.add_overlay(&block.root, 0, 0);
            buffer.tag_table().add(&block.spacing);
            blocks.push(block);
        }
        let active = active_lines(&buffer);
        let mut quiet = Vec::new();
        let mut revealed = Vec::new();
        for (index, block) in blocks.iter().enumerate() {
            let Some(image) = images.get(index) else {
                block.hide(&buffer);
                continue;
            };
            let texture = self.texture(&image.target);
            // 목록 항목의 이어쓰기 줄이면 그 항목 본문에 맞춰 들여 그린다.
            let x = text_x(view, document, image.line.start);
            let available = (view.width() - x - view.right_margin()).max(0);
            block.x.set(x);
            block.bind(image, texture, available, view.is_editable());
            reconcile(&buffer, &block.spacing, vec![image.line.clone()]);
            if image.line.end >= active.start && image.line.start <= active.end {
                revealed.push(image.line.clone());
            } else {
                quiet.push(image.line.clone());
            }
        }
        drop(blocks);
        reconcile(&buffer, &self.tags.media_quiet, quiet);
        reconcile(&buffer, &self.tags.media_source, revealed);
        self.place_images();
    }

    /// 커서가 없는 이미지 줄은 첫 글자만 투명하게 남기고 나머지를 숨긴다.
    /// 긴 참조가 여러 줄로 감겨 이미지 아래가 비지 않게 하고, 커서가 놓일 자리는 남긴다.
    pub(super) fn quiet_image_tails(
        &self,
        document: &Document,
        active: &Range<i32>,
    ) -> Vec<Range<i32>> {
        if self.source.get() || self.media.host.is_none() {
            return Vec::new();
        }
        document
            .images
            .iter()
            .filter(|image| image.line.end < active.start || image.line.start > active.end)
            .map(|image| image.line.start + 1..image.line.end)
            .collect()
    }

    /// 각 이미지를 자기 줄 위 간격에 놓는다. 줄 높이가 확정되는 프레임마다 다시 부른다.
    pub(super) fn place_images(&self) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        for block in self.media.blocks.borrow().iter() {
            let Some(line) = block.line() else {
                continue;
            };
            let (top, _) = view.line_yrange(&buffer.iter_at_offset(line.start));
            view.move_overlay(&block.root, block.x.get(), top + TOP);
        }
    }

    /// 읽은 텍스처. 처음 보는 대상은 화면 밖 스레드에서 읽고 끝나면 다시 그린다.
    fn texture(self: &Rc<Self>, target: &str) -> Texture {
        if let Some(texture) = self.media.textures.borrow().get(target) {
            return texture.clone();
        }
        let path = target
            .strip_prefix(markdown::IMAGE_SCHEME)
            .and_then(AssetId::parse)
            .zip(self.media.host.as_ref())
            .and_then(|(id, host)| host.store.path(&id));
        let Some(path) = path else {
            self.media
                .textures
                .borrow_mut()
                .insert(target.to_owned(), Texture::Missing);
            return Texture::Missing;
        };
        self.media
            .textures
            .borrow_mut()
            .insert(target.to_owned(), Texture::Loading);
        let weak = Rc::downgrade(self);
        let target = target.to_owned();
        glib::spawn_future_local(async move {
            let loaded = gtk::gio::spawn_blocking(move || {
                gdk::Texture::from_filename(&path).map_err(|e| e.to_string())
            })
            .await;
            let Some(editor) = weak.upgrade() else {
                return;
            };
            let texture = match loaded {
                Ok(Ok(texture)) => Texture::Ready(texture),
                _ => Texture::Missing,
            };
            editor.media.textures.borrow_mut().insert(target, texture);
            editor.schedule();
        });
        Texture::Loading
    }

    /// 이미지 줄을 새 원문으로 바꾼다. 줄이 그새 바뀌었으면 아무것도 하지 않는다.
    fn rewrite_image(
        &self,
        line_start: i32,
        edit: impl FnOnce(&markdown::Image) -> Option<String>,
    ) {
        let Some(view) = self.view.upgrade().filter(|view| view.is_editable()) else {
            return;
        };
        if self.input.composing.get() {
            return;
        }
        let Some(image) = self
            .document
            .borrow()
            .images
            .iter()
            .find(|image| image.line.start == line_start)
            .cloned()
        else {
            return;
        };
        let buffer = view.buffer();
        buffer.begin_user_action();
        match edit(&image) {
            Some(line) => {
                // 들여쓰기는 두고 문법만 바꾼다.
                let source = super::super::source_slice(&buffer, image.line.clone());
                let indent = source.chars().take_while(|c| c.is_whitespace()).count() as i32;
                let mut start = buffer.iter_at_offset(image.line.start + indent);
                let mut end = buffer.iter_at_offset(image.line.end);
                buffer.delete(&mut start, &mut end);
                buffer.insert(&mut start, &line);
            }
            None => import::delete_line(&buffer, image.line.clone()),
        }
        buffer.end_user_action();
    }

    fn set_image_width(&self, line_start: i32, width: Option<i32>) {
        let unchanged = self
            .document
            .borrow()
            .images
            .iter()
            .any(|image| image.line.start == line_start && image.width == width);
        if !unchanged {
            self.rewrite_image(line_start, |image| {
                Some(markdown::Image::markup(&image.alt, width, &image.target))
            });
        }
    }

    fn remove_image(&self, line_start: i32) {
        self.rewrite_image(line_start, |_| None);
    }

    fn notice(&self, text: &str) {
        if let Some(host) = &self.media.host {
            (host.notice)(text);
        }
    }
}

/// 이미지·넣을 자리의 왼쪽. 목록 항목에 속한 줄이면 항목 본문 시작이다.
fn text_x(view: &TextView, document: &Document, offset: i32) -> i32 {
    document
        .item_at(offset)
        .map_or(view.left_margin(), |(_, item)| {
            marker_x(view, item.depth) + GUTTER
        })
}

impl LiveEditor {
    /// 끌기 중 놓을 줄 아래(빈 줄이면 그 자리)에 가로선을 보인다. 놓을 수 없으면 거짓.
    fn show_drop_line(&self, y: f64) -> bool {
        let Some(view) = self.view.upgrade().filter(|view| view.is_editable()) else {
            return false;
        };
        let buffer = view.buffer();
        let (_, by) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, 0, y as i32);
        let iter = view.line_at_y(by).0;
        let placement = self.placement(&buffer, &iter);
        let (top, height) = view.line_yrange(&iter);
        let mut end = iter;
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        let x = text_x(&view, &self.document.borrow(), end.offset());
        let y = if placement.blank { top } else { top + height };
        let mut slot = self.media.drop_line.borrow_mut();
        let line = slot.get_or_insert_with(|| {
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            line.add_css_class("md-drop-line");
            line.set_can_target(false);
            view.add_overlay(&line, 0, 0);
            line
        });
        line.set_size_request((view.width() - x - view.right_margin()).max(0), 2);
        view.move_overlay(line, x, y - 1);
        line.set_visible(true);
        true
    }

    fn hide_drop_line(&self) {
        if let Some(line) = self.media.drop_line.borrow().as_ref() {
            line.set_visible(false);
        }
    }
}

/// 표시 크기. 너비가 없으면 원래 크기와 본문 폭 중 작은 쪽이다.
fn display_size(width: Option<i32>, natural: (i32, i32), available: i32) -> (i32, i32) {
    let (w, h) = (natural.0.max(1), natural.1.max(1));
    let min = (*markdown::IMAGE_WIDTH.start()).min(available);
    let shown = width.unwrap_or(w).min(available).max(min);
    (
        shown,
        (f64::from(shown) * f64::from(h) / f64::from(w)).round() as i32,
    )
}

#[cfg(test)]
pub(in crate::app) mod tests;

#[cfg(test)]
mod unit {
    use super::display_size;

    #[test]
    fn fmt_004_display_size_keeps_ratio_within_text_width() {
        assert_eq!(display_size(None, (800, 400), 300), (300, 150));
        assert_eq!(display_size(None, (200, 100), 300), (200, 100));
        assert_eq!(display_size(Some(120), (800, 400), 300), (120, 60));
        assert_eq!(display_size(Some(10), (800, 400), 300), (64, 32));
        assert_eq!(display_size(Some(120), (800, 400), 40), (40, 20));
    }
}
