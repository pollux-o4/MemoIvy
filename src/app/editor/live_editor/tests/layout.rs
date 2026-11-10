use super::*;

pub(in crate::app) fn fmt_003_list_layout_and_hit_testing() {
    let text = "제목\n- 우유\n  - 저지방\n- [ ] 할 일\n- [x] 끝남\n      결과: 이어쓰기\n1. 번호\n- \n**굵게** 끝\n마지막 *줄*";
    let (window, view, source, editor) = shown_window(text);
    let buffer = view.buffer();
    // 입력 직후 첫 레이아웃부터 본문이 목록 앵커 뒤의 공통 열에 놓인다.
    for typed in ["- 첫", "- [ ] 첫", "- 상위\n  - 첫"] {
        buffer.set_text(&format!("{text}\n{typed}"));
        buffer.place_cursor(&buffer.end_iter());
        editor.refresh();
        settle(30);
        let item = editor.document.borrow().items.last().unwrap().content.start;
        let depth = i32::from(typed.contains("\n  "));
        assert_eq!(
            view.cursor_locations(Some(&buffer.iter_at_offset(item)))
                .0
                .x(),
            20 + GUTTER * (depth + 1),
            "{typed:?} content must start aligned without waiting for a frame"
        );
    }
    buffer.set_text("- 앵커 위치");
    editor.refresh();
    settle(30);
    let label = editor.widgets.bullets.borrow()[0].clone();
    let marker_position = || {
        let bounds = label.compute_bounds(&view).unwrap();
        (bounds.x(), bounds.y())
    };
    let before = marker_position();
    buffer.place_cursor(&buffer.end_iter());
    buffer.insert_at_cursor(" 입력");
    editor.refresh();
    settle(30);
    assert_eq!(
        marker_position(),
        before,
        "inline marker moved during typing"
    );
    assert_eq!(
        view.cursor_locations(Some(
            &buffer.iter_at_offset(editor.document.borrow().items[0].content.start)
        ))
        .0
        .x(),
        20 + GUTTER,
        "typing changed the shared list text column"
    );
    // 빈 항목에 한글을 조합·확정·지우는 동안 본문이 1px도 움직이지 않는다.
    // 아랫줄도 위아래로 흔들리지 않는다. 빈 줄과 한글 줄의 높이가 같아야 한다.
    // 한글이 없는 기본 글꼴이어야 빈 줄과 한글 줄의 높이 차이가 드러난다.
    let settings = gtk::Settings::default().unwrap();
    let font = settings.gtk_font_name();
    settings.set_gtk_font_name(Some("Ubuntu Sans 11"));
    buffer.set_text("* \n아래");
    buffer.place_cursor(&source_iter(&buffer, 2));
    editor.refresh();
    settle(30);
    editor.refresh();
    settle(30);
    let below = || view.cursor_locations(Some(&buffer.end_iter())).0.y();
    let below_empty = below();
    let bullet = || marker_ink(&view, 0);
    let bullet_empty = bullet();
    let still = |step: &str| {
        assert_eq!(
            view.cursor_locations(Some(&source_iter(&buffer, 2))).0.x(),
            20 + GUTTER,
            "content moved at {step}"
        );
        assert_eq!(below(), below_empty, "next line moved vertically at {step}");
        assert!(bullet() == bullet_empty, "rendered bullet moved at {step}");
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
    editor.refresh();
    settle(30);
    let empty = editor.document.borrow().items[2].content.start;
    buffer.place_cursor(&buffer.iter_at_offset(empty));
    editor.refresh();
    let expected_x = 20 + GUTTER * 3;
    assert_eq!(
        view.cursor_locations(Some(&buffer.iter_at_offset(empty)))
            .0
            .x(),
        expected_x
    );
    view.emit_preedit_changed("ㄱ");
    drain();
    assert_eq!(
        view.cursor_locations(Some(&buffer.iter_at_offset(empty)))
            .0
            .x(),
        expected_x
    );
    view.emit_preedit_changed("");
    drain();
    // 선택한 체크 항목 문장을 IME 조합으로 교체해도 체크박스와 들여쓰기는 고정한다.
    buffer.set_text("- 바깥\n  - [ ] 시작 안함");
    editor.refresh();
    settle(30);
    let start = editor.document.borrow().items[1].content.start;
    buffer.select_range(
        &buffer.iter_at_offset(buffer.end_iter().offset()),
        &buffer.iter_at_offset(start),
    );
    editor.refresh();
    let body_x = view
        .cursor_locations(Some(&buffer.iter_at_offset(start)))
        .0
        .x();
    assert!(editor.widgets.tasks.borrow()[0].button.is_visible());
    view.emit_preedit_changed("작");
    buffer.insert_at_cursor("작");
    drain();
    assert_eq!(editor.document.borrow().items.len(), 2);
    assert!(editor.widgets.tasks.borrow()[0].button.is_visible());
    assert_eq!(
        view.cursor_locations(Some(&buffer.iter_at_offset(start)))
            .0
            .x(),
        body_x
    );
    view.emit_preedit_changed("");
    drain();
    assert_eq!(editor.document.borrow().items.len(), 2);
    // 기호 모양의 세로 중심이 첫 줄 x-height 중심과 1px 안에서 맞는다.
    buffer.set_text("* 가\n  * 가\n    * 가");
    editor.refresh();
    settle(30);
    let bullets = editor.widgets.bullets.borrow().clone();
    for (depth, (area, start)) in bullets.iter().zip([2, 8, 16]).enumerate() {
        let iter = source_iter(&buffer, start);
        let (_, line_top) = view.buffer_to_window_coords(
            gtk::TextWindowType::Widget,
            0,
            view.cursor_locations(Some(&iter)).0.y(),
        );
        let anchor = crate::presentation::typography::line_anchor(
            &view.create_pango_layout(Some("가")),
            &view.create_pango_layout(Some("x")),
            None,
        );
        let bounds = area
            .compute_point(&view, &gtk::graphene::Point::new(0.0, 0.0))
            .unwrap()
            .y();
        let glyph_center = f64::from(line_top) + anchor.baseline - anchor.x_center;
        let local_center = glyph_center - f64::from(bounds);
        let ink_center = f64::from(bounds) + local_center;
        assert!(
            (0.0..=f64::from(area.height())).contains(&local_center),
            "depth {depth} bullet ink center is inside ascent-sized area: {local_center}"
        );
        assert!(
            (ink_center - glyph_center).abs() <= 1.0,
            "depth {depth} bullet center {ink_center} vs text {glyph_center}; area_top={bounds} ascent={} x_center={} line_top={line_top} baseline={}",
            anchor.ascent,
            anchor.x_center,
            anchor.baseline,
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
    let hanging = |at: i32| view.cursor_locations(Some(&source_iter(&buffer, at))).0.x();
    for caret in [0, offset("우유")] {
        buffer.place_cursor(&source_iter(&buffer, caret));
        settle(40);
        assert_eq!(hanging(offset("우유")), 20 + GUTTER);
        assert_eq!(hanging(offset("저지방")), 20 + GUTTER * 2);
        assert_eq!(hanging(offset("할 일")), 20 + GUTTER);
        assert_eq!(hanging(offset("번호")), 20 + GUTTER);
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
    assert_eq!(hanging(offset("- \n") + 2), 20 + GUTTER);
    // 선택해도 표식을 유지한다. 복사는 원문이다.
    buffer.select_range(
        &source_iter(&buffer, offset("우유")),
        &source_iter(&buffer, offset("우유") + 1),
    );
    settle(30);
    assert!(tagged(&buffer, offset("- 우유"), "md-marker"));
    // 표식 안의 커서는 본문 앞으로 옮긴다.
    buffer.place_cursor(&source_iter(&buffer, offset("- [ ] 할 일")));
    settle(30);
    assert_eq!(
        buffer.iter_at_mark(&buffer.get_insert()).offset(),
        source_iter(&buffer, offset("할 일")).offset()
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
        .iter_at_offset(source_iter(&buffer, offset("끝남")).offset())
        .tags()
        .iter()
        .all(|t| !t.is_left_margin_set()));
    window.destroy();
    drain();
    checkbox_pointer_targets();
}

/// Font fallback ink regression measurements using the active application font.
pub(in crate::app) fn fmt_003_m1_typography_ink_regression() {
    crate::presentation::install();
    let (window, view, _source, editor) = shown_window("p오늘일아\nLogs가나\n# 제목\n## 제목");
    assert_eq!(view.pango_context().language().unwrap().to_string(), "ko");
    let measure = |text: &str, heading_scale: Option<f64>| {
        let layout = view.create_pango_layout(Some(text));
        if let Some(scale) = heading_scale {
            let attrs = pango::AttrList::new();
            let mut size = pango::AttrFloat::new_scale(scale);
            size.set_start_index(0);
            size.set_end_index(text.len() as u32);
            attrs.insert(size);
            let mut weight = pango::AttrInt::new_weight(pango::Weight::Bold);
            weight.set_start_index(0);
            weight.set_end_index(text.len() as u32);
            attrs.insert(weight);
            layout.set_attributes(Some(&attrs));
        }
        let (ink, logical) = layout.extents();
        let baseline = f64::from(layout.baseline()) / f64::from(pango::SCALE);
        let scale = f64::from(pango::SCALE);
        (
            f64::from(ink.y()) / scale,
            f64::from(ink.y() + ink.height()) / scale,
            f64::from(logical.y()) / scale,
            f64::from(logical.y() + logical.height()) / scale,
            baseline,
        )
    };
    let run_family = |layout: &pango::Layout, text: &str, character: char| {
        let byte = text.find(character).unwrap() as i32;
        layout
            .line_readonly(0)
            .unwrap()
            .runs()
            .into_iter()
            .find(|run| {
                let item = run.item();
                item.offset() <= byte && byte < item.offset() + item.length()
            })
            .and_then(|run| {
                run.item()
                    .analysis()
                    .font()
                    .describe()
                    .family()
                    .map(|family| family.to_string())
            })
            .expect("Pango run family")
    };
    let mut failures = Vec::new();
    for line in ["p오늘일아", "Logs가나"] {
        let latin = line.chars().find(|c| c.is_ascii_alphabetic()).unwrap();
        let line_layout = view.create_pango_layout(Some(line));
        let latin_family = run_family(&line_layout, line, latin);
        let hangul = line.chars().find(|c| ('가'..='힣').contains(c)).unwrap();
        let hangul_ink = measure(&hangul.to_string(), None);
        let cap_ink = measure("H", None);
        let cap_center = (cap_ink.0 + cap_ink.4) / 2.0;
        let hangul_center = (hangul_ink.0 + hangul_ink.1) / 2.0;
        let center_delta = (hangul_center - cap_center).abs();
        let latin_family_h = run_family(&view.create_pango_layout(Some("H")), "H", 'H');
        let hangul_families: Vec<_> = line
            .chars()
            .filter(|c| ('가'..='힣').contains(c))
            .map(|c| run_family(&line_layout, line, c))
            .collect();
        let same_family = hangul_families.iter().all(|family| family == &latin_family);
        println!(
            "M1 mixed={line:?} hangul_ink=({:.2},{:.2}) H_cap_top={:.2}px baseline={:.2}px center_delta={center_delta:.2}px latin_family={latin_family:?} H_family={latin_family_h:?} hangul_families={hangul_families:?} same_line_family={same_family}",
            hangul_ink.0, hangul_ink.1, cap_ink.0, cap_ink.4
        );
        println!(
            "M1 mixed={line:?} top_edge_delta={:.2}px bottom_edge_delta={:.2}px (reported only)",
            (hangul_ink.0 - cap_ink.0).abs(),
            (hangul_ink.1 - cap_ink.4).abs()
        );
        if center_delta > 1.0 {
            failures.push(format!(
                "{line:?} Hangul center vs H cap center: {center_delta}px"
            ));
        }
        if !same_family {
            failures.push(format!(
                "{line:?} Latin/Hangul Pango runs use different families"
            ));
        }
    }
    for (name, scale) in [
        ("H1", crate::presentation::typography::EDITOR_H1),
        ("H2", crate::presentation::typography::EDITOR_H2),
    ] {
        let ink = measure("제목", Some(scale));
        println!(
            "M1 {name} ink_top={:.2}px ink_bottom={:.2}px baseline={:.2}px",
            ink.0, ink.1, ink.4
        );
    }
    measure_overlay_markers(&view, &editor, &mut failures);
    assert!(failures.is_empty(), "{}", failures.join("; "));
    window.destroy();
    drain();
}

pub(in crate::app) fn fmt_003_empty_nested_item_keeps_next_paragraph_x() {
    crate::presentation::install();
    let (window, view, _source, editor) = shown_window("");
    let buffer = view.buffer();
    for glyph in markdown::BULLETS {
        assert_eq!(
            view.create_pango_layout(Some(glyph)).unknown_glyphs_count(),
            0,
            "list marker must resolve through Pango fallback: {glyph}"
        );
    }
    let controllers = view.observe_controllers();
    let keys = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .unwrap();
    let mut originals: Vec<String> = [
        "- outer\n  - nested\n- 블로그 (2)",
        "현재 작업 흐름\n- 실마리 (1)\n  - 기존 예쁘게 작업해놓은 페이지로 이동\n- 블로그 (2)\n- [x] 마일스톤 확인했음\n\n| 단계 | 목표 |\n| --- | --- |\n| 원문 | 가져오기 |",
        "- 상위\n  - 중간\n    - 하위\n  - 블로그 (2)\n- [x] 확인",
    ].into_iter().map(str::to_owned).collect();
    for entered_depth in 0..3 {
        for following_depth in 0..3 {
            for final_depth in 0..3 {
                originals.push(format!(
                    "- 상위\n{}- 하위\n{}- 블로그 (2)\n{}- 마지막",
                    "  ".repeat(entered_depth),
                    "  ".repeat(following_depth),
                    "  ".repeat(final_depth)
                ));
            }
        }
    }
    for original in &originals {
        let original = original.as_str();
        buffer.set_text(original);
        editor.refresh();
        settle(30);
        let blog_byte = original.find("블로그").unwrap();
        let previous_end = original[..blog_byte].rfind('\n').unwrap();
        let end = original[..previous_end].chars().count() as i32;
        let blog_offset = original[..blog_byte].chars().count() as i32;
        let position = |offset| view.cursor_locations(Some(&source_iter(&buffer, offset))).0;
        let expected = position(blog_offset);
        buffer.place_cursor(&source_iter(&buffer, end));
        drain();
        assert!(keys.emit_by_name::<bool>(
            "key-pressed",
            &[&gdk::Key::Return, &0u32, &gdk::ModifierType::empty()]
        ));
        drain();
        settle(30);
        let inserted = text(&buffer).chars().count() as i32 - original.chars().count() as i32;
        let actual = position(blog_offset + inserted);
        assert_eq!(
            source_iter(&buffer, blog_offset + inserted)
                .tags()
                .iter()
                .filter(|tag| tag.is_left_margin_set())
                .count(),
            1,
            "empty item must not leak its paragraph tag into the following body"
        );
        assert_eq!(
            actual.x(),
            expected.x(),
            "Enter shifted next body: {original:?}"
        );
        let empty_start = buffer.iter_at_mark(&buffer.get_insert()).offset();
        view.emit_preedit_changed("ㅏㅇ");
        drain();
        assert_eq!(
            position(blog_offset + inserted).x(),
            expected.x(),
            "preedit shifted next body"
        );
        view.emit_preedit_changed("");
        drain();
        assert_eq!(
            buffer.iter_at_mark(&buffer.get_insert()).offset(),
            empty_start
        );
        let inserted_source = text(&buffer);
        let (marker_start, remove) = {
            let document = editor.document.borrow();
            let (_, item) = document.item_at(empty_start).unwrap();
            (
                item.marker.start as usize,
                if item.depth > 0 { 2 } else { item.marker.len() },
            )
        };
        let mut outdented: Vec<_> = inserted_source.chars().collect();
        outdented.drain(marker_start..marker_start + remove);
        assert!(keys.emit_by_name::<bool>(
            "key-pressed",
            &[&gdk::Key::Return, &0u32, &gdk::ModifierType::empty()]
        ));
        drain();
        assert_eq!(
            text(&buffer),
            outdented.iter().collect::<String>(),
            "Enter on an empty item must outdent/end without consuming the following body"
        );
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), inserted_source);
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), original);
        assert_eq!(
            position(blog_offset).x(),
            expected.x(),
            "undo shifted next body"
        );
    }

    window.destroy();
    drain();
}

fn measure_overlay_markers(view: &TextView, editor: &Rc<LiveEditor>, failures: &mut Vec<String>) {
    let lines = [
        "* 가불릿",
        "  * EnglishBullet",
        "    * 한글Bullet",
        "* [x] 가체크",
        "  * [ ] EnglishTask",
        "    * [x] 한글Task",
    ];
    let source = lines.join("\n");
    let buffer = view.buffer();
    buffer.set_text(&source);
    editor.refresh();
    settle(30);
    let x_layout = view.create_pango_layout(Some("x"));
    let (x_ink, _) = x_layout.extents();
    let scale = f64::from(pango::SCALE);
    let first_line_x_height_center = |body: &str| {
        let byte = source.find(body).unwrap();
        let offset = source[..byte].chars().count() as i32;
        let iter = source_iter(&buffer, offset);
        let location = view.cursor_locations(Some(&iter)).0;
        let (_, line_top) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, location.x(), location.y());
        f64::from(line_top) + f64::from(x_layout.baseline()) / scale
            - f64::from(x_ink.height()) / scale / 2.0
    };
    let marker_area_ink_center = |area: &gtk::DrawingArea, body: &str, glyph: &str| {
        let iter = source_iter(
            &buffer,
            source[..source.find(body).unwrap()].chars().count() as i32,
        );
        let location = view.cursor_locations(Some(&iter)).0;
        let (_, line_top) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, location.x(), location.y());
        let anchor = crate::presentation::typography::line_anchor(
            &view.create_pango_layout(Some("가")),
            &x_layout,
            None,
        );
        let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, 1, 1).unwrap();
        let cr = gtk::cairo::Context::new(&surface).unwrap();
        let description = area.pango_context().font_description().unwrap();
        let family = description.family().unwrap_or_else(|| "Sans".into());
        cr.select_font_face(
            &family,
            gtk::cairo::FontSlant::Normal,
            gtk::cairo::FontWeight::Normal,
        );
        cr.set_font_size(f64::from(description.size()) / scale);
        let extents = cr.text_extents(glyph).unwrap();
        let area_top = f64::from(
            area.compute_point(view, &gtk::graphene::Point::new(0., 0.))
                .unwrap()
                .y(),
        );
        let center_y = f64::from(line_top) + anchor.baseline - anchor.x_center - area_top;
        let baseline = center_y - (extents.y_bearing() + extents.height() / 2.0);
        area_top + baseline + extents.y_bearing() + extents.height() / 2.0
    };
    for (index, body) in ["가불릿", "EnglishBullet", "한글Bullet"].iter().enumerate() {
        let area = editor.widgets.bullets.borrow()[index].clone();
        let marker_ink_center = marker_area_ink_center(
            &area,
            body,
            markdown::BULLETS[index.min(markdown::BULLETS.len() - 1)],
        );
        let text_center = first_line_x_height_center(body);
        report_marker_center(
            "bullet",
            index + 1,
            body,
            marker_ink_center,
            text_center,
            failures,
        );
    }
    for (index, body) in ["가체크", "EnglishTask", "한글Task"].iter().enumerate() {
        let marker = editor.widgets.tasks.borrow()[index].marker.clone();
        let iter = source_iter(
            &buffer,
            source[..source.find(body).unwrap()].chars().count() as i32,
        );
        let location = view.cursor_locations(Some(&iter)).0;
        let (_, line_top) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, location.x(), location.y());
        let anchor = crate::presentation::typography::line_anchor(
            &view.create_pango_layout(Some("가")),
            &x_layout,
            None,
        );
        let area_top = f64::from(
            marker
                .compute_point(view, &gtk::graphene::Point::new(0.0, 0.0))
                .unwrap()
                .y(),
        );
        let marker_ink_center =
            area_top + f64::from(line_top) + anchor.baseline - anchor.x_center - area_top;
        let text_center = first_line_x_height_center(body);
        report_marker_center(
            "checkbox",
            index + 1,
            body,
            marker_ink_center,
            text_center,
            failures,
        );
    }
}

fn report_marker_center(
    kind: &str,
    depth: usize,
    first_char: &str,
    marker_ink_center: f64,
    first_line_x_height_center: f64,
    failures: &mut Vec<String>,
) {
    let delta = (marker_ink_center - first_line_x_height_center).abs();
    println!("M1 editor_{kind} depth={depth} first_char={first_char:?} marker_ink_center={marker_ink_center:.2}px first_line_x_height_center={first_line_x_height_center:.2}px delta={delta:.2}px");
    if delta > 1.0 {
        failures.push(format!(
            "editor {kind} depth {depth} {first_char:?}: {delta}px"
        ));
    }
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
        until(|| button.width() >= GUTTER && button.height() > 0);
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
    until(|| first.is_visible() && !editor.refresh_pending());
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
    until(|| !editor.refresh_pending());
    verify(0);
    // 배치 단계에서 클릭 대상도 그려진 표식과 같은 줄을 따라야 한다.
    let offset_from_line = |index: usize| {
        let marker = editor.widgets.tasks.borrow()[index].marker.clone();
        let content = editor.document.borrow().items[index].content.start;
        let line_iter = buffer.iter_at_offset(content);
        let location = view.cursor_locations(Some(&line_iter)).0;
        let (_, line_top) =
            view.buffer_to_window_coords(gtk::TextWindowType::Widget, location.x(), location.y());
        let x_layout = view.create_pango_layout(Some("x"));
        let (x_ink, _) = x_layout.extents();
        let scale = f64::from(pango::SCALE);
        let text_center = f64::from(line_top) + f64::from(x_layout.baseline()) / scale
            - f64::from(x_ink.height()) / scale / 2.0;
        let line_anchor = crate::presentation::typography::line_anchor(
            &view.create_pango_layout(Some("가")),
            &x_layout,
            None,
        );
        let area_top = f64::from(
            marker
                .compute_point(&view, &gtk::graphene::Point::new(0.0, 0.0))
                .unwrap()
                .y(),
        );
        let ink_center =
            area_top + f64::from(line_top) + line_anchor.baseline - line_anchor.x_center - area_top;
        ink_center - text_center
    };
    let resting: Vec<_> = [0, 5, 10, 20].map(offset_from_line).into();
    let target = editor.document.borrow().items[10].content.start;
    buffer.place_cursor(&buffer.iter_at_offset(target));
    // shortcut: GTK 숨김 문법의 iter 스크롤 좌표가 어긋나 실제 조절기로 검증, GTK 갱신 시 iter 경로 재검증.
    let target_y = view
        .cursor_locations(Some(&buffer.iter_at_offset(target)))
        .0
        .y();
    view.vadjustment()
        .unwrap()
        .set_value(f64::from(target_y - view.height() / 2));
    until(|| !editor.refresh_pending());
    drain();
    for (index, resting) in [5, 10, 20].into_iter().zip(resting.into_iter().skip(1)) {
        assert!(
            (offset_from_line(index) - resting).abs() <= 1.,
            "marker {index} follows its line after scrolling: {} vs {resting}",
            offset_from_line(index)
        );
    }
    until(|| {
        let marker = editor.widgets.tasks.borrow()[10].marker.clone();
        let bounds = marker.compute_bounds(&view).unwrap();
        bounds.y() >= 0. && bounds.y() + bounds.height() < view.height() as f32
    });
    verify(10);
    gtk::style_context_remove_provider_for_display(&view.display(), &css);
    window.destroy();
    drain();
}

pub(in crate::app) fn fmt_003_wrapped_list_columns() {
    let original = "- [x] 체크박스에 진행 중을 거치고 완료한 내용이 좁은 창에서도 본문 열에 맞춰 여러 줄로 표시되어야 합니다\n  - 중첩 목록도 긴 한글 문장과 shift enter 입력한 내용이 같은 열에서 이어져야 합니다\n1. 번호 목록 역시 긴 본문을 입력했을 때 줄바꿈된 내용이 같은 본문 열에 정렬되어야 합니다\n\n현재 작업 큐";
    let (window, view, source, editor) = shown_window(original);
    let buffer = view.buffer();
    for selected in [false, true, false] {
        if selected {
            buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
        } else {
            buffer.place_cursor(&buffer.end_iter());
        }
        editor.refresh();
        settle(80);
        let document = editor.document.borrow();
        for item in &document.items {
            let expected = view.left_margin() + GUTTER * (item.depth as i32 + 1);
            let mut iter = buffer.iter_at_offset(item.content.start);
            assert_eq!(
                view.cursor_locations(Some(&iter)).0.x(),
                expected,
                "first line {:?}, selected={selected}",
                item.kind
            );
            let mut wrapped = 0;
            while view.forward_display_line(&mut iter) && iter.offset() < item.content.end {
                assert_eq!(
                    view.cursor_locations(Some(&iter)).0.x(),
                    expected,
                    "wrapped line {:?}, selected={selected}",
                    item.kind
                );
                wrapped += 1;
            }
            assert!(wrapped > 0, "fixture must wrap {:?}", item.kind);
        }
        assert_eq!(text(&buffer), original);
    }
    source.set_active(true);
    settle(40);
    assert_eq!(text(&buffer), original);
    window.destroy();
    drain();
}
