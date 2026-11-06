use super::*;

impl CanvasView {
    pub(in crate::app::canvas) fn present(&self) {
        self.session.presenting.set(true);
        self.widgets.window.present();
        gtk::prelude::GtkWindowExt::set_focus(&self.widgets.window, None::<&gtk::Widget>);
        self.session.presenting.set(false);
    }

    pub(in crate::app) fn destroy(&self) {
        self.widgets.window.destroy();
    }

    pub(in crate::app::canvas) fn update_status(&self) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        let s = st.borrow();
        let failed = port::failed(&s);
        self.widgets.retry.set_visible(failed);
        if failed {
            self.widgets.dock.add_css_class("failed");
        } else {
            self.widgets.dock.remove_css_class("failed");
        }
        let idle = !s.notebook.busy();
        self.widgets
            .undo
            .set_sensitive(self.history.borrow().can_undo() && idle);
        self.widgets
            .redo
            .set_sensitive(self.history.borrow().can_redo() && idle);
        self.widgets.status.set_text(if failed {
            "저장 실패 · 내용은 유지됨"
        } else if !port::saved(&s) {
            "저장 중…"
        } else {
            "저장됨"
        });
    }

    pub(in crate::app::canvas) fn select_only(self: &Rc<Self>, id: Uuid) {
        self.session.selection.borrow_mut().set([id]);
        self.sync_selection();
    }

    pub(in crate::app::canvas) fn sync_selection(self: &Rc<Self>) {
        let selection = self.session.selection.borrow().clone();
        for (id, card) in self.widgets.cards.borrow().iter() {
            card.set_selected(selection.contains(*id));
        }
        panels::sync(self);
        scene::board::update_port(self);
        self.widgets.backdrop.queue_draw();
    }

    pub(in crate::app::canvas) fn render(self: &Rc<Self>) {
        self.update_status();
        scene::board::render(self);
        panels::sync(self);
        scene::board::update_port(self);
    }

    pub(in crate::app::canvas) fn signal(self: &Rc<Self>, id: Uuid) {
        motion::signal(self, id);
    }

    pub(in crate::app::canvas) fn node(&self, id: Uuid) -> Option<NodeView> {
        self.session
            .nodes
            .borrow()
            .iter()
            .find(|n| n.id == id)
            .cloned()
    }

    pub(in crate::app::canvas) fn shown_rect(&self, id: Uuid) -> Option<Rect> {
        self.session
            .shown
            .borrow()
            .get(&id)
            .map(|p| port::rect_at(*p))
    }

    pub(in crate::app::canvas) fn selected_views<T>(
        &self,
        read: impl Fn(&NodeView) -> T,
    ) -> Vec<T> {
        let selection = self.session.selection.borrow();
        self.session
            .nodes
            .borrow()
            .iter()
            .filter(|node| selection.contains(node.id))
            .map(read)
            .collect()
    }

    pub(in crate::app::canvas) fn selected_ids(&self) -> Vec<Uuid> {
        self.selected_views(|node| node.id)
    }

    pub(in crate::app::canvas) fn selected_nodes(&self) -> Vec<layout::Node> {
        self.selected_views(|node| (node.id, node.rect()))
    }

    pub(in crate::app::canvas) fn restore_cursor(&self) {
        self.widgets
            .surface
            .set_cursor_from_name(self.session.space.get().then_some("grab"));
    }
}
