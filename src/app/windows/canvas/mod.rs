mod board;
mod inspector;
mod interaction;
mod motion;
#[cfg(test)]
pub(in crate::app) mod tests;

use crate::app::{
    controller,
    state::{AppState, State},
    widgets::header,
};
use gtk::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, Fixed, Label,
    Orientation, Overlay, ScrolledWindow,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Instant,
};
use uuid::Uuid;

pub(in crate::app) struct CanvasView {
    window: ApplicationWindow,
    state: Weak<RefCell<AppState>>,
    app: Application,
    fixed: Fixed,
    drawing: DrawingArea,
    scroll: ScrolledWindow,
    inspector: GtkBox,
    status: Label,
    summary: Label,
    retry: Button,
    undo: Button,
    zoom_label: Label,
    selected: Cell<Option<Uuid>>,
    zoom: Cell<f64>,
    dragging: Cell<bool>,
    pulse: Cell<Option<(Uuid, Instant)>>,
    zoom_transition: Cell<Option<motion::ZoomTransition>>,
    frame_active: Cell<bool>,
    history: RefCell<Vec<Vec<(Uuid, crate::persistence::canvas::CanvasNode)>>>,
}

pub(in crate::app) fn open(state: &State, app: &Application, selected: Option<Uuid>) {
    if let Some(view) = &state.borrow().canvas {
        if selected.is_some() {
            view.selected.set(selected);
        }
        view.render();
        view.window.present();
        view.reveal();
        return;
    }
    let window = ApplicationWindow::builder()
        .application(app)
        .title("메모 캔버스")
        .default_width(1160)
        .default_height(760)
        .build();
    window.set_transient_for(Some(&state.borrow().library));
    window.set_destroy_with_parent(true);
    super::focus::watch(&window);
    super::shortcuts::attach(&window, state);
    let (_, actions) = header(&window, "메모 캔버스");
    let add = Button::with_label("새 메모");
    add.add_css_class("new-note");
    let st = Rc::downgrade(state);
    add.connect_clicked(move |_| {
        if let Some(st) = st.upgrade() {
            let add = st.borrow().add.clone();
            add.emit_clicked();
        }
    });
    actions.append(&add);
    let root = GtkBox::new(Orientation::Vertical, 0);
    root.add_css_class("canvas-root");
    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.add_css_class("canvas-toolbar");
    let summary = Label::new(None);
    summary.set_hexpand(true);
    summary.set_xalign(0.);
    toolbar.append(&summary);
    let undo = Button::with_label("되돌리기");
    undo.set_tooltip_text(Some("캔버스 배치·연결·완료 변경 되돌리기"));
    toolbar.append(&undo);
    let home = Button::with_label("전체 보기");
    toolbar.append(&home);
    let minus = Button::with_label("−");
    minus.set_tooltip_text(Some("축소"));
    minus.update_property(&[gtk::accessible::Property::Label("축소")]);
    toolbar.append(&minus);
    let zoom_label = Label::new(Some("100%"));
    zoom_label.set_width_chars(5);
    toolbar.append(&zoom_label);
    let plus = Button::with_label("+");
    plus.set_tooltip_text(Some("확대"));
    plus.update_property(&[gtk::accessible::Property::Label("확대")]);
    toolbar.append(&plus);
    root.append(&toolbar);
    let content = GtkBox::new(Orientation::Horizontal, 0);
    content.set_vexpand(true);
    let overlay = Overlay::new();
    let drawing = DrawingArea::new();
    drawing.set_hexpand(true);
    drawing.set_vexpand(true);
    overlay.set_child(Some(&drawing));
    let fixed = Fixed::new();
    let scroll = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&fixed)
        .build();
    overlay.add_overlay(&scroll);
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);
    content.append(&overlay);
    let inspector = GtkBox::new(Orientation::Vertical, 12);
    inspector.set_size_request(244, -1);
    inspector.add_css_class("canvas-inspector");
    let side = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::External)
        .min_content_width(284)
        .max_content_width(284)
        .propagate_natural_width(false)
        .child(&inspector)
        .build();
    side.set_hexpand(false);
    content.append(&side);
    root.append(&content);
    let status = Label::new(None);
    status.set_xalign(0.);
    status.set_wrap(true);
    status.add_css_class("canvas-status");
    let status_row = GtkBox::new(Orientation::Horizontal, 8);
    status.set_hexpand(true);
    status_row.append(&status);
    let retry = Button::with_label("저장 다시 시도");
    retry.set_visible(false);
    let st = Rc::downgrade(state);
    retry.connect_clicked(move |_| {
        if let Some(st) = st.upgrade() {
            let mut s = st.borrow_mut();
            s.queue.retry();
            controller::dispatch(&mut s);
        }
    });
    status_row.append(&retry);
    root.append(&status_row);
    window.set_child(Some(&root));
    let view = Rc::new(CanvasView {
        window: window.clone(),
        state: Rc::downgrade(state),
        app: app.clone(),
        fixed,
        drawing,
        scroll,
        inspector,
        status,
        summary,
        retry,
        undo: undo.clone(),
        zoom_label,
        selected: Cell::new(selected),
        zoom: Cell::new(1.),
        dragging: Cell::new(false),
        pulse: Cell::new(None),
        zoom_transition: Cell::new(None),
        frame_active: Cell::new(false),
        history: RefCell::new(Vec::new()),
    });
    board::attach(&view);
    for (button, factor) in [(minus, 0.8), (plus, 1.25)] {
        let weak = Rc::downgrade(&view);
        button.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                motion::zoom_by(&v, factor);
            }
        });
    }
    let weak = Rc::downgrade(&view);
    home.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            v.fit();
        }
    });
    let weak = Rc::downgrade(&view);
    undo.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            if let Some(st) = v.state.upgrade() {
                if st.borrow().operation.is_some() {
                    return;
                }
                let snapshot = v.history.borrow_mut().pop();
                if let Some(snapshot) = snapshot {
                    for note in &mut st.borrow_mut().store.notes {
                        if let Some((_, node)) = snapshot.iter().find(|(id, _)| *id == note.id) {
                            note.canvas = node.clone();
                        }
                    }
                    let ids: Vec<_> = st.borrow().store.notes.iter().map(|n| n.id).collect();
                    for note in &mut st.borrow_mut().store.notes {
                        note.canvas.next.retain(|id| ids.contains(id));
                    }
                    controller::mark(&st);
                    v.render();
                }
            }
        }
    });
    let weak = Rc::downgrade(state);
    window.connect_close_request(move |w| {
        if let Some(st) = weak.upgrade() {
            let mut s = st.borrow_mut();
            s.library.present();
            if !s.queue.clean() && s.operation.is_none() {
                s.queue.retry();
                controller::dispatch(&mut s);
            }
        }
        w.hide();
        gtk::glib::Propagation::Stop
    });
    state.borrow_mut().canvas = Some(view.clone());
    view.render();
    window.present();
    view.reveal();
}

pub(in crate::app) fn refresh(state: &State) {
    let view = state.borrow().canvas.clone();
    if let Some(view) = view {
        if !view.dragging.get() {
            view.render();
        }
    }
}

pub(in crate::app) fn update_status(state: &State) {
    let view = state.borrow().canvas.clone();
    if let Some(view) = view {
        view.update_status();
    }
}

impl CanvasView {
    pub(in crate::app) fn destroy(&self) {
        self.window.destroy();
    }
    fn update_status(&self) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        let s = st.borrow();
        self.retry.set_visible(s.queue.failed());
        self.undo
            .set_sensitive(!self.history.borrow().is_empty() && s.operation.is_none());
        self.status.set_text(if s.queue.failed() {
            "저장 실패 · 다시 시도로 재저장하세요. 메모와 배치를 유지하고 있어요."
        } else if !s.queue.clean() {
            "저장 중…"
        } else {
            "자동 저장됨 · 스크롤로 탐색 · 이동 핸들 선택 후 방향키로 배치"
        });
    }

    fn checkpoint(&self) {
        if let Some(st) = self.state.upgrade() {
            let mut history = self.history.borrow_mut();
            if history.len() == 40 {
                history.remove(0);
            }
            history.push(
                st.borrow()
                    .store
                    .notes
                    .iter()
                    .map(|n| (n.id, n.canvas.clone()))
                    .collect(),
            );
        }
    }

    fn mutate(self: &Rc<Self>, action: impl FnOnce(&mut Vec<crate::persistence::store::Note>)) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        if st.borrow().operation.is_some() {
            return;
        }
        self.checkpoint();
        action(&mut st.borrow_mut().store.notes);
        controller::mark(&st);
        self.render();
    }

    fn select(self: &Rc<Self>, id: Uuid) {
        self.selected.set(Some(id));
        let mut child = self.fixed.first_child();
        while let Some(card) = child {
            if card.widget_name() == id.to_string() {
                card.add_css_class("selected");
            } else {
                card.remove_css_class("selected");
            }
            child = card.next_sibling();
        }
        inspector::render(self);
        self.drawing.queue_draw();
    }

    fn defer_render(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        gtk::glib::idle_add_local_once(move || {
            if let Some(v) = weak.upgrade() {
                v.render();
            }
        });
    }

    fn render(self: &Rc<Self>) {
        self.update_status();
        board::render(self);
        inspector::render(self);
    }

    fn signal(self: &Rc<Self>, id: Uuid) {
        motion::signal(self, id);
    }

    fn reveal(&self) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        let s = st.borrow();
        if let Some((i, note)) = s
            .store
            .notes
            .iter()
            .enumerate()
            .find(|(_, n)| Some(n.id) == self.selected.get())
        {
            let [x, y] = crate::persistence::canvas::position(note, i);
            for (adjustment, value) in [
                (self.scroll.hadjustment(), x),
                (self.scroll.vadjustment(), y),
            ] {
                adjustment.set_value((value * self.zoom.get() - 64.).max(0.));
            }
        }
    }

    fn fit(self: &Rc<Self>) {
        let Some(st) = self.state.upgrade() else {
            return;
        };
        let (mut width, mut height) = (600_f64, 400_f64);
        for (i, n) in st.borrow().store.notes.iter().enumerate() {
            let [x, y] = crate::persistence::canvas::position(n, i);
            width = width.max(x + crate::persistence::canvas::WIDTH + 64.);
            height = height.max(y + crate::persistence::canvas::HEIGHT + 64.);
        }
        let zoom = (self.scroll.width() as f64 / width)
            .min(self.scroll.height() as f64 / height)
            .clamp(0.02, 1.);
        motion::transition(self, [zoom, 0., 0.]);
    }
}
