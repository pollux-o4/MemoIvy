//! 이미지 넣기: 클립보드 붙여넣기와 끌어 놓기. 바이트는 화면 밖 스레드에서 저장하고
//! 끝나면 참조 줄을 한 번의 사용자 동작으로 넣는다(FMT-004).
use super::*;
use crate::persistence::ImportError;

pub(in super::super) fn connect_import(view: &TextView, editor: &Rc<LiveEditor>) {
    if editor.media.host.is_none() {
        return;
    }
    let weak = Rc::downgrade(editor);
    view.connect_paste_clipboard(move |view| {
        let clipboard = view.clipboard();
        let formats = clipboard.formats();
        // 글자가 함께 있으면 글자 붙여넣기가 우선이다. 이미지 편집기 복사본은 대개 이미지뿐이다.
        if formats.contains_type(glib::GString::static_type())
            || !formats.contains_type(gdk::Texture::static_type())
        {
            return;
        }
        view.stop_signal_emission_by_name("paste-clipboard");
        let Some(editor) = weak.upgrade() else {
            return;
        };
        if !view.is_editable() {
            return;
        }
        let buffer = view.buffer();
        let at = buffer.create_mark(None, &buffer.iter_at_mark(&buffer.get_insert()), true);
        let weak = weak.clone();
        let store = editor.media.host.as_ref().unwrap().store.clone();
        glib::spawn_future_local(async move {
            let imported = match clipboard.read_texture_future().await {
                Ok(Some(texture)) => {
                    gtk::gio::spawn_blocking(move || store.import(&texture.save_to_png_bytes()))
                        .await
                        .unwrap_or_else(|_| Err(ImportError::from("이미지를 읽지 못했어요")))
                }
                _ => Err(ImportError::from("클립보드 이미지를 읽지 못했어요")),
            };
            if let Some(editor) = weak.upgrade() {
                editor.finish_import(&at, vec![imported]);
            }
        });
    });

    let formats = gdk::ContentFormats::builder()
        .add_mime_type(crate::app::widgets::ASSET_MIME)
        .add_type(gdk::FileList::static_type())
        .build();
    let drop =
        gtk::DropTargetAsync::new(Some(formats), gdk::DragAction::COPY | gdk::DragAction::MOVE);
    drop.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = Rc::downgrade(editor);
    drop.connect_drag_motion(move |_, _, _, y| {
        let Some(editor) = weak.upgrade() else {
            return gdk::DragAction::empty();
        };
        if !editor.show_drop_line(y) {
            return gdk::DragAction::empty();
        }
        if editor.media.moving.get().is_some() {
            gdk::DragAction::MOVE
        } else {
            gdk::DragAction::COPY
        }
    });
    let weak = Rc::downgrade(editor);
    drop.connect_drag_leave(move |_, _| {
        if let Some(editor) = weak.upgrade() {
            editor.hide_drop_line();
        }
    });
    let weak = Rc::downgrade(editor);
    drop.connect_drop(move |_, drop, _, y| {
        let Some(editor) = weak.upgrade() else {
            return false;
        };
        editor.hide_drop_line();
        let Some(view) = editor.view.upgrade().filter(|view| view.is_editable()) else {
            return false;
        };
        let target = drop_target(&view, y);
        // 같은 메모 안에서 끈 이미지는 줄만 옮긴다. 다른 메모로 끌면 복사다.
        if let Some(from) = editor.media.moving.take() {
            editor.move_image(from, &target);
            drop.finish(gdk::DragAction::MOVE);
            return true;
        }
        let buffer = view.buffer();
        let at = buffer.create_mark(None, &target, true);
        let store = editor.media.host.as_ref().unwrap().store.clone();
        let weak = weak.clone();
        let drop = drop.clone();
        glib::spawn_future_local(async move {
            let imported = read_drop(&drop, store).await;
            drop.finish(if imported.is_empty() {
                gdk::DragAction::empty()
            } else {
                gdk::DragAction::COPY
            });
            if let Some(editor) = weak.upgrade() {
                editor.finish_import(&at, imported);
            }
        });
        true
    });
    view.add_controller(drop);
}

/// 놓을 줄의 시작. 가로 위치는 보지 않아 줄 끝 너머나 들여쓴 빈 곳에 놓아도 그 줄이다.
fn drop_target(view: &TextView, y: f64) -> gtk::TextIter {
    let (_, by) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, 0, y as i32);
    view.line_at_y(by).0
}

/// 파일 화면의 첨부는 해시만 받고, 파일 관리자의 파일은 복사해 넣는다.
async fn read_drop(drop: &gdk::Drop, store: AssetStore) -> Vec<Result<AssetId, ImportError>> {
    let asset = crate::app::widgets::ASSET_MIME;
    if drop.formats().contain_mime_type(asset) {
        let Ok((stream, _)) = drop.read_future(&[asset], glib::Priority::DEFAULT).await else {
            return Vec::new();
        };
        let Ok(bytes) = stream.read_bytes_future(64, glib::Priority::DEFAULT).await else {
            return Vec::new();
        };
        let id = std::str::from_utf8(&bytes)
            .ok()
            .and_then(AssetId::parse)
            .filter(|id| store.path(id).is_some())
            .ok_or_else(|| ImportError::from("파일을 찾을 수 없어요"));
        return vec![id];
    }
    let Ok(value) = drop
        .read_value_future(gdk::FileList::static_type(), glib::Priority::DEFAULT)
        .await
    else {
        return Vec::new();
    };
    let Ok(files) = value.get::<gdk::FileList>() else {
        return Vec::new();
    };
    let paths: Vec<_> = files
        .files()
        .iter()
        .filter_map(|file| file.path())
        .collect();
    gtk::gio::spawn_blocking(move || {
        paths
            .iter()
            .map(|path| store.import_file(path))
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default()
}

/// 넣을 자리. 빈 줄이면 그 줄을 쓰고, 아니면 줄 끝 뒤에 새 줄을 만든다.
pub(super) struct Placement {
    pub(super) blank: bool,
    /// 목록 항목 안이면 그 항목 본문 들여쓰기. 참조 줄이 항목의 이어쓰기 줄이 되어 목록이 끊기지 않는다.
    indent: String,
}

impl LiveEditor {
    /// 받은 이미지를 표시 위치에 넣는다. 실패한 것은 이유를 알리고 넣지 않는다.
    pub(super) fn finish_import(
        &self,
        at: &gtk::TextMark,
        imported: Vec<Result<AssetId, ImportError>>,
    ) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        let mut lines = Vec::new();
        for result in imported {
            match result {
                Ok(id) => lines.push(markdown::Image::markup("", None, &id.target())),
                Err(error) => self.notice(&error.to_string()),
            }
        }
        if !lines.is_empty() && view.is_editable() {
            let at = buffer.iter_at_mark(at);
            let placement = self.placement(&buffer, &at);
            buffer.begin_user_action();
            insert_lines(&buffer, at, &placement, &lines, true);
            buffer.end_user_action();
            view.grab_focus();
        }
        buffer.delete_mark(at);
    }

    pub(super) fn placement(&self, buffer: &TextBuffer, at: &gtk::TextIter) -> Placement {
        let (mut start, mut end) = (*at, *at);
        start.set_line_offset(0);
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        let blank = super::plain_text(buffer, &start, &end).trim().is_empty();
        let document = self.document.borrow();
        let indent = document
            .item_at(end.offset())
            .filter(|_| !blank)
            .map(|(_, item)| {
                super::super::super::source_slice(buffer, item.marker.clone())
                    .chars()
                    .map(|c| if c == '\t' { "    " } else { " " })
                    .collect()
            })
            .unwrap_or_default();
        Placement { blank, indent }
    }

    /// 같은 메모 안에서 이미지 줄을 옮긴다. 지우기와 넣기를 한 번의 undo로 묶는다.
    pub(super) fn move_image(&self, from: i32, to: &gtk::TextIter) {
        let Some(view) = self.view.upgrade().filter(|view| view.is_editable()) else {
            return;
        };
        let Some(image) = self
            .document
            .borrow()
            .images
            .iter()
            .find(|image| image.line.start == from)
            .cloned()
        else {
            return;
        };
        if (image.line.start..=image.line.end).contains(&to.offset()) {
            return;
        }
        let buffer = view.buffer();
        let placement = self.placement(&buffer, to);
        let mark = buffer.create_mark(None, to, true);
        buffer.begin_user_action();
        delete_line(&buffer, image.line.clone());
        let line = markdown::Image::markup(&image.alt, image.width, &image.target);
        insert_lines(
            &buffer,
            buffer.iter_at_mark(&mark),
            &placement,
            &[line],
            false,
        );
        buffer.end_user_action();
        buffer.delete_mark(&mark);
    }
}

/// 참조 줄을 넣는다. `trailing`이면 목록 밖에서 이어 쓸 빈 줄을 하나 둔다.
pub(super) fn insert_lines(
    buffer: &TextBuffer,
    at: gtk::TextIter,
    placement: &Placement,
    lines: &[String],
    trailing: bool,
) {
    let mut iter = at;
    if placement.blank {
        iter.set_line_offset(0);
    } else {
        if !iter.ends_line() {
            iter.forward_to_line_end();
        }
        buffer.insert(&mut iter, "\n");
    }
    let indented: Vec<_> = lines
        .iter()
        .map(|line| format!("{}{line}", placement.indent))
        .collect();
    buffer.insert(&mut iter, &indented.join("\n"));
    if placement.blank || (trailing && placement.indent.is_empty()) {
        buffer.insert(&mut iter, "\n");
    } else if !iter.is_end() {
        iter.forward_char();
    }
    buffer.place_cursor(&iter);
}

/// 줄바꿈까지 지워 빈 줄을 남기지 않는다. 마지막 줄이면 앞 줄바꿈을 지운다.
pub(super) fn delete_line(buffer: &TextBuffer, line: Range<i32>) {
    let mut start = buffer.iter_at_offset(line.start);
    let mut end = buffer.iter_at_offset(line.end);
    if !end.is_end() {
        end.forward_char();
    } else if start.backward_char() && start.char() != '\n' {
        start.forward_char();
    }
    buffer.delete(&mut start, &mut end);
}
