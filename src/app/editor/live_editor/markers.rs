use super::*;

// shortcut: 기존 사용자 정의 체크의 호버·눌림 시각 상태 미반영, 체크 상호작용 스타일 정비 시 반영.
#[allow(deprecated)]
fn draw_task_marker(
    area: &gtk::DrawingArea,
    cr: &gtk::cairo::Context,
    center_y: f64,
    state: TaskState,
) {
    let size = 16.0;
    let x = (f64::from(GUTTER) - size) / 2.0;
    let y = center_y - size / 2.0;
    let border = area.color();
    let accent = area
        .style_context()
        .lookup_color("memoivy_accent")
        .unwrap_or(border);
    let set_color = |color: gdk::RGBA, alpha: f64| {
        cr.set_source_rgba(
            f64::from(color.red()),
            f64::from(color.green()),
            f64::from(color.blue()),
            f64::from(color.alpha()) * alpha,
        );
    };
    cr.new_path();
    cr.arc(
        x + size - 4.0,
        y + 4.0,
        4.0,
        1.5 * std::f64::consts::PI,
        2.0 * std::f64::consts::PI,
    );
    cr.arc(
        x + size - 4.0,
        y + size - 4.0,
        4.0,
        0.0,
        0.5 * std::f64::consts::PI,
    );
    cr.arc(
        x + 4.0,
        y + size - 4.0,
        4.0,
        0.5 * std::f64::consts::PI,
        std::f64::consts::PI,
    );
    cr.arc(
        x + 4.0,
        y + 4.0,
        4.0,
        std::f64::consts::PI,
        1.5 * std::f64::consts::PI,
    );
    cr.close_path();
    match state {
        TaskState::Done => {
            set_color(accent, 1.0);
            let _ = cr.fill_preserve();
            set_color(accent, 1.0);
            cr.set_line_width(1.5);
            cr.stroke().ok();
            let foreground = area
                .style_context()
                .lookup_color("memoivy_on_accent")
                .unwrap_or_else(|| gdk::RGBA::new(1.0, 1.0, 1.0, 1.0));
            set_color(foreground, 1.0);
            cr.set_line_width(1.8);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            cr.set_line_join(gtk::cairo::LineJoin::Round);
            cr.move_to(x + 4.0, y + 8.0);
            cr.line_to(x + 7.0, y + 11.0);
            cr.line_to(x + 12.0, y + 5.0);
            cr.stroke().ok();
        }
        TaskState::Doing => {
            set_color(border, 0.46);
            cr.set_line_width(1.5);
            cr.stroke().ok();
            cr.rectangle(x + 1.0, y + 1.0, size / 2.0 - 1.0, size - 2.0);
            set_color(accent, 1.0);
            let _ = cr.fill();
        }
        TaskState::Open => {
            set_color(border, 0.46);
            cr.set_line_width(1.5);
            cr.stroke().ok();
        }
    }
}

type DrawMarker = Box<dyn Fn(&gtk::Snapshot)>;

#[derive(Default)]
pub(super) struct MarkerWidgets {
    pub tasks: RefCell<Vec<TaskWidget>>,
    pub bullets: RefCell<Vec<gtk::DrawingArea>>,
    pub syncing: Cell<bool>,
    pub draws: RefCell<Vec<DrawMarker>>,
}

impl LiveEditor {
    pub(super) fn sync_tasks(self: &Rc<Self>, view: &TextView, document: &Document) {
        self.widgets.draws.borrow_mut().clear();
        let buffer = view.buffer();
        self.sync_bullets(view, document);
        let mut widgets = self.widgets.tasks.borrow_mut();
        for widget in widgets.iter().skip(document.tasks.len()) {
            widget.button.set_visible(false);
            widget.slot.set_visible(false);
        }
        while widgets.len() < document.tasks.len() {
            let button = gtk::CheckButton::new();
            button.add_css_class("md-checkbox");
            button.set_cursor_from_name(Some("default"));
            button.set_focusable(false);
            button.set_halign(gtk::Align::End);
            button.set_opacity(0.0);
            button.set_size_request(GUTTER, -1);
            button.set_tooltip_text(Some(
                "완료 상태 변경 · Ctrl+Enter\n진행 중 표시 · Shift+클릭",
            ));
            let offset = Rc::new(Cell::new(0));
            let position = offset.clone();
            let weak = Rc::downgrade(self);
            button.connect_toggled(move |_| {
                if let Some(editor) = weak.upgrade() {
                    if !editor.widgets.syncing.get() {
                        editor.toggle_task(position.get(), false);
                    }
                }
            });
            // Shift+클릭은 체크 버튼 자체 클릭보다 먼저 가로채 진행 중으로 바꾼다.
            let shift_click = gtk::GestureClick::new();
            shift_click.set_propagation_phase(gtk::PropagationPhase::Capture);
            let position = offset.clone();
            let weak = Rc::downgrade(self);
            shift_click.connect_pressed(move |gesture, _, _, _| {
                if !gesture
                    .current_event_state()
                    .contains(gdk::ModifierType::SHIFT_MASK)
                {
                    return;
                }
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(editor) = weak.upgrade() {
                    editor.toggle_task(position.get(), true);
                }
            });
            button.add_controller(shift_click);
            let marker = gtk::DrawingArea::new();
            marker.add_css_class("md-task-marker");
            marker.set_can_target(false);
            marker.set_halign(gtk::Align::End);
            let slot = gtk::Overlay::new();
            slot.set_size_request(GUTTER, -1);
            slot.set_child(Some(&marker));
            slot.add_overlay(&button);
            button.set_size_request(GUTTER, -1);
            let state = Rc::new(Cell::new(TaskState::Open));
            widgets.push(TaskWidget {
                button,
                slot,
                marker,
                state,
                offset,
            });
        }
        self.widgets.syncing.set(true);
        for (widget, task) in widgets.iter().zip(&document.tasks) {
            widget.offset.set(task.range.start);
            widget.button.set_active(task.state == TaskState::Done);
            widget
                .button
                .set_inconsistent(task.state == TaskState::Doing);
            widget.state.set(task.state);
            widget.marker.queue_draw();
            widget.button.set_sensitive(view.is_editable());
            let iter = buffer.iter_at_offset(task.range.start);
            let mut end = iter;
            end.forward_to_line_end();
            let label = super::super::source_slice(&buffer, task.range.end..end.offset());
            widget
                .button
                .update_property(&[gtk::accessible::Property::Label(label.trim())]);
            // 인용문처럼 원문으로 두는 항목에는 위젯을 올리지 않는다.
            let item = document
                .item_at(task.range.start)
                .map(|(_, item)| item)
                .filter(|item| {
                    item.marker.start <= task.range.start && task.range.end <= item.marker.end
                });
            let visible = !self.source.get() && item.is_some();
            widget.button.set_visible(visible);
            widget.slot.set_visible(visible);
            if let (true, Some(item)) = (visible, item) {
                let x = marker_x(view, item.depth);
                let line_metrics = crate::presentation::typography::line_anchor(
                    &view.create_pango_layout(Some("가")),
                    &view.create_pango_layout(Some("x")),
                    None,
                );
                let ascent = line_metrics.ascent.ceil() as i32;
                let marker = widget.marker.clone();
                let state = widget.state.clone();
                let weak_view = view.downgrade();
                let content = item.content.start;
                let x_center = line_metrics.x_center;
                marker.set_content_width(GUTTER);
                marker.set_content_height(ascent);
                marker.set_size_request(GUTTER, ascent);
                let marker_draw = marker.clone();
                self.widgets
                    .draws
                    .borrow_mut()
                    .push(Box::new(move |snapshot| {
                        let area = &marker_draw;
                        let Some(view) = weak_view.upgrade() else {
                            return;
                        };
                        let Some(bounds) = area.compute_bounds(&view) else {
                            return;
                        };
                        let cairo = snapshot.append_cairo(&bounds);
                        cairo.translate(f64::from(bounds.x()), f64::from(bounds.y()));
                        let cr = &cairo;
                        let iter = view.buffer().iter_at_offset(content);
                        let line_y = view.cursor_locations(Some(&iter)).0.y();
                        let (_, line_top) =
                            view.buffer_to_window_coords(gtk::TextWindowType::Widget, 0, line_y);
                        let area_top = area
                            .compute_point(&view, &gtk::graphene::Point::new(0.0, 0.0))
                            .map_or(0.0, |point| f64::from(point.y()));
                        let center_y =
                            f64::from(line_top) + line_metrics.baseline - x_center - area_top;
                        draw_task_marker(area, cr, center_y, state.get());
                    }));
                widget.slot.set_size_request(GUTTER, ascent);
                widget.button.set_size_request(GUTTER, ascent);
                widget.button.set_opacity(0.0);
                if widget.slot.parent().is_none() {
                    view.add_overlay(&widget.slot, 0, 0);
                }
                let slot = widget.slot.clone();
                let weak = view.downgrade();
                self.layout.allocations.borrow_mut().push(Box::new(move || {
                    if let Some(view) = weak.upgrade() {
                        let line_y = view
                            .cursor_locations(Some(&view.buffer().iter_at_offset(content)))
                            .0
                            .y();
                        let y = (f64::from(line_y) + line_metrics.baseline
                            - x_center
                            - f64::from(ascent) / 2.)
                            .round() as i32;
                        view.move_overlay(&slot, x, y);
                    }
                }));
            }
        }
        self.widgets.syncing.set(false);
    }

    /// 글머리 기호는 원문을 바꾸지 않고 표식 칸에 그린다. 입력·클릭은 본문으로 통과한다.
    pub(super) fn sync_bullets(&self, view: &TextView, document: &Document) {
        let bullets: Vec<_> = document
            .items
            .iter()
            .filter(|item| item.kind == ListKind::Bullet && !self.source.get())
            .collect();
        let mut widgets = self.widgets.bullets.borrow_mut();
        for widget in widgets.iter().skip(bullets.len()) {
            widget.set_visible(false);
        }
        while widgets.len() < bullets.len() {
            let marker = gtk::DrawingArea::new();
            marker.add_css_class("md-bullet");
            marker.set_can_target(false);
            widgets.push(marker);
        }
        for (area, item) in widgets.iter().zip(&bullets) {
            let x = marker_x(view, item.depth);
            let width = GUTTER - MARKER_GAP;
            let anchor = crate::presentation::typography::line_anchor(
                &view.create_pango_layout(Some("가")),
                &view.create_pango_layout(Some("x")),
                None,
            );
            let ascent = anchor.ascent.ceil() as i32;
            let glyph = markdown::BULLETS[item.depth.min(markdown::BULLETS.len() - 1)];
            let weak_view = view.downgrade();
            let content = item.content.start;
            area.set_content_width(width);
            area.set_content_height(ascent);
            area.set_size_request(width, ascent);
            area.set_halign(gtk::Align::Start);
            let marker_draw = area.clone();
            self.widgets
                .draws
                .borrow_mut()
                .push(Box::new(move |snapshot| {
                    let area = &marker_draw;
                    let Some(view) = weak_view.upgrade() else {
                        return;
                    };
                    let Some(bounds) = area.compute_bounds(&view) else {
                        return;
                    };
                    let layout = area.create_pango_layout(Some(glyph));
                    let (ink, logical) = layout.pixel_extents();
                    let line_y = view
                        .cursor_locations(Some(&view.buffer().iter_at_offset(content)))
                        .0
                        .y();
                    let (_, line_top) =
                        view.buffer_to_window_coords(gtk::TextWindowType::Widget, 0, line_y);
                    let center_y = f64::from(line_top) + anchor.baseline - anchor.x_center;
                    // Pango supplies fallback glyphs, matching the text instead of Cairo's toy font API.
                    snapshot.save();
                    snapshot.translate(&gtk::graphene::Point::new(
                        bounds.x() + (width - logical.width()) as f32,
                        (center_y - f64::from(ink.y()) - f64::from(ink.height()) / 2.0) as f32,
                    ));
                    snapshot.append_layout(&layout, &area.color());
                    snapshot.restore();
                }));
            area.set_visible(true);
            area.set_margin_start(0);
            if area.parent().is_none() {
                view.add_overlay(area, 0, 0);
            }
            let marker = area.clone();
            let weak = view.downgrade();
            self.layout.allocations.borrow_mut().push(Box::new(move || {
                if let Some(view) = weak.upgrade() {
                    let line_y = view
                        .cursor_locations(Some(&view.buffer().iter_at_offset(content)))
                        .0
                        .y();
                    let y = (f64::from(line_y) + anchor.baseline
                        - anchor.x_center
                        - f64::from(ascent) / 2.)
                        .round() as i32;
                    view.move_overlay(&marker, x, y);
                }
            }));
        }
    }

    /// `doing`이면 진행 중을, 아니면 완료를 켜고 끈다.
    pub(super) fn toggle_task(self: &Rc<Self>, offset: i32, doing: bool) -> bool {
        if self.input.composing.get() {
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
        let Some(item) = task.item.and_then(|index| document.items.get(index)) else {
            return false;
        };
        let mut start = buffer.iter_at_offset(item.marker.start);
        start.set_line_offset(0);
        let mut end = start;
        end.forward_to_line_end();
        while start.offset() < end.offset() && start.char() != '[' {
            start.forward_char();
        }
        if start.offset() >= end.offset() || !start.forward_char() {
            return false;
        }
        let mut end = start;
        if !end.forward_char() {
            return false;
        }
        buffer.begin_user_action();
        buffer.delete(&mut start, &mut end);
        buffer.insert(&mut start, task.state.toggled(doing).mark());
        buffer.end_user_action();
        true
    }
}
