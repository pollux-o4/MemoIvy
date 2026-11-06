//! FMT-004 위젯 시나리오. 실제 파일 저장소와 텍스처 읽기를 거친다.
use super::*;
use crate::app::test_support::until;
use crate::persistence::Attachments;

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
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

pub(in crate::app) fn fmt_004_image_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let store = Attachments::beside(&dir.path().join("notes.json")).store();
    let id = store.import(&png(400, 200)).unwrap();
    let notices = Rc::new(RefCell::new(Vec::<String>::new()));
    let seen = notices.clone();
    let view = TextView::new();
    configure(&view);
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
    assert_eq!(block().root.height_request(), h);
    assert_eq!(block().spacing.pixels_above_lines(), TOP + h + GAP);
    assert!(block().root.accessible_role() != gtk::AccessibleRole::None);

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
