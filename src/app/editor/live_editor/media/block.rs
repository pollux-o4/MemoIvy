//! 이미지 한 장의 위젯. 호버하면 도구와 모서리 핸들이 페이드로 나타난다.
//! 끄는 동안은 위젯과 줄 간격만 바꾸고, 놓을 때 원문을 한 번 고친다.
use super::*;

pub(super) struct Block {
    pub root: gtk::Overlay,
    pub(super) picture: gtk::Picture,
    pub(super) placeholder: gtk::Label,
    remove: gtk::Button,
    handle: gtk::Box,
    /// 이 이미지 줄에만 거는 위 간격. 위젯마다 하나라 다른 줄에 영향이 없다.
    pub spacing: TextTag,
    image: RefCell<Option<markdown::Image>>,
    texture: RefCell<Option<gdk::Texture>>,
    /// 지금 그리는 너비와 높이/너비 비율, 본문 폭.
    shown: Cell<(i32, f64, i32)>,
    /// 끌기 시작 너비. 끄는 중이면 Some.
    drag: Cell<Option<i32>>,
    /// 왼쪽 위치. 목록 항목 안이면 들여 그린다.
    pub x: Cell<i32>,
}

impl Block {
    pub fn new(editor: &Rc<LiveEditor>) -> Rc<Self> {
        let picture = gtk::Picture::new();
        picture.set_content_fit(gtk::ContentFit::Fill);
        picture.set_can_shrink(true);
        picture.add_css_class("md-image-picture");
        let placeholder = gtk::Label::new(None);
        placeholder.add_css_class("md-image-placeholder");
        let root = gtk::Overlay::new();
        root.add_css_class("md-image");
        root.set_overflow(gtk::Overflow::Hidden);
        let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
        frame.append(&picture);
        frame.append(&placeholder);
        root.set_child(Some(&frame));

        let tools = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        tools.add_css_class("md-image-tools");
        tools.set_halign(gtk::Align::End);
        tools.set_valign(gtk::Align::Start);
        let expand = tool(crate::presentation::icons::EXPAND, "크게 보기");
        let remove = tool(
            crate::presentation::icons::TRASH,
            "메모에서 빼기 · 파일은 남아요",
        );
        tools.append(&expand);
        tools.append(&remove);
        root.add_overlay(&tools);

        let handle = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        handle.add_css_class("md-image-handle");
        handle.set_halign(gtk::Align::End);
        handle.set_valign(gtk::Align::End);
        handle.set_cursor_from_name(Some("se-resize"));
        handle.set_tooltip_text(Some("끌어서 크기 조절 · 두 번 눌러 원래 크기"));
        root.add_overlay(&handle);

        let block = Rc::new(Self {
            root,
            picture,
            placeholder,
            remove,
            handle,
            spacing: TextTag::new(None),
            image: RefCell::new(None),
            texture: RefCell::new(None),
            shown: Cell::new((0, 1.0, 0)),
            drag: Cell::new(None),
            x: Cell::new(0),
        });
        block.connect(editor, &expand);
        block
    }

    fn connect(self: &Rc<Self>, editor: &Rc<LiveEditor>, expand: &gtk::Button) {
        let hover = gtk::EventControllerMotion::new();
        let root = self.root.downgrade();
        hover.connect_enter(move |_, _, _| {
            if let Some(root) = root.upgrade() {
                root.add_css_class("hover");
            }
        });
        let weak = Rc::downgrade(self);
        hover.connect_leave(move |_| {
            if let Some(block) = weak.upgrade().filter(|block| block.drag.get().is_none()) {
                block.root.remove_css_class("hover");
            }
        });
        self.root.add_controller(hover);

        let (block, owner) = (Rc::downgrade(self), Rc::downgrade(editor));
        let open = move || {
            let (Some(block), Some(editor)) = (block.upgrade(), owner.upgrade()) else {
                return;
            };
            let texture = block.texture.borrow().clone();
            let window = editor
                .view
                .upgrade()
                .and_then(|view| view.root().and_downcast::<gtk::Window>());
            if let (Some(texture), Some(window)) = (texture, window) {
                let alt = block
                    .image
                    .borrow()
                    .as_ref()
                    .map(|i| i.alt.clone())
                    .unwrap_or_default();
                crate::app::widgets::show_image(&window, &texture, &alt);
            }
        };
        let open = Rc::new(open);
        let clicked = open.clone();
        expand.connect_clicked(move |_| clicked());
        let double = gtk::GestureClick::new();
        double.connect_pressed(move |gesture, count, _, _| {
            if count == 2 {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                open();
            }
        });
        self.picture.add_controller(double);

        let (block, owner) = (Rc::downgrade(self), Rc::downgrade(editor));
        self.remove.connect_clicked(move |_| {
            if let (Some(block), Some(editor)) = (block.upgrade(), owner.upgrade()) {
                if let Some(line) = block.line() {
                    editor.remove_image(line.start);
                }
            }
        });

        let drag = gtk::GestureDrag::new();
        let weak = Rc::downgrade(self);
        drag.connect_drag_begin(move |gesture, _, _| {
            if let Some(block) = weak.upgrade() {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                block.drag.set(Some(block.shown.get().0));
                block.root.add_css_class("resizing");
            }
        });
        let weak = Rc::downgrade(self);
        drag.connect_drag_update(move |_, dx, dy| {
            let Some(block) = weak.upgrade() else {
                return;
            };
            let Some(start) = block.drag.get() else {
                return;
            };
            let (_, ratio, available) = block.shown.get();
            // 모서리를 대각선으로 끌므로 세로 이동도 너비로 환산해 평균한다.
            let delta = (dx + dy / ratio.max(0.01)) / 2.0;
            let min = (*markdown::IMAGE_WIDTH.start()).min(available);
            let width = (f64::from(start) + delta).round() as i32;
            block.resize(width.clamp(min, available.max(min)), ratio, available);
        });
        let (weak, owner) = (Rc::downgrade(self), Rc::downgrade(editor));
        drag.connect_drag_end(move |_, _, _| {
            let Some(block) = weak.upgrade() else {
                return;
            };
            block.root.remove_css_class("resizing");
            let Some(start) = block.drag.take() else {
                return;
            };
            let width = block.shown.get().0;
            if let (Some(editor), Some(line), true) =
                (owner.upgrade(), block.line(), width != start)
            {
                editor.set_image_width(line.start, Some(width));
            }
        });
        self.handle.add_controller(drag);
        let reset = gtk::GestureClick::new();
        let (weak, owner) = (Rc::downgrade(self), Rc::downgrade(editor));
        reset.connect_pressed(move |gesture, count, _, _| {
            if count != 2 {
                return;
            }
            gesture.set_state(gtk::EventSequenceState::Claimed);
            if let (Some(block), Some(editor)) = (weak.upgrade(), owner.upgrade()) {
                if let Some(line) = block.line() {
                    editor.set_image_width(line.start, None);
                }
            }
        });
        self.handle.add_controller(reset);
        self.connect_move(editor);
    }

    /// 이미지를 끌어 다른 줄로 옮긴다. 다른 메모에 놓으면 그 메모에는 복사된다.
    fn connect_move(self: &Rc<Self>, editor: &Rc<LiveEditor>) {
        let source = gtk::DragSource::new();
        source.set_actions(gdk::DragAction::MOVE | gdk::DragAction::COPY);
        let (weak, owner) = (Rc::downgrade(self), Rc::downgrade(editor));
        source.connect_prepare(move |source, x, y| {
            let block = weak.upgrade()?;
            let editor = owner.upgrade()?;
            if !editor.view.upgrade()?.is_editable() || block.drag.get().is_some() {
                return None;
            }
            let image = block.image.borrow().clone()?;
            let hash = image
                .target
                .strip_prefix(markdown::IMAGE_SCHEME)?
                .to_owned();
            editor.media.moving.set(Some(image.line.start));
            source.set_icon(
                Some(&gtk::WidgetPaintable::new(Some(&block.picture))),
                x as i32,
                y as i32,
            );
            let markup = markdown::Image::markup(&image.alt, image.width, &image.target);
            Some(gdk::ContentProvider::new_union(&[
                gdk::ContentProvider::for_bytes(
                    crate::app::widgets::ASSET_MIME,
                    &glib::Bytes::from(hash.as_bytes()),
                ),
                gdk::ContentProvider::for_value(&markup.to_value()),
            ]))
        });
        let root = self.root.downgrade();
        source.connect_drag_begin(move |_, _| {
            if let Some(root) = root.upgrade() {
                root.add_css_class("dragging");
            }
        });
        let (root, owner) = (self.root.downgrade(), Rc::downgrade(editor));
        source.connect_drag_end(move |_, _, _| {
            if let Some(root) = root.upgrade() {
                root.remove_css_class("dragging");
            }
            if let Some(editor) = owner.upgrade() {
                editor.media.moving.set(None);
                editor.hide_drop_line();
            }
        });
        self.picture.add_controller(source);
    }

    pub fn line(&self) -> Option<Range<i32>> {
        self.image
            .borrow()
            .as_ref()
            .filter(|_| self.root.is_visible())
            .map(|image| image.line.clone())
    }

    pub fn bind(&self, image: &markdown::Image, texture: Texture, available: i32, editable: bool) {
        let alt = if image.alt.is_empty() {
            "이미지"
        } else {
            &image.alt
        };
        self.root
            .update_property(&[gtk::accessible::Property::Label(alt)]);
        self.remove.set_sensitive(editable);
        self.handle.set_visible(editable);
        *self.image.borrow_mut() = Some(image.clone());
        self.root.set_visible(true);
        let ready = match texture {
            Texture::Ready(texture) => Some(texture),
            Texture::Loading => {
                self.placeholder.set_text("");
                None
            }
            Texture::Missing => {
                self.placeholder.set_text("이미지를 찾을 수 없음");
                None
            }
        };
        self.picture.set_visible(ready.is_some());
        self.placeholder.set_visible(ready.is_none());
        self.handle.set_visible(editable && ready.is_some());
        if self.texture.borrow().as_ref() != ready.as_ref() {
            self.picture.set_paintable(ready.as_ref());
            *self.texture.borrow_mut() = ready.clone();
        }
        let Some(texture) = ready else {
            let width = available.clamp(0, 240);
            self.resize(
                width,
                f64::from(PLACEHOLDER) / f64::from(width.max(1)),
                available,
            );
            return;
        };
        if self.drag.get().is_some() {
            return;
        }
        let (w, h) = display_size(image.width, (texture.width(), texture.height()), available);
        self.resize(w, f64::from(h) / f64::from(w.max(1)), available);
    }

    /// 위젯 크기와 줄 간격을 함께 바꾼다. 끌기 중에는 GTK가 포인터 이벤트를 프레임 단위로 모아 준다.
    fn resize(&self, width: i32, ratio: f64, available: i32) {
        let height = (f64::from(width) * ratio).round() as i32;
        self.shown.set((width, ratio, available));
        self.root.set_size_request(width, height);
        let above = TOP + height + GAP;
        if self.spacing.pixels_above_lines() != above {
            self.spacing.set_pixels_above_lines(above);
        }
    }

    pub fn hide(&self, buffer: &TextBuffer) {
        if self.root.is_visible() {
            self.root.set_visible(false);
            self.root.remove_css_class("hover");
        }
        *self.image.borrow_mut() = None;
        reconcile(buffer, &self.spacing, Vec::new());
    }
}

fn tool(icon: &'static str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&crate::presentation::icons::image(icon)));
    button.add_css_class("md-image-tool");
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button.set_focusable(false);
    button
}
