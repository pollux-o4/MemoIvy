use super::*;

impl CanvasView {
    /// 배치·연결·완료 변경은 실제로 바뀐 경우에만 이력을 기록하고 저장한다.
    pub(in crate::app::canvas) fn mutate(
        self: &Rc<Self>,
        action: impl FnOnce(&mut port::Notes<'_>),
    ) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        if st.borrow().notebook.busy() {
            self.render();
            return;
        }
        let before = port::snapshot(&st.borrow());
        st.borrow_mut().notebook.edit_canvas(action);
        if port::snapshot(&st.borrow()) == before {
            self.render();
            return;
        }
        self.history.borrow_mut().record(before);
        lifecycle::mark(&st);
        self.render();
    }

    pub(in crate::app::canvas) fn undo(self: &Rc<Self>) {
        self.step_history(true);
    }

    pub(in crate::app::canvas) fn redo(self: &Rc<Self>) {
        self.step_history(false);
    }

    pub(in crate::app::canvas) fn step_history(self: &Rc<Self>, backwards: bool) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        if st.borrow().notebook.busy() || !self.session.interaction.borrow().is_idle() {
            return;
        }
        let current = port::snapshot(&st.borrow());
        let Some(snapshot) = self.history.borrow_mut().step(current, backwards) else {
            return;
        };
        st.borrow_mut()
            .notebook
            .edit_canvas(|canvas| port::restore(canvas, &snapshot));
        lifecycle::mark(&st);
        self.render();
    }

    pub(in crate::app::canvas) fn apply_moves(self: &Rc<Self>, moves: Moves) {
        if moves.is_empty() {
            return;
        }
        self.mutate(|notes| port::move_nodes(notes, &moves));
        self.reveal();
    }

    pub(in crate::app::canvas) fn toggle_done(self: &Rc<Self>, id: Uuid) {
        let Some(node) = self.node(id) else {
            return;
        };
        let done = !node.done;
        self.mutate(|notes| port::set_done(notes, id, done));
        if done && !node.next.is_empty() {
            self.signal(id);
        }
    }

    pub(in crate::app::canvas) fn toggle_selected_done(self: &Rc<Self>) {
        let selected = self.selected_views(|n| (n.id, n.done, !n.next.is_empty()));
        if selected.is_empty() {
            return;
        }
        let done = selected.iter().any(|(_, done, _)| !done);
        self.mutate(|notes| {
            for (id, _, _) in &selected {
                port::set_done(notes, *id, done);
            }
        });
        if done {
            for (id, was_done, linked) in selected {
                if linked && !was_done {
                    self.signal(id);
                }
            }
        }
    }

    /// NOTE-002: 복구할 수 없으므로 취소가 기본인 확인을 거친다.
    pub(in crate::app::canvas) fn delete_selected(self: &Rc<Self>) {
        let ids = self.selected_ids();
        let Some(st) = self.state.upgrade() else {
            return;
        };
        if ids.is_empty()
            || !self.session.interaction.borrow().is_idle()
            || st.borrow().notebook.busy()
        {
            return;
        }
        let title = match ids.as_slice() {
            [id] => self.node(*id).map(|n| n.display_title().to_owned()),
            _ => None,
        };
        crate::app::shared::confirm_delete(&self.widgets.window, &st, ids, title);
    }

    pub(in crate::app::canvas) fn connect_nodes(self: &Rc<Self>, from: Uuid, to: Uuid) {
        if !port::can_connect(&self.session.nodes.borrow(), from, to) {
            return;
        }
        self.mutate(|notes| {
            port::connect(notes, from, to);
        });
        motion::link_added(self, from, to);
    }

    pub(in crate::app::canvas) fn open_note(self: &Rc<Self>, id: Uuid) {
        if let Some(st) = self.state.upgrade() {
            shell_commands::open_note(&st, &self.app, id);
        }
    }
}
