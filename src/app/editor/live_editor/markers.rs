use super::*;

#[derive(Default)]
pub(super) struct MarkerWidgets {
    pub tasks: RefCell<Vec<TaskWidget>>,
    pub bullets: RefCell<Vec<gtk::Label>>,
    pub syncing: Cell<bool>,
}

impl LiveEditor {
    pub(super) fn sync_tasks(self: &Rc<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        let document = self.document.borrow();
        self.sync_bullets(&view, &document);
        let mut widgets = self.widgets.tasks.borrow_mut();
        // shortcut: GTK overlay에는 공개 제거 API가 없어 창 안에서 최대 개수만큼 재사용, GTK가 제거 API를 제공하면 회수 정책 재검토.
        for widget in widgets.iter().skip(document.tasks.len()) {
            widget.button.set_visible(false);
        }
        while widgets.len() < document.tasks.len() {
            let button = gtk::CheckButton::new();
            button.add_css_class("md-checkbox");
            button.set_cursor_from_name(Some("default"));
            button.set_focusable(false);
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
            view.add_overlay(&button, 0, 0);
            widgets.push(TaskWidget { button, offset });
        }
        self.widgets.syncing.set(true);
        for (widget, task) in widgets.iter().zip(&document.tasks) {
            widget.offset.set(task.range.start);
            widget.button.set_active(task.state == TaskState::Done);
            widget
                .button
                .set_inconsistent(task.state == TaskState::Doing);
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
            if let (true, Some(item)) = (visible, item) {
                place_marker(
                    &view,
                    &widget.button,
                    item,
                    marker_x(&view, item.depth),
                    GUTTER,
                    0,
                );
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
            let label = gtk::Label::new(None);
            label.add_css_class("md-bullet");
            label.set_can_target(false);
            label.set_xalign(1.0);
            view.add_overlay(&label, 0, 0);
            widgets.push(label);
        }
        for (label, item) in widgets.iter().zip(&bullets) {
            label.set_text(markdown::BULLETS[item.depth.min(markdown::BULLETS.len() - 1)]);
            label.set_visible(true);
            let x = marker_x(view, item.depth);
            place_marker(view, label, item, x, GUTTER - MARKER_GAP, ink_offset(label));
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
        let mut start = buffer.iter_at_offset(task.range.start + 1);
        let mut end = buffer.iter_at_offset(task.range.start + 2);
        buffer.begin_user_action();
        buffer.delete(&mut start, &mut end);
        buffer.insert(&mut start, task.state.toggled(doing).mark());
        buffer.end_user_action();
        true
    }
}

/// 표식 위젯을 항목 첫 화면 줄에 맞춰 올린다. `nudge`는 줄 가운데에서 아래로 옮길 픽셀이다.
fn place_marker(
    view: &TextView,
    widget: &impl IsA<gtk::Widget>,
    item: &markdown::ListItem,
    x: i32,
    width: i32,
    nudge: i32,
) {
    let (y, height) = first_line_band(view, &view.buffer().iter_at_offset(item.content.start));
    widget.set_size_request(width, height);
    view.move_overlay(widget, x, y + nudge);
}

/// 기호 모양이 줄 가운데에 오도록 옮길 픽셀. 라벨은 글자 상자를 가운데 두지만
/// `•`처럼 상자 안에서 한쪽에 치우친 기호는 본문보다 위아래로 어긋나 보인다.
fn ink_offset(label: &gtk::Label) -> i32 {
    let (ink, logical) = label.layout().extents();
    let ink_center = ink.y() + ink.height() / 2;
    let box_center = logical.y() + logical.height() / 2;
    ((box_center - ink_center) as f64 / f64::from(pango::SCALE)).round() as i32
}

/// 항목 첫 화면 줄의 위치와 높이. 위쪽 줄 간격은 뺀다.
/// 글자 높이는 글꼴마다 달라 빈 줄과 한글 줄에서 표식이 흔들린다. 고정된 줄 높이를 쓴다.
fn first_line_band(view: &TextView, iter: &gtk::TextIter) -> (i32, i32) {
    let (top, height) = view.line_yrange(iter);
    let y = top + view.pixels_above_lines();
    let mut start = *iter;
    start.set_line_offset(0);
    let mut next = start;
    let bottom = if view.forward_display_line(&mut next) && next.line() == start.line() {
        view.iter_location(&next).y() - view.pixels_inside_wrap()
    } else {
        top + height - view.pixels_below_lines()
    };
    (y, bottom - y)
}
