use super::*;
use crate::app::test_support::{count, drain, find_checkbox, settle, until};
use std::time::Instant;

fn text(buffer: &TextBuffer) -> String {
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

fn tagged(buffer: &TextBuffer, offset: i32, name: &str) -> bool {
    let tag = buffer.tag_table().lookup(name).unwrap();
    buffer.iter_at_offset(offset).has_tag(&tag)
}

fn shown_window(text: &str) -> (gtk::Window, TextView, CheckButton, Rc<LiveEditor>) {
    let view = TextView::new();
    configure(&view);
    view.buffer().set_text(text);
    // 실제 편집 창과 같은 순서: 스크롤 뷰에 넣은 뒤 붙인다.
    let scroll = gtk::ScrolledWindow::builder().child(&view).build();
    let (source, editor) = attach_editor(&view);
    let window = gtk::Window::builder()
        .default_width(420)
        .default_height(320)
        .child(&scroll)
        .build();
    window.present();
    until(|| {
        view.is_mapped()
            && view.width() > 0
            && view.height() > 0
            && !editor.pending.get()
            && editor.layout.frames.get() == 0
    });
    (window, view, source, editor)
}

/// 숨긴 표식이 있는 줄의 모든 지점을 눌러도 GTK가 중단되지 않아야 한다.
/// 줄 아래 간격을 다시 쓰면 이 검사가 프로세스 중단으로 실패한다.
pub(in crate::app) fn fmt_003_list_layout_and_hit_testing() {
    let text =
        "제목\n- 우유\n  - 저지방\n- [ ] 할 일\n- [x] 끝남\n1. 번호\n- \n**굵게** 끝\n마지막 *줄*";
    let (window, view, source, _editor) = shown_window(text);
    let buffer = view.buffer();
    buffer.place_cursor(&buffer.iter_at_offset(0));
    settle(60);
    for y in 0..view.height() {
        for x in (0..view.width()).step_by(13) {
            let (bx, by) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, x, y);
            let _ = view.iter_at_position(bx, by);
            let _ = view.iter_at_location(bx, by);
        }
    }
    let offset = |needle: &str| text[..text.find(needle).unwrap()].chars().count() as i32;
    // 모든 항목은 표식을 왼쪽 여백으로 내어 쓰고 본문은 표식 칸 뒤에서 시작한다. 커서 줄도 같다.
    let hanging = |at: i32| {
        buffer
            .iter_at_offset(at)
            .tags()
            .into_iter()
            .find(|tag| tag.is_indent_set() && tag.indent() != 0)
            .map(|tag| tag.left_margin() - tag.indent())
    };
    for caret in [0, offset("우유")] {
        buffer.place_cursor(&buffer.iter_at_offset(caret));
        settle(40);
        assert_eq!(hanging(offset("우유")), Some(20 + GUTTER));
        assert_eq!(hanging(offset("저지방")), Some(20 + GUTTER * 2));
        assert_eq!(hanging(offset("할 일")), Some(20 + GUTTER));
        assert_eq!(hanging(offset("번호")), Some(20 + GUTTER));
        assert!(
            tagged(&buffer, offset("- 우유"), "md-marker"),
            "caret line stays rendered"
        );
    }
    assert!(tagged(&buffer, offset("끝남"), "md-task-done"));
    assert!(!tagged(&buffer, offset("할 일"), "md-task-done"));
    // 번호는 글자 그대로 쓰고, 빈 항목도 줄이 접히지 않은 채 표식이 그려진다.
    assert!(!tagged(&buffer, offset("1. "), "md-marker"));
    assert!(tagged(&buffer, offset("1. "), "md-syntax"));
    assert!(tagged(&buffer, offset("- \n"), "md-marker"));
    assert!(!tagged(&buffer, offset("- \n"), "md-hidden"));
    // 선택해도 표식을 유지한다. 복사는 원문이다.
    buffer.select_range(
        &buffer.iter_at_offset(offset("우유")),
        &buffer.iter_at_offset(offset("우유") + 1),
    );
    settle(30);
    assert!(tagged(&buffer, offset("- 우유"), "md-marker"));
    // 표식 안의 커서는 본문 앞으로 옮긴다.
    buffer.place_cursor(&buffer.iter_at_offset(offset("- [ ] 할 일")));
    settle(30);
    assert_eq!(
        buffer.iter_at_mark(&buffer.get_insert()).offset(),
        offset("할 일")
    );
    // 긴 목록을 스크롤해도 화면 안 항목의 표식이 남아 있다.
    let long = "- [ ] 항목\n- 기호\n".repeat(200);
    buffer.set_text(&long);
    buffer.place_cursor(&buffer.start_iter());
    settle(120);
    let adjustment = view.vadjustment().unwrap();
    adjustment.set_value(adjustment.upper() / 2.);
    settle(120);
    assert_eq!(count(view.upcast_ref(), "md-checkbox"), 200);
    assert_eq!(count(view.upcast_ref(), "md-bullet"), 200);
    buffer.set_text(text);
    settle(30);
    // 원문 보기에는 어떤 표시 태그도 남지 않는다.
    source.set_active(true);
    settle(20);
    assert!(!tagged(&buffer, offset("끝남"), "md-task-done"));
    assert!(buffer
        .iter_at_offset(offset("끝남"))
        .tags()
        .iter()
        .all(|t| !t.is_left_margin_set()));
    window.destroy();
    drain();
}

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
    settle(450);
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
    buffer.place_cursor(&buffer.iter_at_offset(2));
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
    buffer.place_cursor(&buffer.iter_at_offset(2));
    drain();
    editor.sync_caret_preview();
    until(|| editor.preview.is_shown() && editor.preview.card_is_mapped());
    editor.preview.dismiss();
    buffer.place_cursor(&buffer.iter_at_offset(3));
    settle(750);
    assert!(!editor.preview.is_shown(), "dismissed stays dismissed");
    buffer.place_cursor(&buffer.end_iter());
    drain();
    // 조합 중에는 뜨지 않는다.
    buffer.place_cursor(&buffer.iter_at_offset(2));
    view.emit_preedit_changed("한");
    settle(750);
    assert!(!editor.preview.is_shown());
    view.emit_preedit_changed("");
    // 대기 중 창을 닫아도 타이머·보조 표면이 남지 않는다.
    editor.preview.hover(&link(0), anchor);
    window.destroy();
    settle(450);
}

pub(in crate::app) fn fmt_001_live_editor_preserves_source_selection_undo_and_preedit() {
    let view = TextView::new();
    let buffer = view.buffer();
    let original =
        "# 제목😀\n**굵게** *기울임* ~~취소~~\n- [ ] 할 일\n[링크](https://example.com)\n\n";
    buffer.set_text(original);
    let source = attach(&view);
    buffer.place_cursor(&buffer.end_iter());
    drain();
    assert!(
        tagged(&buffer, 0, "md-hidden"),
        "inactive heading syntax must hide"
    );
    let bold = original[..original.find("**").unwrap()].chars().count() as i32;
    assert!(tagged(&buffer, bold, "md-hidden"));
    buffer.place_cursor(&buffer.iter_at_offset(bold + 3));
    drain();
    assert!(
        !tagged(&buffer, bold, "md-hidden"),
        "caret line reveals syntax"
    );
    assert!(
        tagged(&buffer, 0, "md-hidden"),
        "do not reveal neighboring line"
    );
    assert_eq!(text(&buffer), original);
    assert!(
        !buffer.can_undo(),
        "display updates must not enter undo history"
    );
    let checkbox = find_checkbox(view.upcast_ref()).unwrap();
    buffer.place_cursor(&buffer.end_iter());
    drain();
    assert!(
        checkbox.is_visible(),
        "inactive tasks have native checkboxes"
    );
    checkbox.set_active(true);
    drain();
    assert_eq!(text(&buffer), original.replace("[ ]", "[x]"));
    buffer.undo();
    drain();
    assert_eq!(text(&buffer), original);

    buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
    // Clipboard actions can arrive before the queued visibility update.
    view.emit_by_name::<()>("copy-clipboard", &[]);
    let copied = glib::MainContext::default()
        .block_on(view.clipboard().read_text_future())
        .unwrap()
        .unwrap();
    assert_eq!(copied.as_str(), original);
    view.emit_by_name::<()>("cut-clipboard", &[]);
    drain();
    assert_eq!(text(&buffer), "");
    assert!(
        !checkbox.is_visible(),
        "removed tasks leave no stale overlays"
    );
    buffer.undo();
    drain();
    assert_eq!(text(&buffer), original, "cut is one undo action");
    buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
    drain();
    assert!(
        !tagged(&buffer, 0, "md-hidden"),
        "selection must include raw syntax"
    );
    let selection = buffer.selection_bounds().unwrap();
    source.set_active(true);
    drain();
    assert_eq!(
        buffer.selection_bounds().unwrap().0.offset(),
        selection.0.offset()
    );
    assert_eq!(
        buffer.selection_bounds().unwrap().1.offset(),
        selection.1.offset()
    );
    assert!(!tagged(&buffer, 0, "md-h1"));
    source.set_active(false);
    buffer.place_cursor(&buffer.end_iter());
    drain();
    view.emit_preedit_changed("한");
    buffer.place_cursor(&buffer.start_iter());
    drain();
    assert!(
        tagged(&buffer, 0, "md-hidden"),
        "composition freezes layout"
    );
    view.emit_preedit_changed("");
    drain();
    assert!(!tagged(&buffer, 0, "md-hidden"));
    buffer.place_cursor(&buffer.end_iter());
    buffer.begin_user_action();
    buffer.insert_at_cursor("입력📝");
    buffer.end_user_action();
    drain();
    buffer.undo();
    drain();
    assert_eq!(text(&buffer), original);
    buffer.redo();
    drain();
    assert_eq!(text(&buffer), format!("{original}입력📝"));
    buffer.undo();
    drain();

    // Run the same task mutation used by the keyboard/click handlers through a captured key signal.
    let task = original[..original.find("[ ]").unwrap()].chars().count() as i32;
    buffer.place_cursor(&buffer.iter_at_offset(task));
    drain();
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
        &[&gdk::Key::Return, &0u32, &gdk::ModifierType::CONTROL_MASK],
    );
    assert!(stopped);
    drain();
    assert_eq!(text(&buffer), original.replace("[ ]", "[x]"));
    buffer.undo();
    drain();
    assert_eq!(
        text(&buffer),
        original,
        "one undo restores a checkbox toggle"
    );
    view.set_editable(false);
    drain();
    assert!(!checkbox.is_sensitive());
    let stopped: bool = key.emit_by_name(
        "key-pressed",
        &[&gdk::Key::Return, &0u32, &gdk::ModifierType::CONTROL_MASK],
    );
    assert!(!stopped);
    assert_eq!(text(&buffer), original);
    view.set_editable(true);

    buffer.set_text("한글😀");
    buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
    assert!(super::super::formatting::handle(
        &view,
        gdk::Key::b,
        gdk::ModifierType::CONTROL_MASK
    ));
    drain();
    assert_eq!(text(&buffer), "**한글😀**");
    assert!(super::super::formatting::handle(
        &view,
        gdk::Key::b,
        gdk::ModifierType::CONTROL_MASK
    ));
    drain();
    assert_eq!(
        text(&buffer),
        "한글😀",
        "repeat shortcut removes formatting"
    );
    buffer.undo();
    drain();
    assert_eq!(text(&buffer), "**한글😀**");

    // PERF-001-sized real widget path; report the bounded synthetic input timings.
    buffer.set_text(&"**한글😀** [링크](https://example.com)\n".repeat(230));
    drain();
    let mut samples = Vec::new();
    for _ in 0..100 {
        let started = Instant::now();
        buffer.insert_at_cursor("a");
        drain();
        samples.push(started.elapsed());
    }
    samples.sort();
    println!(
        "FMT-001 widget input+idle p95={:.3}ms",
        samples[94].as_secs_f64() * 1000.
    );
    assert!(samples[94] < std::time::Duration::from_millis(50));
}
