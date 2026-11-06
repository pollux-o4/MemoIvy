use super::*;

/// 쓰는 흐름: 표식 입력 즉시 렌더, Enter 이어 쓰기, 빈 항목 종료, Backspace, Tab, 번호 밀기, undo.
pub(in crate::app) fn fmt_003_typing_flow() {
    let (window, view, _source, editor) = shown_window("");
    let buffer = view.buffer();
    let controllers = view.observe_controllers();
    let keys = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .unwrap();
    let press = |key: gdk::Key, modifiers: gdk::ModifierType| -> bool {
        let stopped = keys.emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers]);
        drain();
        stopped
    };
    let none = gdk::ModifierType::empty();
    let type_text = |value: &str| {
        buffer.begin_user_action();
        buffer.insert_at_cursor(value);
        buffer.end_user_action();
        drain();
    };
    let caret = || buffer.iter_at_mark(&buffer.get_insert()).offset();
    // `- ` 두 글자로 바로 기호가 되고 본문은 그 뒤에 이어진다.
    type_text("메모\n- ");
    assert_eq!(editor.document.borrow().items.len(), 1);
    assert!(tagged(&buffer, 3, "md-marker"));
    type_text("사과");
    assert!(press(gdk::Key::Return, none));
    assert_eq!(text(&buffer), "메모\n- 사과\n- ");
    type_text("우유");
    assert!(press(gdk::Key::Tab, none));
    assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유");
    // Shift+Tab moves a nested item's continuation and descendants with it.
    buffer.set_text("- 상위\n  - 부모\n    이어쓰기\n    - 자식\n      자식 이어쓰기\n- 형제");
    buffer.place_cursor(&buffer.iter_at_offset("- 상위\n  - 부모".chars().count() as i32));
    drain();
    assert!(press(gdk::Key::ISO_Left_Tab, gdk::ModifierType::SHIFT_MASK));
    assert_eq!(
        text(&buffer),
        "- 상위\n- 부모\n  이어쓰기\n  - 자식\n    자식 이어쓰기\n- 형제"
    );
    buffer.undo();
    drain();
    assert_eq!(
        text(&buffer),
        "- 상위\n  - 부모\n    이어쓰기\n    - 자식\n      자식 이어쓰기\n- 형제"
    );
    buffer.set_text("메모\n- 사과\n  - 우유");
    buffer.place_cursor(&buffer.end_iter());
    drain();
    assert!(press(gdk::Key::Return, none));
    assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유\n  - ");
    // 빈 항목: 한 단계 내어 쓰고, 다시 누르면 목록이 끝난다.
    assert!(press(gdk::Key::Return, none));
    assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유\n- ");
    assert!(press(gdk::Key::Return, none));
    assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유\n");
    // 체크: 완료 항목 뒤 새 항목은 비어 있다. 한 번의 undo로 되돌린다.
    buffer.set_text("- [x] 끝남");
    buffer.place_cursor(&buffer.end_iter());
    drain();
    assert!(press(gdk::Key::Return, none));
    assert_eq!(text(&buffer), "- [x] 끝남\n- [ ] ");
    assert_eq!(editor.document.borrow().tasks.len(), 2);
    buffer.undo();
    drain();
    assert_eq!(text(&buffer), "- [x] 끝남");
    // Shift+Enter keeps a task continuation at the task text column, after `- [ ] `.
    buffer.set_text("- [ ] 본문");
    buffer.place_cursor(&buffer.end_iter());
    drain();
    assert!(press(gdk::Key::Return, gdk::ModifierType::SHIFT_MASK));
    assert_eq!(text(&buffer), "- [ ] 본문\n      ");
    assert_eq!(
        editor.document.borrow().items[0].content.end,
        buffer.end_iter().offset()
    );
    // 번호: 사이에 끼우면 뒤 번호가 밀리고 undo 한 번으로 복구된다.
    buffer.set_text("1. 가\n2. 나\n  - 하위\n3. 다\n\n1. 별개");
    buffer.place_cursor(&buffer.iter_at_offset(4));
    drain();
    assert!(press(gdk::Key::Return, none));
    assert_eq!(
        text(&buffer),
        "1. 가\n2. \n3. 나\n  - 하위\n4. 다\n\n1. 별개"
    );
    buffer.undo();
    drain();
    assert_eq!(text(&buffer), "1. 가\n2. 나\n  - 하위\n3. 다\n\n1. 별개");
    // Backspace: 본문 맨 앞에서 표식을 지운다. 중간에서는 평소대로 지운다.
    buffer.set_text("앞\n- [ ] 본문");
    buffer.place_cursor(&buffer.iter_at_offset(8));
    drain();
    assert_eq!(caret(), 8);
    assert!(press(gdk::Key::BackSpace, none));
    assert_eq!(text(&buffer), "앞\n본문");
    buffer.undo();
    drain();
    buffer.place_cursor(&buffer.iter_at_offset(9));
    drain();
    assert!(!press(gdk::Key::BackSpace, none));
    // 왼쪽 화살표는 표식을 건너 앞 줄 끝으로 간다.
    buffer.place_cursor(&buffer.iter_at_offset(8));
    drain();
    assert!(press(gdk::Key::Left, none));
    drain();
    assert_eq!(caret(), 1);
    // 앞 줄 끝의 Delete는 표식까지 지워 본문만 잇는다.
    assert!(press(gdk::Key::Delete, none));
    assert_eq!(text(&buffer), "앞본문");
    // 항목이 아닌 줄의 Enter·Tab은 평소대로다.
    buffer.set_text("평문");
    buffer.place_cursor(&buffer.end_iter());
    drain();
    assert!(!press(gdk::Key::Return, none));
    assert!(!press(gdk::Key::Tab, none));
    // 조합 중 Enter는 입력기에 맡기고 확정 뒤 표식을 채운다.
    buffer.set_text("- 항목");
    buffer.place_cursor(&buffer.end_iter());
    drain();
    view.emit_preedit_changed("한");
    assert!(!press(gdk::Key::Return, none));
    buffer.insert_at_cursor("\n");
    view.emit_preedit_changed("");
    drain();
    assert_eq!(text(&buffer), "- 항목\n- ");
    window.destroy();
    drain();
}
