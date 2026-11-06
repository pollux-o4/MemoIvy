use super::*;

pub(in crate::app) fn fmt_003_list_layout_and_hit_testing() {
    let text = "제목\n- 우유\n  - 저지방\n- [ ] 할 일\n- [x] 끝남\n      결과: 이어쓰기\n1. 번호\n- \n**굵게** 끝\n마지막 *줄*";
    let (window, view, source, editor) = shown_window(text);
    let buffer = view.buffer();
    // 입력 직후 첫 그림부터 본문이 제자리에 있어야 한다. 다음 프레임 보정에 기대지 않는다.
    // 폰트마다 측정 폭이 실제와 어긋날 수 있다. 어긋난 측정값에서 시작해도 첫 그림이 맞아야 한다.
    for typed in ["- 첫", "- [ ] 첫", "- 상위\n  - 첫"] {
        editor.layout.hang_fix.borrow_mut().clear();
        for marker in ["- ", "- [ ] ", "  - "] {
            editor.layout.hang_fix.borrow_mut().insert(marker.into(), 3);
        }
        buffer.set_text(&format!("{text}\n{typed}"));
        buffer.place_cursor(&buffer.end_iter());
        editor.refresh();
        let item = buffer.end_iter().offset() - 1;
        let depth = i32::from(typed.contains("\n  "));
        assert_eq!(
            view.iter_location(&buffer.iter_at_offset(item)).x(),
            20 + GUTTER * (depth + 1),
            "{typed:?} content must start aligned without waiting for a frame"
        );
    }
    // 빈 항목에 한글을 조합·확정·지우는 동안 본문이 1px도 움직이지 않는다.
    // 아랫줄도 위아래로 흔들리지 않는다. 빈 줄과 한글 줄의 높이가 같아야 한다.
    // 한글이 없는 기본 글꼴이어야 빈 줄과 한글 줄의 높이 차이가 드러난다.
    let settings = gtk::Settings::default().unwrap();
    let font = settings.gtk_font_name();
    settings.set_gtk_font_name(Some("Ubuntu Sans 11"));
    editor.layout.hang_fix.borrow_mut().clear();
    buffer.set_text("* \n아래");
    buffer.place_cursor(&buffer.iter_at_offset(2));
    editor.refresh();
    settle(30);
    editor.refresh();
    settle(30);
    let below = || view.iter_location(&buffer.end_iter()).y();
    let below_empty = below();
    let bullet = || {
        let label = editor.widgets.bullets.borrow()[0].clone();
        let point = label.compute_point(&view, &gtk::graphene::Point::new(0.0, 0.0));
        (point.map(|p| p.y() as i32), label.height_request())
    };
    let bullet_empty = bullet();
    let still = |step: &str| {
        assert_eq!(
            view.iter_location(&buffer.iter_at_offset(2)).x(),
            20 + GUTTER,
            "content moved at {step}"
        );
        assert_eq!(below(), below_empty, "next line moved vertically at {step}");
        assert_eq!(bullet(), bullet_empty, "bullet moved vertically at {step}");
    };
    still("empty");
    // 한글 입력기: 자음이 이어지면 앞 글자를 확정하고 새 글자를 조합한다.
    view.emit_preedit_changed("ㅁ");
    drain();
    still("preedit");
    for (commit, next) in [("ㅁ", "ㄴ"), ("ㄴ", "ㅇ")] {
        buffer.insert_at_cursor(commit);
        view.emit_preedit_changed(next);
        drain();
        still(commit);
    }
    view.emit_preedit_changed("");
    drain();
    for _ in 0..2 {
        let mut end = buffer.iter_at_mark(&buffer.get_insert());
        let mut start = end;
        start.backward_char();
        buffer.delete(&mut start, &mut end);
        drain();
        still("delete");
    }
    // 조합 중에도 세 단계 목록의 표식·본문 위계와 폭 보정은 움직이지 않는다.
    buffer.set_text("- 실마리\n  - 에드센스 보완중 (2)\n    - \n  - 블로그\n  - 내용 보완 (1)");
    editor.layout.hang_fix.borrow_mut().clear();
    editor.refresh();
    settle(30);
    let content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
    let line_start = content.find("    - \n").unwrap();
    let empty = content[..line_start].chars().count() as i32 + 6;
    buffer.place_cursor(&buffer.iter_at_offset(empty));
    editor.refresh();
    let expected_x = 20 + GUTTER * 3;
    let hang_fix = editor.layout.hang_fix.borrow().clone();
    assert_eq!(
        view.iter_location(&buffer.iter_at_offset(empty)).x(),
        expected_x
    );
    view.emit_preedit_changed("ㄱ");
    drain();
    assert_eq!(
        view.iter_location(&buffer.iter_at_offset(empty)).x(),
        expected_x
    );
    assert_eq!(*editor.layout.hang_fix.borrow(), hang_fix);
    view.emit_preedit_changed("");
    drain();
    // 선택한 체크 항목 문장을 IME 조합으로 교체해도 체크박스와 들여쓰기는 고정한다.
    buffer.set_text("- 바깥\n  - [ ] 시작 안함");
    editor.refresh();
    settle(30);
    let content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
    let start = content.find("시작 안함").unwrap();
    let start = content[..start].chars().count() as i32;
    buffer.select_range(
        &buffer.iter_at_offset(buffer.end_iter().offset()),
        &buffer.iter_at_offset(start),
    );
    editor.refresh();
    let body_x = view.iter_location(&buffer.iter_at_offset(start)).x();
    assert!(editor.widgets.tasks.borrow()[0].button.is_visible());
    view.emit_preedit_changed("작");
    buffer.insert_at_cursor("작");
    drain();
    assert_eq!(editor.document.borrow().items.len(), 2);
    assert!(editor.widgets.tasks.borrow()[0].button.is_visible());
    assert_eq!(
        view.iter_location(&buffer.iter_at_offset(start)).x(),
        body_x
    );
    view.emit_preedit_changed("");
    drain();
    assert_eq!(editor.document.borrow().items.len(), 2);
    // 기호 모양의 세로 중심이 옆 한글의 세로 중심과 1px 안에서 맞는다.
    buffer.set_text("* 가\n  * 가\n    * 가");
    editor.refresh();
    settle(30);
    let bullets = editor.widgets.bullets.borrow().clone();
    for (depth, (label, start)) in bullets.iter().zip([2, 8, 16]).enumerate() {
        let glyph = view.iter_location(&buffer.iter_at_offset(start));
        let (_, glyph_y) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, glyph.x(), glyph.y());
        let glyph_center = f64::from(glyph_y) + f64::from(glyph.height()) / 2.0;
        let top = label
            .compute_point(&view, &gtk::graphene::Point::new(0.0, 0.0))
            .unwrap()
            .y();
        let (ink, logical) = label.layout().extents();
        let scale = f64::from(pango::SCALE);
        let ink_center = f64::from(top)
            + f64::from(label.height()) / 2.0
            + (f64::from(ink.y() + ink.height() / 2)
                - f64::from(logical.y() + logical.height() / 2))
                / scale;
        assert!(
            (ink_center - glyph_center).abs() <= 1.0,
            "depth {depth} bullet center {ink_center} vs text {glyph_center}"
        );
    }
    settings.set_gtk_font_name(font.as_deref());
    buffer.set_text(text);
    settle(30);
    // TextView가 text 커서를 쓰는 실제 조건에서도 체크는 화살표로 구분한다.
    view.set_cursor_from_name(Some("text"));
    let checkbox = find_checkbox(view.upcast_ref()).unwrap();
    let mut cursor_owner = Some(checkbox.clone().upcast::<gtk::Widget>());
    let cursor = loop {
        let widget =
            cursor_owner.expect("checkbox cursor must resolve through its widget ancestry");
        if let Some(cursor) = widget.cursor() {
            break cursor;
        }
        cursor_owner = widget.parent();
    };
    assert_eq!(
        cursor.name().as_deref(),
        Some("default"),
        "checkbox must not inherit the editor I-beam"
    );
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
    assert!(tagged(&buffer, offset("결과:"), "md-task-done"));
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
    checkbox_pointer_targets();
}

/// 실제 앱 CSS·줄바꿈·중첩·스크롤·확대 글꼴에서도 체크의 표면과 클릭 대상이 일치한다.
pub(super) fn checkbox_pointer_targets() {
    crate::presentation::install();
    let original = format!(
        "- [ ] {}\n  - [ ] 중첩 항목\n{}",
        "긴 본문 ".repeat(24),
        "- [ ] 다음 항목\n".repeat(30)
    );
    let (window, view, source, editor) = shown_window(&original);
    let buffer = view.buffer();
    view.set_cursor_from_name(Some("text"));
    let picked_checkbox = |x: f64, y: f64, button: &CheckButton| {
        view.pick(x, y, gtk::PickFlags::DEFAULT)
            .is_some_and(|picked| {
                picked == *button.upcast_ref::<gtk::Widget>() || picked.is_ancestor(button)
            })
    };
    let verify = |index: usize| {
        let button = editor.widgets.tasks.borrow()[index].button.clone();
        let bounds = button.compute_bounds(&view).unwrap();
        assert_eq!(
            button.cursor().and_then(|cursor| cursor.name()).as_deref(),
            Some("default")
        );
        assert!(
            bounds.width() >= GUTTER as f32,
            "touch target spans the marker gutter: {bounds:?}"
        );
        assert!(bounds.height() > 0.);
        let check = button.first_child().unwrap();
        assert!(
            check.width() >= 13 && check.height() >= 13,
            "visual checkbox retains a square surface"
        );
        let x = f64::from(bounds.x() + bounds.width() / 2.);
        let y = f64::from(bounds.y() + bounds.height() / 2.);
        assert!(
            picked_checkbox(x, y, &button),
            "native check target follows rendered marker"
        );
        assert!(
            !picked_checkbox(f64::from(bounds.x() + GUTTER as f32 + 2.), y, &button),
            "body text is outside check target"
        );
        button
    };
    let first = verify(0);
    verify(1);
    let toggles = Rc::new(Cell::new(0));
    let seen = toggles.clone();
    buffer.connect_insert_text(move |_, _, inserted| {
        if inserted == "x" {
            seen.set(seen.get() + 1);
        }
    });
    assert!(first.activate());
    until(|| text(&buffer).starts_with("- [x] "));
    assert_eq!(
        toggles.get(),
        1,
        "native activation changes the source exactly once"
    );
    buffer.undo();
    until(|| text(&buffer) == original);
    let start = editor.document.borrow().items[0].content.start;
    buffer.select_range(
        &buffer.iter_at_offset(start),
        &buffer.iter_at_offset(start + 2),
    );
    drain();
    assert_eq!(text(&buffer), original, "selection leaves source unchanged");
    source.set_active(true);
    until(|| !first.is_visible());
    source.set_active(false);
    until(|| first.is_visible() && editor.layout.frames.get() == 0);
    // 글꼴 확대는 GTK 논리 좌표의 히트 영역도 다시 계산해야 한다.
    let css = gtk::CssProvider::new();
    css.load_from_data("textview.checkbox-target-test { font-size: 24px; }");
    gtk::style_context_add_provider_for_display(
        &view.display(),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );
    view.add_css_class("checkbox-target-test");
    buffer.place_cursor(&buffer.start_iter());
    editor.schedule();
    until(|| editor.layout.frames.get() == 0 && !editor.refresh_pending());
    verify(0);
    let target = editor.document.borrow().items[10].content.start;
    buffer.place_cursor(&buffer.iter_at_offset(target));
    view.scroll_to_iter(&mut buffer.iter_at_offset(target), 0., true, 0., 0.5);
    until(|| {
        let button = editor.widgets.tasks.borrow()[10].button.clone();
        let bounds = button.compute_bounds(&view).unwrap();
        bounds.y() >= 0.
            && bounds.y() + bounds.height() < view.height() as f32
            && editor.layout.frames.get() == 0
    });
    verify(10);
    gtk::style_context_remove_provider_for_display(&view.display(), &css);
    window.destroy();
    drain();
}
