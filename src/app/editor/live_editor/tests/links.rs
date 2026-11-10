use super::*;

pub(in crate::app) fn fmt_003_link_preview_lifecycle() {
    let text = "[이름](https://a.example/b) 그리고 https://c.example\n\n둘째 줄";
    let (window, view, _source, editor) = shown_window(text);
    let buffer = view.buffer();
    buffer.place_cursor(&buffer.end_iter());
    settle(60);
    let link = |i: usize| editor.document.borrow().links[i].clone();
    let anchor = gdk::Rectangle::new(10, 10, 1, 18);
    // 지나가는 포인터에는 뜨지 않는다.
    editor.preview.hover(&link(0), anchor);
    assert!(editor.preview.is_pending());
    editor.preview.pointer_left();
    assert!(!editor.preview.is_pending());
    settle(100);
    assert_eq!(editor.preview.shown_url(), None);
    // 머무르면 뜨고, 이름과 목적지를 함께 보여 준다. 포커스는 편집기에 남는다.
    view.grab_focus();
    editor.preview.hover(&link(0), anchor);
    until(|| editor.preview.is_shown() && editor.preview.card_is_mapped());
    assert_eq!(
        editor.preview.shown_url().as_deref(),
        Some("https://a.example/b")
    );
    assert!(editor.preview.card_is_visible());
    assert_eq!(editor.preview.title(), "이름");
    assert!(view.is_focus(), "preview must not steal typing focus");
    // 전환 대기 중에는 기존 카드를 유지하고, 취소하면 그 카드의 닫기 유예로 돌아간다.
    editor.preview.hover(&link(1), anchor);
    assert!(editor.preview.is_pending());
    assert_eq!(
        editor.preview.shown_url().as_deref(),
        Some("https://a.example/b")
    );
    editor.preview.pointer_left();
    assert!(!editor.preview.is_pending());
    assert!(editor.preview.is_shown());
    // 옆 링크로 옮기면 기다리지 않고 내용을 바꾼다.
    editor.preview.hover(&link(1), anchor);
    until(&|| editor.preview.shown_url().as_deref() == Some("https://c.example"));
    assert_eq!(editor.preview.title(), "c.example");
    // 복사는 카드를 닫지 않고 원본 주소를 넣는다.
    editor.preview.click_copy();
    let copied = glib::MainContext::default()
        .block_on(view.clipboard().read_text_future())
        .unwrap()
        .unwrap();
    assert_eq!(copied.as_str(), "https://c.example");
    assert!(editor.preview.is_shown());
    // 벗어나면 잠시 뒤 닫힌다.
    editor.preview.pointer_left();
    assert!(editor.preview.is_shown(), "grace period to reach the card");
    until(&|| !editor.preview.is_shown());
    until(|| !editor.preview.card_is_visible());
    // Esc는 포인터를 옮기지 않고 닫는다.
    editor.preview.hover(&link(0), anchor);
    until(|| editor.preview.is_shown() && editor.preview.card_is_mapped());
    let controllers = view.observe_controllers();
    let key = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .unwrap();
    let stopped: bool = key.emit_by_name(
        "key-pressed",
        &[&gdk::Key::Escape, &0u32, &gdk::ModifierType::empty()],
    );
    assert!(stopped);
    assert!(!editor.preview.is_shown());
    until(|| !editor.preview.card_is_visible());
    // 주소 수정은 원문의 주소만 선택한다.
    editor.preview.hover(&link(0), anchor);
    until(|| editor.preview.is_shown() && editor.preview.card_is_mapped());
    editor.preview.click_edit();
    until(|| !editor.preview.card_is_visible());
    let (start, end) = buffer.selection_bounds().unwrap();
    assert_eq!(buffer.text(&start, &end, true), "https://a.example/b");
    assert!(!editor.preview.is_shown());
    // 입력하면 예약까지 취소되고 표시 태그는 undo에 들어가지 않는다.
    buffer.place_cursor(&buffer.end_iter());
    editor.preview.hover(&link(0), anchor);
    until(|| editor.preview.is_shown() && editor.preview.card_is_mapped());
    assert!(!buffer.can_undo());
    buffer.insert_at_cursor("a");
    assert!(!editor.preview.is_shown());
    buffer.undo();
    settle(30);
    // 커서가 링크에 머무르면 키보드로도 뜨고, 벗어나면 닫힌다.
    view.grab_focus();
    buffer.place_cursor(&source_iter(&buffer, 2));
    drain();
    editor.sync_caret_preview();
    until(|| editor.preview.is_shown() && editor.preview.card_is_mapped());
    assert_eq!(
        editor.preview.shown_url().as_deref(),
        Some("https://a.example/b")
    );
    buffer.place_cursor(&buffer.end_iter());
    until(&|| !editor.preview.is_shown());
    // Esc로 닫으면 같은 링크 안에서 움직여도 다시 뜨지 않고, 벗어났다 오면 뜬다.
    buffer.place_cursor(&source_iter(&buffer, 2));
    drain();
    editor.sync_caret_preview();
    until(|| editor.preview.is_shown() && editor.preview.card_is_mapped());
    editor.preview.dismiss();
    buffer.place_cursor(&source_iter(&buffer, 3));
    settle(160);
    assert!(!editor.preview.is_shown(), "dismissed stays dismissed");
    buffer.place_cursor(&buffer.end_iter());
    drain();
    // 조합 중에는 뜨지 않는다.
    buffer.place_cursor(&source_iter(&buffer, 2));
    view.emit_preedit_changed("한");
    settle(160);
    assert!(!editor.preview.is_shown());
    view.emit_preedit_changed("");
    // 대기 중 창을 닫아도 타이머·보조 표면이 남지 않는다.
    editor.preview.hover(&link(0), anchor);
    window.destroy();
    settle(100);
}
