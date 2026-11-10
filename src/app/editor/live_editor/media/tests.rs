//! FMT-004 위젯 시나리오. 실제 파일 저장소와 텍스처 읽기를 거친다.
use super::*;
use crate::app::test_support::until;
use crate::persistence::Attachments;
use crate::{app::shell::state::AppState, persistence::Note};
use std::path::Path;
use uuid::Uuid;

fn png(width: i32, height: i32) -> Vec<u8> {
    let pixels = vec![200u8; (width * height * 3) as usize];
    let texture = gdk::MemoryTexture::new(
        width,
        height,
        gdk::MemoryFormat::R8g8b8,
        &glib::Bytes::from(&pixels),
        (width * 3) as usize,
    );
    texture.save_to_png_bytes().to_vec()
}

fn text(buffer: &TextBuffer) -> String {
    plain_text(buffer, &buffer.start_iter(), &buffer.end_iter())
}

fn saved_text(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    let serialized = String::from_utf8(bytes).unwrap();
    assert!(
        !serialized.contains('\u{fffc}'),
        "saved JSON contains U+FFFC"
    );
    serde_json::from_str::<serde_json::Value>(&serialized).unwrap()[0]["text"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn open_note(
    path: &Path,
    id: Uuid,
    source: &str,
) -> (gtk::Application, crate::app::shell::state::State, TextView) {
    let app = gtk::Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.ImageAnchorTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let state = AppState::fixture(
        path.to_owned(),
        &app,
        vec![serde_json::from_value::<Note>(serde_json::json!({
            "id": id,
            "text": source,
            "color": "yellow",
            "modified": 1
        }))
        .unwrap()],
    );
    crate::app::shell::commands::open_note(&state, &app, id);
    let view = state.borrow().views.windows[&id].view.clone();
    until(|| {
        if !view.is_mapped() {
            return false;
        }
        let mut iter = view.buffer().start_iter();
        loop {
            if iter.child_anchor().is_some() {
                return true;
            }
            if !iter.forward_char() {
                return false;
            }
        }
    });
    (app, state, view)
}

pub(in crate::app) fn fmt_004_image_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let store = Attachments::beside(&dir.path().join("notes.json")).store();
    let id = store.import(&png(400, 200)).unwrap();
    let notices = Rc::new(RefCell::new(Vec::<String>::new()));
    let seen = notices.clone();
    let view = new_view();
    view.buffer().set_text("제목");
    let scroll = gtk::ScrolledWindow::builder().child(&view).build();
    let (source, editor) = attach_editor(
        &view,
        Some(MediaHost {
            store: store.clone(),
            notice: Box::new(move |text| seen.borrow_mut().push(text.to_owned())),
        }),
    );
    let window = gtk::Window::builder()
        .default_width(420)
        .default_height(420)
        .child(&scroll)
        .build();
    window.present();
    until(|| view.is_mapped() && view.width() > 0 && !editor.refresh_pending());
    let buffer = view.buffer();

    // 넣기: 글이 있는 줄 다음에 넣고 이어 쓸 빈 줄을 둔다. 한 번의 undo로 되돌린다.
    let reference = format!("![](asset:{id})");
    let end = buffer.create_mark(None, &buffer.end_iter(), true);
    editor.finish_import(&end, vec![Ok(id.clone())]);
    assert_eq!(text(&buffer), format!("제목\n{reference}\n"));
    assert_eq!(buffer.iter_at_mark(&buffer.get_insert()).line(), 2);
    buffer.undo();
    assert_eq!(text(&buffer), "제목");
    buffer.redo();

    // 표시: 텍스처를 화면 밖에서 읽은 뒤 본문 폭 안에서 비율대로 그리고, 그 높이만큼 줄 위를 비운다.
    let block = || editor.media.blocks.borrow()[0].clone();
    until(|| {
        !editor.media.blocks.borrow().is_empty()
            && block().picture.paintable().is_some()
            && block().root.is_visible()
    });
    let available = view.width() - view.left_margin() - view.right_margin();
    let (w, h) = display_size(None, (400, 200), available);
    until(|| block().root.width_request() == w);
    assert_eq!(block().root.height_request(), h + GAP);
    assert_eq!(block().spacing.pixels_above_lines(), TOP);
    assert!(block().root.accessible_role() != gtk::AccessibleRole::None);

    // The rendered image must be the pointer target, not the underlying text selection.
    until(|| block().picture.width() > 0 && block().picture.height() > 0);
    let bounds = block().picture.compute_bounds(&view).unwrap();
    let picked = view
        .pick(
            f64::from(bounds.x() + bounds.width() / 2.),
            f64::from(bounds.y() + bounds.height() / 2.),
            gtk::PickFlags::DEFAULT,
        )
        .unwrap();
    assert!(
        picked == block().picture || picked.is_ancestor(&block().root),
        "visible image must receive pointer events, picked {}",
        picked.type_().name()
    );

    // 크기 조절은 놓을 때 원문에 한 번 기록하고, 원래 크기로 되돌릴 수 있다.
    let line = block().line().unwrap().start;
    editor.set_image_width(line, Some(120));
    assert_eq!(text(&buffer), format!("제목\n![|120](asset:{id})\n"));
    until(|| block().root.width_request() == 120);
    buffer.undo();
    assert_eq!(text(&buffer), format!("제목\n{reference}\n"));
    editor.refresh();

    // 원문 보기에서는 이미지 대신 문법을 보인다.
    source.set_active(true);
    until(|| !block().root.is_visible());
    source.set_active(false);
    until(|| block().root.is_visible());

    // 메모에서 빼기는 줄만 지우고 파일은 남긴다.
    editor.remove_image(line);
    assert_eq!(text(&buffer), "제목\n");
    assert!(store.path(&id).is_some());
    buffer.undo();
    assert_eq!(text(&buffer), format!("제목\n{reference}\n"));

    // 찾을 수 없는 파일은 원문을 지우지 않고 자리 표시를 그린다.
    let missing = format!("![](asset:{})", "0".repeat(64));
    buffer.set_text(&missing);
    until(|| {
        block().placeholder.is_visible() && block().placeholder.text() == "이미지를 찾을 수 없음"
    });
    assert_eq!(text(&buffer), missing);

    // 거부된 넣기는 이유만 알리고 본문을 바꾸지 않는다.
    let mark = buffer.create_mark(None, &buffer.end_iter(), true);
    editor.finish_import(&mark, vec![store.import(b"not an image")]);
    assert_eq!(text(&buffer), missing);
    assert_eq!(notices.borrow().len(), 1);

    // 목록 항목에 놓으면 그 항목의 이어쓰기 줄이 되어 목록이 끊기지 않고, 항목 본문에 맞춰 그린다.
    buffer.set_text("- 항목\n  - 하위\n- 다음");
    editor.refresh();
    let item_end = buffer.iter_at_line(1).unwrap();
    let mark = buffer.create_mark(None, &item_end, true);
    editor.finish_import(&mark, vec![Ok(id.clone())]);
    assert_eq!(
        text(&buffer),
        format!("- 항목\n  - 하위\n    {reference}\n- 다음")
    );
    editor.refresh();
    let items: Vec<_> = editor
        .document
        .borrow()
        .items
        .iter()
        .map(|i| i.depth)
        .collect();
    assert_eq!(items, [0, 1, 0], "목록 구조가 유지된다");
    until(|| block().x.get() == view.left_margin() + GUTTER * 2);
    until(|| {
        block()
            .root
            .compute_point(&view, &gtk::graphene::Point::new(0., 0.))
            .is_some_and(|point| {
                view.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    point.x().round() as i32,
                    point.y().round() as i32,
                )
                .0 == block().x.get()
            })
    });
    let root = block()
        .root
        .compute_point(&view, &gtk::graphene::Point::new(0.0, 0.0))
        .unwrap();
    let (actual_x, _) = view.window_to_buffer_coords(
        gtk::TextWindowType::Widget,
        root.x().round() as i32,
        root.y().round() as i32,
    );
    assert_eq!(actual_x, block().x.get());

    // 같은 메모 안에서 끌어 옮기면 줄만 옮기고 한 번의 undo로 되돌린다.
    let original = format!("첫째\n{reference}\n둘째\n셋째");
    buffer.set_text(&original);
    editor.refresh();
    let line = editor.document.borrow().images[0].line.start;
    editor.move_image(line, &buffer.iter_at_line(3).unwrap());
    assert_eq!(text(&buffer), format!("첫째\n둘째\n셋째\n{reference}"));
    buffer.undo();
    assert_eq!(text(&buffer), original);
    editor.refresh();
    editor.move_image(line, &buffer.iter_at_line(1).unwrap());
    assert_eq!(text(&buffer), original, "자기 줄에 놓으면 그대로다");
    assert!(editor.show_drop_line(5.0));
    editor.hide_drop_line();
    window.destroy();
}

pub(in crate::app) fn fmt_004_image_overlay_tracks_reserved_line_each_frame() {
    let dir = tempfile::tempdir().unwrap();
    let store = Attachments::beside(&dir.path().join("notes.json")).store();
    let id = store.import(&png(400, 200)).unwrap();
    let view = new_view();
    view.buffer()
        .set_text(&format!("- [ ] 마일스톤 확인했음\n![](asset:{id})\n"));
    let mismatches = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::new(Cell::new(0));
    let scroll = gtk::ScrolledWindow::builder().child(&view).build();
    let (_, editor) = attach_editor(
        &view,
        Some(MediaHost {
            store,
            notice: Box::new(|_| {}),
        }),
    );
    let window = gtk::Window::builder()
        .default_width(420)
        .default_height(420)
        .child(&scroll)
        .build();
    window.present();
    until(|| {
        view.is_mapped()
            && view.width() > 0
            && !editor.refresh_pending()
            && editor
                .media
                .blocks
                .borrow()
                .first()
                .is_some_and(|block| block.root.is_visible() && block.picture.paintable().is_some())
    });
    let block = editor.media.blocks.borrow()[0].clone();
    crate::app::test_support::settle(30);
    let bounds = block.picture.compute_bounds(&view).unwrap();
    let picked = view
        .pick(
            f64::from(bounds.x() + bounds.width() / 2.),
            f64::from(bounds.y() + bounds.height() / 2.),
            gtk::PickFlags::DEFAULT,
        )
        .unwrap();
    assert!(
        picked == block.picture || picked.is_ancestor(&block.root),
        "image below task overlays must receive pointer events, picked {}",
        picked.type_().name()
    );

    let bad_frames = mismatches.clone();
    let frame_count = observed.clone();
    let frame_view = view.clone();
    view.frame_clock().unwrap().connect_after_paint(move |_| {
        let line = block.line().unwrap().start;
        let (top, _) = frame_view.line_yrange(&frame_view.buffer().iter_at_offset(line));
        let actual_window = block
            .root
            .compute_point(&frame_view, &gtk::graphene::Point::new(0.0, 0.0))
            .unwrap()
            .y()
            .round() as i32;
        let (_, actual) =
            frame_view.window_to_buffer_coords(gtk::TextWindowType::Widget, 0, actual_window);
        let expected = top + TOP;
        if actual != expected {
            bad_frames.borrow_mut().push((actual, expected));
        }
        frame_count.set(frame_count.get() + 1);
    });

    let buffer = view.buffer();
    let mut end = buffer.iter_at_line(0).unwrap();
    end.forward_to_line_end();
    buffer.place_cursor(&end);
    buffer.insert_at_cursor("\n");
    let before = observed.get();
    until(|| observed.get() > before);
    for character in " asdfasdf".chars() {
        buffer.insert_at_cursor(&character.to_string());
        let before = observed.get();
        until(|| observed.get() > before);
    }
    buffer.insert(&mut buffer.end_iter(), &"\n아래 본문".repeat(40));
    let before = observed.get();
    until(|| observed.get() > before);
    view.vadjustment().unwrap().set_value(20.);
    let before = observed.get();
    until(|| observed.get() > before);
    let block = editor.media.blocks.borrow()[0].clone();
    let bounds = block.picture.compute_bounds(&view).unwrap();
    let picked = view
        .pick(
            f64::from(bounds.x() + bounds.width() / 2.),
            f64::from((bounds.y() + bounds.height() / 2.).max(1.)),
            gtk::PickFlags::DEFAULT,
        )
        .unwrap();
    assert!(
        picked == block.picture || picked.is_ancestor(&block.root),
        "scrolled image must retain pointer target"
    );
    assert!(
        observed.get() >= 9,
        "every typed character must cross a frame"
    );
    assert!(
        mismatches.borrow().is_empty(),
        "image must match its reserved y in every painted frame: {:?}",
        mismatches.borrow()
    );
    window.destroy();
}

pub(in crate::app) fn fmt_004_image_anchor_storage_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let id = Uuid::new_v4();
    let reference = "![표](asset:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef)";
    let original = format!("위 줄\n{reference}\n아래 줄\n- 목록\n");
    let (_app, state, view) = open_note(&path, id, &original);
    let buffer = view.buffer();

    // 위 줄 입력과 이미지 줄 이동을 실제 버퍼 편집으로 수행한다.
    let mut first_end = buffer.iter_at_line(0).unwrap();
    first_end.forward_to_line_end();
    buffer.place_cursor(&first_end);
    buffer.insert_at_cursor(" 입력");
    let image_source = "위 줄 입력\n".chars().count() as i32;
    let image_offset = anchors::AnchorMap::read(&buffer).to_buffer_before_anchor(image_source);
    let mut image_start = buffer.iter_at_offset(image_offset);
    let mut image_end = image_start;
    image_end.forward_to_line_end();
    image_end.forward_char();
    let moved = plain_text(&buffer, &image_start, &image_end);
    assert_eq!(moved, format!("{reference}\n"));
    buffer.begin_user_action();
    buffer.delete(&mut image_start, &mut image_end);
    let image_destination = "위 줄 입력\n아래 줄\n".chars().count() as i32;
    let mapped_destination = anchors::AnchorMap::read(&buffer).to_buffer(image_destination);
    buffer.insert(&mut buffer.iter_at_offset(mapped_destination), &moved);
    buffer.end_user_action();
    let after_move = format!("위 줄 입력\n아래 줄\n{reference}\n- 목록\n");
    until(|| text(&buffer) == after_move);

    // 삭제를 undo한 저장본에는 원본 이미지 문법이 그대로 있어야 한다.
    let image_source = "위 줄 입력\n아래 줄\n".chars().count() as i32;
    let image_offset = anchors::AnchorMap::read(&buffer).to_buffer_before_anchor(image_source);
    let mut moved_start = buffer.iter_at_offset(image_offset);
    let mut moved_end = moved_start;
    moved_end.forward_to_line_end();
    moved_end.forward_char();
    buffer.begin_user_action();
    buffer.delete(&mut moved_start, &mut moved_end);
    buffer.end_user_action();
    until(|| text(&buffer) == "위 줄 입력\n아래 줄\n- 목록\n");
    buffer.undo();
    until(|| text(&buffer) == after_move);
    assert!(buffer.can_redo(), "anchor normalization must preserve redo");
    crate::app::shell::lifecycle::flush(&state);
    until(|| {
        matches!(
            state.borrow().notebook.status(),
            crate::persistence::SaveStatus::Saved
        )
    });
    assert_eq!(saved_text(&path), after_move);

    // Redo the deletion and verify the disk snapshot remains free of structural characters.
    buffer.redo();
    until(|| !text(&buffer).contains(reference));
    let after_redo = text(&buffer);
    crate::app::shell::lifecycle::flush(&state);
    until(|| {
        matches!(
            state.borrow().notebook.status(),
            crate::persistence::SaveStatus::Saved
        )
    });
    assert_eq!(saved_text(&path), after_redo);
    assert!(!after_redo.contains(reference));
    state.borrow().views.windows[&id].window.destroy();
}

pub(in crate::app) fn fmt_004_image_anchor_clipboard_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let id = Uuid::new_v4();
    let reference = "![표](asset:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef)";
    let original = format!("위 줄\n{reference}\n아래 줄\n");
    let (_app, state, view) = open_note(&path, id, &original);
    let buffer = view.buffer();

    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.select_range(&start, &end);
    view.emit_by_name::<()>("copy-clipboard", &[]);
    let clipboard = glib::MainContext::default()
        .block_on(view.clipboard().read_text_future())
        .unwrap()
        .unwrap()
        .to_string();
    assert_eq!(clipboard, original);
    assert!(!clipboard.contains('\u{fffc}'));

    view.emit_by_name::<()>("cut-clipboard", &[]);
    until(|| {
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .is_empty()
    });
    view.emit_by_name::<()>("paste-clipboard", &[]);
    until(|| buffer.text(&buffer.start_iter(), &buffer.end_iter(), true) == original);
    assert!(!buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .contains('\u{fffc}'));

    // Backspace at the position after the inline anchor removes only that anchor;
    // the next parse must attach the image widget to a fresh live anchor.
    let mut iter = buffer.start_iter();
    let old_anchor = loop {
        if let Some(anchor) = iter.child_anchor() {
            break anchor;
        }
        assert!(iter.forward_char());
    };
    let anchor_iter = buffer.iter_at_child_anchor(&old_anchor);
    let mut after_anchor = anchor_iter;
    after_anchor.forward_char();
    buffer.place_cursor(&after_anchor);
    view.emit_by_name::<()>("backspace", &[]);
    let refreshed = Rc::new(RefCell::new(None));
    let found = refreshed.clone();
    until(|| {
        let mut scan = buffer.start_iter();
        loop {
            if let Some(anchor) = scan.child_anchor() {
                if anchor != old_anchor {
                    found.replace(Some(anchor));
                    return true;
                }
            }
            if !scan.forward_char() {
                return false;
            }
        }
    });
    let new_anchor = refreshed.borrow().clone().unwrap();
    assert_eq!(
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), true),
        original
    );
    assert!(old_anchor.is_deleted());
    assert!(
        old_anchor.widgets().is_empty(),
        "deleted anchor has no orphan widget"
    );
    assert_eq!(
        new_anchor.widgets().len(),
        1,
        "image is attached to its live anchor"
    );

    crate::app::shell::lifecycle::flush(&state);
    until(|| {
        matches!(
            state.borrow().notebook.status(),
            crate::persistence::SaveStatus::Saved
        )
    });
    assert_eq!(saved_text(&path), original);
    state.borrow().views.windows[&id].window.destroy();
}
