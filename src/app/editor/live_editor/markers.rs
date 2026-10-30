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
            button.set_focusable(false);
            button.set_tooltip_text(Some("완료 상태 변경 · Ctrl+Enter"));
            let offset = Rc::new(Cell::new(0));
            let position = offset.clone();
            let weak = Rc::downgrade(self);
            button.connect_toggled(move |_| {
                if let Some(editor) = weak.upgrade() {
                    if !editor.widgets.syncing.get() {
                        editor.toggle_task(position.get());
                    }
                }
            });
            view.add_overlay(&button, 0, 0);
            widgets.push(TaskWidget { button, offset });
        }
        self.widgets.syncing.set(true);
        for (widget, task) in widgets.iter().zip(&document.tasks) {
            widget.offset.set(task.range.start);
            widget.button.set_active(task.checked);
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
                let rect = view.iter_location(&buffer.iter_at_offset(item.content.start));
                widget.button.set_size_request(CHECK_BOX, rect.height());
                view.move_overlay(&widget.button, marker_x(&view, item.depth), rect.y());
            }
        }
        self.widgets.syncing.set(false);
    }

    /// 글머리 기호는 원문을 바꾸지 않고 표식 칸에 그린다. 입력·클릭은 본문으로 통과한다.
    pub(super) fn sync_bullets(&self, view: &TextView, document: &Document) {
        let buffer = view.buffer();
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
            view.add_overlay(&label, 0, 0);
            widgets.push(label);
        }
        for (label, item) in widgets.iter().zip(&bullets) {
            label.set_text(markdown::BULLETS[item.depth.min(markdown::BULLETS.len() - 1)]);
            let rect = view.iter_location(&buffer.iter_at_offset(item.content.start));
            label.set_size_request(BULLET_BOX, rect.height());
            label.set_visible(true);
            view.move_overlay(label, marker_x(view, item.depth), rect.y());
        }
    }

    pub(super) fn toggle_task(self: &Rc<Self>, offset: i32) -> bool {
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
        buffer.insert(&mut start, if task.checked { " " } else { "x" });
        buffer.end_user_action();
        true
    }
}
