use crate::presentation::markdown::{self, Document, Style};
use gtk::{gdk, glib, pango, prelude::*, CheckButton, TextBuffer, TextTag, TextView};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    ops::Range,
    rc::Rc,
};

struct TaskWidget {
    button: gtk::CheckButton,
    offset: Rc<Cell<i32>>,
}

struct LiveEditor {
    view: glib::WeakRef<TextView>,
    document: RefCell<Document>,
    tags: Vec<(Style, TextTag)>,
    hidden: TextTag,
    dirty: Cell<bool>,
    restyle: Cell<bool>,
    pending: Cell<bool>,
    composing: Cell<bool>,
    source: Cell<bool>,
    task_widgets: RefCell<Vec<TaskWidget>>,
    syncing_tasks: Cell<bool>,
    layout_frames: Cell<u8>,
}

impl LiveEditor {
    fn schedule(self: &Rc<Self>) {
        if self.pending.replace(true) {
            return;
        }
        let editor = self.clone();
        glib::idle_add_local_once(move || {
            editor.pending.set(false);
            editor.refresh();
        });
    }

    fn refresh(self: &Rc<Self>) {
        if self.composing.get() {
            return;
        }
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        if self.dirty.replace(false) {
            let text = buffer.text(&start, &end, true);
            *self.document.borrow_mut() = markdown::parse(&text);
            self.restyle.set(true);
        }
        let document = self.document.borrow();
        if self.restyle.replace(false) {
            for (style, tag) in &self.tags {
                let ranges = if self.source.get() {
                    Vec::new()
                } else {
                    document
                        .spans
                        .iter()
                        .filter(|span| span.style == *style)
                        .map(|span| span.range.clone())
                        .collect()
                };
                reconcile(&buffer, tag, ranges);
            }
        }
        let active = active_lines(&buffer);
        let hidden = if self.source.get() {
            Vec::new()
        } else {
            document
                .markers
                .iter()
                .filter(|marker| {
                    marker.owner.end <= active.start || marker.owner.start >= active.end
                })
                .map(|marker| marker.range.clone())
                .collect()
        };
        reconcile(&buffer, &self.hidden, hidden);
        drop(document);
        self.sync_tasks();
        self.layout_frames.set(2);
    }

    fn sync_tasks(self: &Rc<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        let document = self.document.borrow();
        let active = active_lines(&buffer);
        let mut widgets = self.task_widgets.borrow_mut();
        // shortcut: GTK overlay에는 공개 제거 API가 없어 창 안에서 최대 개수만큼 재사용, GTK가 제거 API를 제공하면 회수 정책 재검토.
        for widget in widgets.iter().skip(document.tasks.len()) {
            widget.button.set_visible(false);
        }
        while widgets.len() < document.tasks.len() {
            let button = gtk::CheckButton::new();
            button.add_css_class("md-checkbox");
            button.set_tooltip_text(Some("완료 상태 변경 · Ctrl+Enter"));
            let offset = Rc::new(Cell::new(0));
            let position = offset.clone();
            let weak = Rc::downgrade(self);
            button.connect_toggled(move |_| {
                if let Some(editor) = weak.upgrade() {
                    if !editor.syncing_tasks.get() {
                        editor.toggle_task(position.get());
                    }
                }
            });
            view.add_overlay(&button, 0, 0);
            widgets.push(TaskWidget { button, offset });
        }
        self.syncing_tasks.set(true);
        for (widget, task) in widgets.iter().zip(&document.tasks) {
            widget.offset.set(task.range.start);
            widget.button.set_active(task.checked);
            widget.button.set_sensitive(view.is_editable());
            let iter = buffer.iter_at_offset(task.range.start);
            let mut end = iter;
            end.forward_to_line_end();
            let label = buffer.text(&buffer.iter_at_offset(task.range.end), &end, true);
            widget
                .button
                .update_property(&[gtk::accessible::Property::Label(label.trim())]);
            let visible = !self.source.get()
                && (task.range.end <= active.start || task.range.start >= active.end);
            widget.button.set_visible(visible);
            if visible {
                let rect = view.iter_location(&buffer.iter_at_offset(task.display_start));
                let end = view.iter_location(&buffer.iter_at_offset(task.range.end));
                // Cover the reserved source marker without changing buffer contents or offsets.
                widget
                    .button
                    .set_size_request((end.x() - rect.x()).max(18), rect.height());
                view.move_overlay(&widget.button, rect.x(), rect.y());
            }
        }
        self.syncing_tasks.set(false);
    }

    fn toggle_task(self: &Rc<Self>, offset: i32) -> bool {
        if self.composing.get() {
            return false;
        }
        let Some(view) = self.view.upgrade() else {
            return false;
        };
        if !view.is_editable() {
            return false;
        }
        self.refresh();
        let document = self.document.borrow();
        let Some(task) = document
            .tasks
            .iter()
            .find(|task| task.range.contains(&offset))
        else {
            return false;
        };
        let buffer = view.buffer();
        let mut start = buffer.iter_at_offset(task.range.start + 1);
        let mut end = buffer.iter_at_offset(task.range.start + 2);
        buffer.begin_user_action();
        buffer.delete(&mut start, &mut end);
        buffer.insert(&mut start, if task.checked { " " } else { "x" });
        buffer.end_user_action();
        true
    }
}

// GTK already moves tag boundaries with edits; change only ranges whose decoration differs.
fn reconcile(buffer: &TextBuffer, tag: &TextTag, mut desired: Vec<Range<i32>>) {
    desired.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<i32>> = Vec::new();
    for range in desired {
        if let Some(last) = merged.last_mut().filter(|last| range.start <= last.end) {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    let desired: BTreeSet<_> = merged
        .into_iter()
        .map(|range| (range.start, range.end))
        .collect();
    let mut current = BTreeSet::new();
    let mut iter = buffer.start_iter();
    loop {
        if !iter.has_tag(tag) && !iter.forward_to_tag_toggle(Some(tag)) {
            break;
        }
        let start = iter.offset();
        iter.forward_to_tag_toggle(Some(tag));
        if start < iter.offset() {
            current.insert((start, iter.offset()));
        }
        if iter.is_end() {
            break;
        }
    }
    for &(start, end) in current.difference(&desired) {
        buffer.remove_tag(
            tag,
            &buffer.iter_at_offset(start),
            &buffer.iter_at_offset(end),
        );
    }
    for &(start, end) in desired.difference(&current) {
        buffer.apply_tag(
            tag,
            &buffer.iter_at_offset(start),
            &buffer.iter_at_offset(end),
        );
    }
}

fn copy_source(view: &TextView, cut: bool) {
    if cut && !view.is_editable() {
        return;
    }
    let buffer = view.buffer();
    if let Some((start, end)) = buffer.selection_bounds() {
        view.clipboard().set_text(&buffer.text(&start, &end, true));
        if cut {
            buffer.begin_user_action();
            buffer.delete_selection(true, true);
            buffer.end_user_action();
        }
    }
}

fn active_lines(buffer: &TextBuffer) -> Range<i32> {
    let (mut start, mut end) = buffer.selection_bounds().unwrap_or_else(|| {
        let cursor = buffer.iter_at_mark(&buffer.get_insert());
        (cursor, cursor)
    });
    start.set_line_offset(0);
    let has_next = end.forward_line();
    // Include an empty last line and the caret at EOF, without revealing the next line.
    start.offset()..(end.offset() + i32::from(!has_next))
}

pub(in crate::app) fn attach(view: &TextView) -> CheckButton {
    let buffer = view.buffer();
    buffer.set_enable_undo(true);
    let tags = vec![
        (
            Style::Heading1,
            TextTag::builder()
                .name("md-h1")
                .scale(1.6)
                .weight(700)
                .build(),
        ),
        (
            Style::Heading2,
            TextTag::builder()
                .name("md-h2")
                .scale(1.3)
                .weight(700)
                .build(),
        ),
        (
            Style::Bold,
            TextTag::builder().name("md-bold").weight(700).build(),
        ),
        (
            Style::Italic,
            TextTag::builder()
                .name("md-italic")
                .style(pango::Style::Italic)
                .build(),
        ),
        (
            Style::Strike,
            TextTag::builder()
                .name("md-strike")
                .strikethrough(true)
                .build(),
        ),
        (
            Style::Link,
            TextTag::builder()
                .name("md-link")
                .underline(pango::Underline::Single)
                .build(),
        ),
        (
            Style::Task,
            TextTag::builder()
                .name("md-task")
                .weight(700)
                .family("monospace")
                .build(),
        ),
    ];
    let hidden = TextTag::builder().name("md-hidden").invisible(true).build();
    for (_, tag) in &tags {
        buffer.tag_table().add(tag);
    }
    buffer.tag_table().add(&hidden);
    let editor = Rc::new(LiveEditor {
        view: view.downgrade(),
        document: RefCell::new(Document::default()),
        tags,
        hidden,
        dirty: Cell::new(true),
        restyle: Cell::new(true),
        pending: Cell::new(false),
        composing: Cell::new(false),
        source: Cell::new(false),
        task_widgets: RefCell::new(Vec::new()),
        syncing_tasks: Cell::new(false),
        layout_frames: Cell::new(0),
    });
    editor.refresh();
    let state = editor.clone();
    view.connect_map(move |_| state.schedule());
    let state = editor.clone();
    view.connect_editable_notify(move |_| state.schedule());
    let weak = Rc::downgrade(&editor);
    let width = Cell::new(0);
    view.add_tick_callback(move |view, _| {
        if let Some(editor) = weak.upgrade() {
            if view.width() != width.get() {
                width.set(view.width());
                editor.schedule();
            }
            // Text layout validates after tag changes; settle overlays on the following frames.
            if !editor.composing.get() && editor.layout_frames.get() > 0 {
                editor.sync_tasks();
                editor.layout_frames.set(editor.layout_frames.get() - 1);
            }
        }
        glib::ControlFlow::Continue
    });
    let state = editor.clone();
    buffer.connect_changed(move |_| {
        state.dirty.set(true);
        state.schedule();
    });
    let state = editor.clone();
    buffer.connect_mark_set(move |_, _, mark| {
        if mark
            .name()
            .is_some_and(|name| name == "insert" || name == "selection_bound")
        {
            state.schedule();
        }
    });
    let state = editor.clone();
    view.connect_preedit_changed(move |_, preedit| {
        state.composing.set(!preedit.is_empty());
        if preedit.is_empty() {
            state.schedule();
        }
    });
    let source = CheckButton::with_label("Markdown 원문 보기");
    source.add_css_class("lsm-action");
    source.set_tooltip_text(Some("Markdown 원문 표시 · 기본은 실시간 서식 편집"));
    let state = editor.clone();
    source.connect_toggled(move |button| {
        state.source.set(button.is_active());
        state.restyle.set(true);
        state.refresh();
        if let Some(view) = state.view.upgrade() {
            view.grab_focus();
        }
    });
    let click = gtk::GestureClick::new();
    click.set_button(1);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let state = editor.clone();
    click.connect_pressed(move |gesture, count, x, y| {
        if count != 1 || state.composing.get() {
            return;
        }
        let Some(view) = state.view.upgrade() else {
            return;
        };
        let (x, y) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
        let Some(iter) = view.iter_at_location(x, y) else {
            return;
        };
        let offset = iter.offset();
        if gesture
            .current_event_state()
            .contains(gdk::ModifierType::CONTROL_MASK)
        {
            state.refresh();
            if let Some(link) = state
                .document
                .borrow()
                .links
                .iter()
                .find(|link| link.range.contains(&offset))
            {
                let window = view.root().and_downcast::<gtk::Window>();
                // shortcut: FMT-001 링크 제스처를 유지, UriLauncher 전환 시 임시 핸들러 검증과 함께 교체.
                #[allow(deprecated)]
                gtk::show_uri(window.as_ref(), &link.url, gdk::CURRENT_TIME);
                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        } else if !view.buffer().has_selection() && state.toggle_task(offset) {
            gesture.set_state(gtk::EventSequenceState::Claimed);
        }
    });
    view.add_controller(click);
    view.connect_copy_clipboard(|view| {
        view.stop_signal_emission_by_name("copy-clipboard");
        copy_source(view, false);
    });
    view.connect_cut_clipboard(|view| {
        view.stop_signal_emission_by_name("cut-clipboard");
        copy_source(view, true);
    });
    let source_toggle = source.downgrade();
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if !editor.composing.get() {
            if let Some(view) = editor.view.upgrade() {
                if super::formatting::handle(&view, key, modifiers) {
                    return glib::Propagation::Stop;
                }
            }
        }
        if (key == gdk::Key::m || key == gdk::Key::M)
            && modifiers.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
        {
            if let Some(source) = source_toggle.upgrade() {
                source.set_active(!source.is_active());
            }
            return glib::Propagation::Stop;
        }
        if key == gdk::Key::Return && modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
            if let Some(view) = editor.view.upgrade() {
                let buffer = view.buffer();
                if !buffer.has_selection() {
                    let cursor = buffer.iter_at_mark(&buffer.get_insert());
                    editor.refresh();
                    let task = editor
                        .document
                        .borrow()
                        .tasks
                        .iter()
                        .find(|task| {
                            buffer.iter_at_offset(task.range.start).line() == cursor.line()
                        })
                        .map(|task| task.range.start);
                    if task.is_some_and(|offset| editor.toggle_task(offset)) {
                        return glib::Propagation::Stop;
                    }
                }
            }
        }
        glib::Propagation::Proceed
    });
    view.add_controller(keys);
    source
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn drain() {
        let context = glib::MainContext::default();
        while context.pending() {
            context.iteration(false);
        }
    }

    fn text(buffer: &TextBuffer) -> String {
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string()
    }

    fn tagged(buffer: &TextBuffer, offset: i32, name: &str) -> bool {
        let tag = buffer.tag_table().lookup(name).unwrap();
        buffer.iter_at_offset(offset).has_tag(&tag)
    }

    #[test]
    fn fmt_001_live_editor_preserves_source_selection_undo_and_preedit() {
        gtk::init().expect("FMT-001 requires a display; run under xvfb-run");
        crate::app::windows::focus::tests::focus_001_releases_and_restores_input_owner();
        crate::app::windows::canvas::tests::canvas_001_widget_flow_and_recovery();
        crate::app::windows::canvas::tests::perf_002_frames();
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
        fn find_checkbox(widget: &gtk::Widget) -> Option<gtk::CheckButton> {
            if let Some(button) = widget.downcast_ref::<gtk::CheckButton>() {
                return Some(button.clone());
            }
            let mut child = widget.first_child();
            while let Some(widget) = child {
                if let Some(button) = find_checkbox(&widget) {
                    return Some(button);
                }
                child = widget.next_sibling();
            }
            None
        }
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
}
