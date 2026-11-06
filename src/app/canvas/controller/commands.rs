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

/// 들어오는 카메라 스프링이 대부분 끝나는 시간.
const SETTLE_MS: u64 = 600;

impl CanvasView {
    /// 프로젝트 카드 안으로 들어간다.
    pub(in crate::app::canvas) fn enter(self: &Rc<Self>, id: Uuid) {
        if self.node(id).is_some_and(|n| n.project.is_some()) {
            self.go_to(Some(id));
        }
    }

    /// 한 단계 위로 나온다. 최상위면 아무것도 하지 않는다.
    pub(in crate::app::canvas) fn leave(self: &Rc<Self>) -> bool {
        let Some(scope) = self.session.scope.get() else {
            return false;
        };
        let parent = self
            .state
            .upgrade()
            .and_then(|st| port::parent_of(&st.borrow(), scope));
        self.go_to(parent);
        true
    }

    /// 경로·바로가기·카드에서 다른 단계로 건너간다. 확대해 들어가거나 물러나며 화면을 바꾼다.
    pub(in crate::app::canvas) fn go_to(self: &Rc<Self>, scope: Option<Uuid>) {
        let from = self.session.scope.get();
        if from == scope
            || self.session.diving.get()
            || !self.session.interaction.borrow().is_idle()
        {
            return;
        }
        let Some(st) = self.state.upgrade() else {
            return;
        };
        if scope.is_some_and(|id| !port::exists(&st.borrow(), id)) {
            return;
        }
        let target_path = port::path(&st.borrow(), scope);
        let from_path = port::path(&st.borrow(), from);
        drop(st);
        let deeper = from.is_none_or(|f| target_path.iter().any(|p| p.id == f));
        // 지금 화면에서 목적지로 이어지는 카드. 그 카드로 파고들거나 그 카드에서 빠져나온다.
        let anchor = target_path.iter().find_map(|p| self.shown_rect(p.id));
        self.session
            .cameras
            .borrow_mut()
            .insert(from, self.session.camera.get());
        self.session.selection.borrow_mut().set([]);
        self.sync_selection();
        // 떠나는 화면을 한 장으로 굳힌다. 새 단계는 그 아래에서 바로 배치해 모션 전에 무거운 일을 끝낸다.
        let texture = motion::animated(self)
            .then(|| scene::dive::capture(self))
            .flatten();
        let Some(texture) = texture else {
            self.switch_scope(scope, &from_path, false);
            return;
        };
        let camera = self.session.camera.get();
        self.session.diving.set(true);
        let target = self.switch_scope(scope, &from_path, true);
        // 들어갈 때는 누른 카드가 화면이 되고, 나올 때는 화면이 방금 있던 프로젝트 카드로 줄어든다.
        let (kind, card, zoom) = if deeper {
            let card = anchor.and_then(|rect| scene::dive::card_box(self, camera, rect));
            (
                scene::dive::Kind::Enter { camera: target },
                card,
                camera.zoom,
            )
        } else {
            let card = from_path
                .iter()
                .find_map(|p| self.shown_rect(p.id))
                .and_then(|rect| scene::dive::card_box(self, target, rect));
            (scene::dive::Kind::Leave, card, target.zoom)
        };
        scene::dive::begin(self, texture, kind, card, zoom);
        let weak = Rc::downgrade(self);
        scene::dive::play(self, move || {
            if let Some(v) = weak.upgrade() {
                v.session.diving.set(false);
                v.select_came_from(&from_path);
                scene::board::update_port(&v);
            }
        });
    }

    /// 새 단계를 도착 카메라로 배치하고 그 카메라를 돌려준다. `animate`면 전환 모션은 호출한 쪽이 맡는다.
    fn switch_scope(
        self: &Rc<Self>,
        scope: Option<Uuid>,
        from_path: &[port::Place],
        animate: bool,
    ) -> Camera {
        motion::stop_camera(self);
        self.session.scope.set(scope);
        self.session.fitted.set(false);
        self.render();
        self.session.fitted.set(true);
        let size = motion::viewport(self);
        let remembered = self.session.cameras.borrow().get(&scope).copied();
        let rects: Vec<Rect> = self
            .session
            .nodes
            .borrow()
            .iter()
            .map(|n| n.rect())
            .collect();
        let target = remembered
            .or_else(|| layout::bounds(rects).map(|area| Camera::fitting(area, size, FIT_PADDING)))
            .unwrap_or_default();
        // 나와서 돌아간 단계에서는 방금 있던 프로젝트 카드가 보여야 어디서 나왔는지 이어진다.
        let came_from = from_path.iter().find_map(|p| self.shown_rect(p.id));
        let target = match came_from {
            Some(rect) if !target.shows(rect, size, FIT_PADDING) => {
                self.keep_camera(Camera::centered_on(rect, size, target.zoom))
            }
            _ => target,
        };
        if let Some(id) = scope {
            self.record_visit(id);
        }
        if !animate {
            scene::board::set_camera(self, target);
            self.select_came_from(from_path);
            self.render();
            return target;
        }
        scene::board::set_camera(self, target);
        self.render();
        target
    }

    /// 나온 단계에서 방금 있던 프로젝트를 선택해 둔다. 전환이 끝난 뒤에 해 모션 중 패널이 튀어나오지 않는다.
    fn select_came_from(self: &Rc<Self>, from_path: &[port::Place]) {
        if let Some(id) = from_path
            .last()
            .map(|p| p.id)
            .filter(|id| self.node(*id).is_some())
        {
            self.select_only(id);
        }
    }

    /// 방문 기록은 파일에 쓰므로 들어오는 모션이 끝난 뒤에 남긴다. 모션 중 메인 스레드 I/O는 프레임을 밀어낸다.
    fn record_visit(self: &Rc<Self>, id: Uuid) {
        let weak = Rc::downgrade(self);
        let delay = if motion::animated(self) { SETTLE_MS } else { 0 };
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
            let Some(v) = weak.upgrade() else {
                return;
            };
            if let Some(st) = v.state.upgrade() {
                st.borrow_mut()
                    .navigation
                    .record(id, crate::app::shared::unix_now());
            }
            v.sync_selection();
        });
    }

    /// 선택한 메모를 새 프로젝트로 묶는다. 프로젝트 메모는 선택 영역 왼쪽 위에 만든다.
    pub(in crate::app::canvas) fn group_selected(self: &Rc<Self>) {
        let selected = self.selected_nodes();
        let Some(st) = self.state.upgrade() else {
            return;
        };
        let Some(area) = layout::bounds(selected.iter().map(|n| n.1)) else {
            return;
        };
        if st.borrow().notebook.busy() || !self.session.interaction.borrow().is_idle() {
            return;
        }
        let ids = selected.into_iter().map(|n| n.0).collect();
        shell_commands::group_into_project(&st, ids, [area.x, area.y], self.session.scope.get());
    }

    /// 새 프로젝트 메모 안으로 옮긴다. 배치는 그대로 두어 안에서도 같은 모양으로 보인다.
    pub(in crate::app::canvas) fn adopt(self: &Rc<Self>, project: Uuid, children: Vec<Uuid>) {
        self.mutate(|notes| {
            for id in &children {
                port::set_parent(notes, *id, Some(project));
            }
        });
    }

    /// 메모들을 다른 프로젝트 안으로 옮긴다. 안쪽의 빈자리에 같은 모양을 유지한 채 놓는다.
    pub(in crate::app::canvas) fn move_into(self: &Rc<Self>, ids: Vec<Uuid>, target: Option<Uuid>) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        let rects: Vec<(Uuid, Rect)> = ids
            .iter()
            .filter_map(|id| self.shown_rect(*id).map(|r| (*id, r)))
            .collect();
        let Some(area) = layout::bounds(rects.iter().map(|r| r.1)) else {
            return;
        };
        let spot = port::vacant(&st.borrow().notebook, target);
        drop(st);
        let moves: Moves = rects
            .iter()
            .map(|(id, r)| (*id, [spot[0] + r.x - area.x, spot[1] + r.y - area.y]))
            .collect();
        self.mutate(|notes| {
            let moved: Moves = moves
                .into_iter()
                .filter(|(id, _)| port::set_parent(notes, *id, target))
                .collect();
            port::move_nodes(notes, &moved);
        });
    }

    /// 지금 프로젝트 밖(한 단계 위)으로 선택을 꺼낸다.
    pub(in crate::app::canvas) fn lift_selected(self: &Rc<Self>) {
        let Some(scope) = self.session.scope.get() else {
            return;
        };
        let parent = self
            .state
            .upgrade()
            .and_then(|st| port::parent_of(&st.borrow(), scope));
        self.move_into(self.selected_ids(), parent);
    }

    pub(in crate::app::canvas) fn toggle_pin(self: &Rc<Self>, id: Uuid) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        let pinned = !port::pinned(&st.borrow(), id);
        st.borrow_mut()
            .notebook
            .edit_canvas(|notes| port::set_pinned(notes, id, pinned));
        lifecycle::mark(&st);
        self.render();
    }
}
