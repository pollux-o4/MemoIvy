use super::*;

#[derive(Default)]
pub(super) struct InputState {
    pub composing: Cell<bool>,
    /// 커서 이동과 입력을 구분해 타이핑 중 링크 카드를 띄우지 않는다.
    pub edited: Cell<bool>,
    pub caret_moved: Cell<bool>,
    /// 조합 중 Enter는 확정된 뒤 다음 항목 표식을 채운다.
    pub commit_newline: Cell<bool>,
}

#[derive(Default)]
pub(super) struct PointerState {
    pub hovered: RefCell<Option<Range<i32>>>,
    pub position: Cell<Option<(f64, f64)>>,
}

impl LiveEditor {
    pub(super) fn link_at(&self, offset: i32) -> Option<markdown::Link> {
        self.document
            .borrow()
            .links
            .iter()
            .find(|link| link.owner.contains(&offset))
            .cloned()
    }

    /// 포인터 또는 커서를 따라 링크 카드 예약을 갱신한다.
    pub(super) fn sync_caret_preview(self: &Rc<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        if self.input.composing.get() || buffer.has_selection() || !view.is_focus() {
            self.preview.caret_left();
            return;
        }
        let offset = buffer.iter_at_mark(&buffer.get_insert()).offset();
        let link = self.link_at(offset);
        if let Some(link) = link {
            self.preview
                .caret(&link, LinkPreview::anchor(&view, offset, None));
        } else {
            self.preview.caret_left();
        }
    }
}

pub(super) fn copy_source(view: &TextView, cut: bool) {
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

pub(super) fn connect_updates(view: &TextView, buffer: &TextBuffer, editor: &Rc<LiveEditor>) {
    let state = editor.clone();
    view.connect_map(move |_| state.schedule());
    let state = editor.clone();
    view.connect_editable_notify(move |_| state.schedule());
    let weak = Rc::downgrade(editor);
    let width = Cell::new(0);
    view.add_tick_callback(move |view, _| {
        if let Some(editor) = weak.upgrade() {
            if view.width() != width.get() {
                width.set(view.width());
                editor.preview.hide();
                editor.schedule();
            }
            // Text layout validates after tag changes; settle overlays on the following frames.
            if !editor.input.composing.get() && editor.layout.frames.get() > 0 {
                editor.sync_tasks();
                editor.place_images();
                editor.layout.frames.set(editor.layout.frames.get() - 1);
                if editor.layout.frames.get() == 0 {
                    editor.correct_hang();
                }
            }
        }
        glib::ControlFlow::Continue
    });
    let state = editor.clone();
    buffer.connect_changed(move |_| {
        state.dirty.set(true);
        state.input.edited.set(true);
        // 입력이 시작되면 카드는 바로 물러나고, 포인터가 다시 움직일 때까지 뜨지 않는다.
        state.preview.hide();
        state.pointer.hovered.replace(None);
        state.schedule_edit();
    });
    let state = editor.clone();
    buffer.connect_mark_set(move |_, _, mark| {
        if mark
            .name()
            .is_some_and(|name| name == "insert" || name == "selection_bound")
        {
            state.input.caret_moved.set(true);
            state.schedule();
        }
    });
    let state = editor.clone();
    // shortcut: 조합 신호 재현으로만 검증, 실제 Wayland·한글 IME 환경에서 Enter 확정 동작 재검증.
    view.connect_preedit_changed(move |_, preedit| {
        state.input.composing.set(!preedit.is_empty());
        if preedit.is_empty() {
            state.schedule();
        } else {
            state.preview.hide();
        }
    });
    // 이미 스크롤 뷰 안일 수도, 나중에 들어갈 수도 있어 현재 조절기와 이후 교체를 모두 잇는다.
    let weak = Rc::downgrade(editor);
    let watch = move |view: &TextView| {
        let Some(adjustment) = view.vadjustment() else {
            return;
        };
        let scrolled = weak.clone();
        adjustment.connect_value_changed(move |_| {
            if let Some(editor) = scrolled.upgrade() {
                editor.preview.hide();
                // GTK 4.18 이하(Snap gnome-46 런타임)는 스크롤 때 오버레이 표식을 다시 배치하지 않아
                // 표식이 문서 좌표에 남는다. 4.22는 스스로 하므로 여기서는 재배치만 요청한다.
                if let Some(view) = editor.view.upgrade() {
                    view.queue_allocate();
                }
            }
        });
        // 긴 문서는 뒤늦게 줄 높이가 확정되므로 표식 위치도 다시 맞춘다.
        let measured = weak.clone();
        adjustment.connect_changed(move |_| {
            if let Some(editor) = measured.upgrade() {
                editor.layout.frames.set(2);
            }
        });
    };
    watch(view);
    view.connect_vadjustment_notify(watch);
    if let Some(settings) = gtk::Settings::default() {
        for property in crate::presentation::theme::WATCHED {
            if settings.find_property(property).is_some() {
                let weak = Rc::downgrade(editor);
                settings.connect_notify_local(Some(property), move |_, _| {
                    if let Some(editor) = weak.upgrade() {
                        editor.schedule();
                    }
                });
            }
        }
    }
    let weak = Rc::downgrade(editor);
    view.connect_has_focus_notify(move |view| {
        if !view.has_focus() {
            if let Some(editor) = weak.upgrade() {
                // 다른 앱으로 넘어가면 떠 있는 표면을 남기지 않는다.
                editor.preview.hide();
            }
        }
    });
}

pub(super) fn source_toggle(editor: &Rc<LiveEditor>) -> CheckButton {
    let source = CheckButton::with_label("Markdown 원문 보기");
    source.add_css_class("memoivy-action");
    source.set_tooltip_text(Some("Markdown 원문 표시 · 기본은 실시간 서식 편집"));
    let state = editor.clone();
    source.connect_toggled(move |button| {
        state.source.set(button.is_active());
        state.restyle.set(true);
        state.preview.hide();
        state.refresh();
        if let Some(view) = state.view.upgrade() {
            view.grab_focus();
        }
    });
    source
}

pub(super) fn connect_gestures(view: &TextView, editor: &Rc<LiveEditor>) {
    let click = gtk::GestureClick::new();
    click.set_button(1);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let state = editor.clone();
    click.connect_pressed(move |gesture, count, x, y| {
        if count != 1 || state.input.composing.get() {
            return;
        }
        let Some(view) = state.view.upgrade() else {
            return;
        };
        let Some(iter) = char_at(&view, x, y) else {
            return;
        };
        let offset = iter.offset();
        if gesture
            .current_event_state()
            .contains(gdk::ModifierType::CONTROL_MASK)
        {
            state.refresh();
            if let Some(link) = state.link_at(offset) {
                state.preview.hide();
                link_preview::open_url(&view, &link.url);
                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        } else if !view.buffer().has_selection()
            && state.toggle_task(
                offset,
                gesture
                    .current_event_state()
                    .contains(gdk::ModifierType::SHIFT_MASK),
            )
        {
            gesture.set_state(gtk::EventSequenceState::Claimed);
        }
    });
    view.add_controller(click);
    connect_hover(view, editor);
    view.connect_copy_clipboard(|view| {
        view.stop_signal_emission_by_name("copy-clipboard");
        copy_source(view, false);
    });
    view.connect_cut_clipboard(|view| {
        view.stop_signal_emission_by_name("cut-clipboard");
        copy_source(view, true);
    });
}

/// 포인터 바로 아래 글자. 가까운 경계로 반올림하지 않고, 줄 끝 너머 빈 곳은 None이다.
pub(super) fn char_at(view: &TextView, x: f64, y: f64) -> Option<gtk::TextIter> {
    let (bx, by) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
    view.iter_at_position(bx, by).map(|(iter, _)| iter)
}

/// 링크 위 포인터를 카드 예약으로 옮긴다. 끄는 중이나 조합 중에는 띄우지 않는다.
pub(super) fn connect_hover(view: &TextView, editor: &Rc<LiveEditor>) {
    let motion = gtk::EventControllerMotion::new();
    let state = editor.clone();
    motion.connect_motion(move |controller, x, y| {
        let Some(view) = state.view.upgrade() else {
            return;
        };
        let modifiers = controller.current_event_state();
        let link =
            if modifiers.contains(gdk::ModifierType::BUTTON1_MASK) || state.input.composing.get() {
                None
            } else {
                char_at(&view, x, y).and_then(|iter| {
                    state
                        .link_at(iter.offset())
                        .map(|link| (link, iter.offset()))
                })
            };
        let owner = link.as_ref().map(|(link, _)| link.owner.clone());
        // 표면이 바뀌면 GTK가 제자리 이동을 합성한다. 진짜로 움직였을 때만 카드를 예약한다.
        let moved = state.pointer.position.replace(Some((x, y))) != Some((x, y));
        // Ctrl을 누르면 눌러서 여는 곳임을 커서로 알린다.
        let opens = owner.is_some() && modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        view.set_cursor_from_name(Some(if opens { "pointer" } else { "text" }));
        if !moved || *state.pointer.hovered.borrow() == owner {
            return;
        }
        state.pointer.hovered.replace(owner);
        match link {
            Some((link, offset)) => state
                .preview
                .hover(&link, LinkPreview::anchor(&view, offset, Some(x as i32))),
            None => state.preview.pointer_left(),
        }
    });
    let state = editor.clone();
    motion.connect_leave(move |_| {
        state.pointer.hovered.replace(None);
        state.preview.pointer_left();
    });
    view.add_controller(motion);
}

pub(super) fn connect_keys(view: &TextView, editor: Rc<LiveEditor>, source: &CheckButton) {
    let source_toggle = source.downgrade();
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if key == gdk::Key::Escape && editor.preview.is_shown() {
            editor.preview.dismiss();
            return glib::Propagation::Stop;
        }
        if !editor.input.composing.get() {
            if let Some(view) = editor.view.upgrade() {
                if super::super::formatting::handle(&view, key, modifiers) {
                    return glib::Propagation::Stop;
                }
                if view.is_editable() && !editor.source.get() {
                    // 대기 중인 갱신을 먼저 반영해 커서와 항목 범위가 어긋나지 않게 한다.
                    if editor.refresh_pending() {
                        editor.refresh_now();
                    }
                    let document = editor.document.borrow();
                    if list_edit::handle(&view, &document, key, modifiers) {
                        return glib::Propagation::Stop;
                    }
                }
            }
        } else if matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
            editor.input.commit_newline.set(true);
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
                    if task.is_some_and(|offset| editor.toggle_task(offset, false)) {
                        return glib::Propagation::Stop;
                    }
                    // 체크 항목이 아니면 커서 아래 링크를 연다. 키보드만으로 링크를 여는 경로다.
                    if let Some(link) = editor.link_at(cursor.offset()) {
                        editor.preview.hide();
                        link_preview::open_url(&view, &link.url);
                        return glib::Propagation::Stop;
                    }
                }
            }
        }
        glib::Propagation::Proceed
    });
    view.add_controller(keys);
}
