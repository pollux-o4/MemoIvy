use super::*;

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
    assert!(crate::app::editor::formatting::handle(
        &view,
        gdk::Key::b,
        gdk::ModifierType::CONTROL_MASK
    ));
    drain();
    assert_eq!(text(&buffer), "**한글😀**");
    assert!(crate::app::editor::formatting::handle(
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
