//! 메모 캔버스 프런트엔드 모듈. 화면 API는 `open`·`refresh`·`update_status`·`CanvasView::destroy`다.
//! 초기 배치·빈자리 계산은 shell 명령에서 사용하는 보조 진입점이다.
//!
//! - `model`: GTK 없는 규칙 (카메라·정렬·선택·스프링)
//! - `scene`: 보드 위 그림과 카드 위젯
//! - `input`: 포인터·키보드 해석
//! - `panels`: 떠 있는 도구 (독·정렬 막대·선택 패널)
//! - `motion`: 프레임 구동 / `port`: 저장 계층과의 유일한 경계
mod input;
mod model;
mod motion;
mod panels;
mod port;
mod scene;
#[cfg(test)]
pub(in crate::app) mod tests;

use crate::app::{
    shared::header,
    shell::{
        commands, lifecycle as controller,
        state::{AppState, State},
    },
};
use crate::presentation::icons;
use gtk::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, Fixed, Label,
    Orientation, Overlay, Revealer,
};
use input::Interaction;
use model::{
    camera::Camera,
    layout::{self, Guide, Moves, Rect},
    selection::Selection,
};
use port::NodeView;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};
use uuid::Uuid;

const HISTORY_LIMIT: usize = 40;
const FIT_PADDING: f64 = 72.;
const MIN_BOARD: (i32, i32) = (560, 360);

pub(in crate::app) struct CanvasView {
    state: Weak<RefCell<AppState>>,
    app: Application,
    widgets: CanvasWidgets,
    session: BoardSession,
    motion: RefCell<motion::MotionState>,
    history: RefCell<model::history::History<port::Snapshot>>,
}

struct CanvasWidgets {
    window: ApplicationWindow,
    surface: Overlay,
    stage: Fixed,
    world: Fixed,
    port_handle: GtkBox,
    backdrop: DrawingArea,
    hud: DrawingArea,
    panel: Revealer,
    inspector: GtkBox,
    align_bar: Revealer,
    align_count: Label,
    distribute: [Button; 2],
    empty: GtkBox,
    status: Label,
    summary: Label,
    retry: Button,
    dock: GtkBox,
    undo: Button,
    redo: Button,
    zoom_label: Button,
    cards: RefCell<HashMap<Uuid, scene::card::Card>>,
}

struct BoardSession {
    inspector_key: RefCell<String>,
    nodes: RefCell<Vec<NodeView>>,
    content: Cell<Option<Rect>>,
    shown: RefCell<HashMap<Uuid, [f64; 2]>>,
    selection: RefCell<Selection>,
    camera: Cell<Camera>,
    camera_queued: Cell<bool>,
    fitted: Cell<bool>,
    interaction: RefCell<Interaction>,
    guides: RefCell<Vec<Guide>>,
    pointer: Cell<(f64, f64)>,
    space: Cell<bool>,
    port_owner: Cell<Option<Uuid>>,
    last_click: Cell<Option<(Uuid, u32)>>,
    drag_modifiers: Cell<(bool, bool)>,
    drag_styled: Cell<bool>,
    presenting: Cell<bool>,
    summaries: RefCell<port::SummaryCache>,
}

pub(in crate::app) fn open(state: &State, app: &Application, selected: Option<Uuid>) {
    let existing = state.borrow().views.canvas.clone();
    if let Some(view) = existing {
        if let Some(id) = selected {
            view.session.selection.borrow_mut().set([id]);
        }
        view.render();
        view.present();
        view.reveal();
        return;
    }
    let window = ApplicationWindow::builder()
        .application(app)
        .title("메모 캔버스")
        .default_width(1160)
        .default_height(760)
        .build();
    window.set_transient_for(Some(&state.borrow().views.library.window));
    window.set_destroy_with_parent(true);
    crate::app::shared::focus::watch(&window);
    crate::app::shared::shortcuts::attach(&window, state);
    let (_, actions) = header(&window, "메모 캔버스");
    let add = Button::new();
    let add_contents = GtkBox::new(Orientation::Horizontal, 4);
    add_contents.append(&icons::image(icons::ADD));
    add_contents.append(&Label::new(Some("새 메모")));
    add.set_child(Some(&add_contents));
    add.add_css_class("new-note");
    add.set_tooltip_text(Some("새 메모 · Ctrl+N"));
    let st = Rc::downgrade(state);
    add.connect_clicked(move |_| {
        if let Some(st) = st.upgrade() {
            commands::new_note(&st);
        }
    });
    actions.append(&add);

    let root = Overlay::new();
    root.add_css_class("canvas-root");
    let backdrop = DrawingArea::new();
    backdrop.add_css_class("canvas-backdrop");
    backdrop.set_hexpand(true);
    backdrop.set_vexpand(true);
    root.set_child(Some(&backdrop));
    // Overlay는 주 자식만 측정한다. 멀리 놓인 카드가 창 최소 크기를 키우지 못하게 막는다.
    let surface = Overlay::new();
    surface.set_child(Some(&GtkBox::new(Orientation::Horizontal, 0)));
    let stage = Fixed::new();
    stage.set_overflow(gtk::Overflow::Visible);
    surface.add_overlay(&stage);
    surface.add_css_class("canvas-surface");
    surface.set_overflow(gtk::Overflow::Hidden);
    surface.set_hexpand(true);
    surface.set_vexpand(true);
    surface.set_size_request(MIN_BOARD.0, MIN_BOARD.1);
    let world = Fixed::new();
    // GtkFixed는 기본으로 자기 영역을 잘라 가장자리 카드의 그림자·핸들이 사라진다.
    world.set_overflow(gtk::Overflow::Visible);
    stage.put(&world, 0., 0.);
    root.add_overlay(&surface);
    let hud = DrawingArea::new();
    hud.set_can_target(false);
    hud.set_hexpand(true);
    hud.set_vexpand(true);
    root.add_overlay(&hud);
    let port_handle = GtkBox::new(Orientation::Horizontal, 0);
    port_handle.add_css_class("canvas-port");
    port_handle.set_size_request(scene::card::PORT_SIZE as i32, scene::card::PORT_SIZE as i32);
    port_handle.set_visible(false);
    port_handle.set_cursor_from_name(Some("crosshair"));
    port_handle.set_tooltip_text(Some("끌어서 다음 메모에 연결"));
    world.put(&port_handle, 0., 0.);

    let floating = panels::build(state);
    root.add_overlay(&floating.dock_row);
    root.add_overlay(&floating.align_bar);
    root.add_overlay(&floating.panel);
    root.add_overlay(&floating.empty);
    window.set_child(Some(&root));

    let view = Rc::new(CanvasView {
        state: Rc::downgrade(state),
        app: app.clone(),
        widgets: CanvasWidgets {
            window: window.clone(),
            surface,
            stage,
            world,
            port_handle,
            backdrop,
            hud,
            panel: floating.panel,
            inspector: floating.inspector,
            align_bar: floating.align_bar,
            align_count: floating.align_count,
            distribute: floating.distribute,
            empty: floating.empty,
            status: floating.status,
            summary: floating.summary,
            retry: floating.retry,
            dock: floating.dock,
            undo: floating.undo,
            redo: floating.redo,
            zoom_label: floating.zoom_label,
            cards: RefCell::new(HashMap::new()),
        },
        session: BoardSession {
            inspector_key: RefCell::new(String::new()),
            nodes: RefCell::new(Vec::new()),
            content: Cell::new(None),
            shown: RefCell::new(HashMap::new()),
            selection: RefCell::new(Selection::default()),
            camera: Cell::new(Camera::default()),
            camera_queued: Cell::new(false),
            fitted: Cell::new(false),
            interaction: RefCell::new(Interaction::Idle),
            guides: RefCell::new(Vec::new()),
            pointer: Cell::new((0., 0.)),
            space: Cell::new(false),
            port_owner: Cell::new(None),
            last_click: Cell::new(None),
            drag_modifiers: Cell::new((false, false)),
            drag_styled: Cell::new(false),
            presenting: Cell::new(false),
            summaries: RefCell::new(port::SummaryCache::default()),
        },
        motion: RefCell::new(motion::MotionState::default()),
        history: RefCell::new(model::history::History::new(HISTORY_LIMIT)),
    });
    if let Some(id) = selected {
        view.session.selection.borrow_mut().set([id]);
    }
    scene::paint::attach(&view);
    input::attach(&view);
    panels::wire(&view, floating.actions);
    let weak = Rc::downgrade(state);
    window.connect_close_request(move |w| {
        if let Some(st) = weak.upgrade() {
            st.borrow().views.library.window.present();
            controller::flush(&st);
        }
        w.set_visible(false);
        gtk::glib::Propagation::Stop
    });
    state.borrow_mut().views.canvas = Some(view.clone());
    view.render();
    view.present();
}

pub(in crate::app) fn refresh(state: &State) {
    let view = state.borrow().views.canvas.clone();
    if let Some(view) = view {
        // 숨겨진 캔버스는 입력마다 다시 계산하지 않고 다시 열 때 맞춘다.
        if view.session.interaction.borrow().is_idle() && view.widgets.window.is_visible() {
            view.render();
        }
    }
}

pub(in crate::app) fn update_status(state: &State) {
    let view = state.borrow().views.canvas.clone();
    if let Some(view) = view {
        view.update_status();
    }
}

impl CanvasView {
    fn keep_camera(&self, camera: Camera) -> Camera {
        let viewport = (
            f64::from(self.widgets.surface.width()),
            f64::from(self.widgets.surface.height()),
        );
        if viewport.0 < 1. || viewport.1 < 1. {
            return camera;
        }
        self.session
            .content
            .get()
            .map_or(camera, |area| camera.kept_near(area, viewport))
    }

    fn queue_camera(self: &Rc<Self>, camera: Camera) {
        // 입력 좌표는 즉시 누적하고 카드·변환 갱신은 프레임당 한 번만 한다.
        self.session.camera.set(self.keep_camera(camera));
        if self.session.camera_queued.replace(true) {
            return;
        }
        let weak = Rc::downgrade(self);
        self.widgets.surface.add_tick_callback(move |_, _| {
            if let Some(view) = weak.upgrade() {
                view.session.camera_queued.set(false);
                scene::board::set_camera(&view, view.session.camera.get());
                input::pointer::follow(&view);
            }
            gtk::glib::ControlFlow::Break
        });
    }

    /// GTK는 창을 띄울 때 첫 위젯에 포커스를 준다. 그 때문에 카드가 저절로 선택되지 않게 한다.
    fn present(&self) {
        self.session.presenting.set(true);
        self.widgets.window.present();
        gtk::prelude::GtkWindowExt::set_focus(&self.widgets.window, None::<&gtk::Widget>);
        self.session.presenting.set(false);
    }

    pub(in crate::app) fn destroy(&self) {
        self.widgets.window.destroy();
    }

    fn update_status(&self) {
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

    /// 배치·연결·완료 변경의 단일 진입점. 실제로 바뀐 경우에만 기록하고 저장을 요청한다.
    fn mutate(self: &Rc<Self>, action: impl FnOnce(&mut port::Notes<'_>)) {
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
        controller::mark(&st);
        self.render();
    }

    fn undo(self: &Rc<Self>) {
        self.step_history(true);
    }

    fn redo(self: &Rc<Self>) {
        self.step_history(false);
    }

    fn step_history(self: &Rc<Self>, backwards: bool) {
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
        controller::mark(&st);
        self.render();
    }

    fn select_only(self: &Rc<Self>, id: Uuid) {
        self.session.selection.borrow_mut().set([id]);
        self.sync_selection();
    }

    fn sync_selection(self: &Rc<Self>) {
        let selection = self.session.selection.borrow().clone();
        for (id, card) in self.widgets.cards.borrow().iter() {
            card.set_selected(selection.contains(*id));
        }
        panels::sync(self);
        scene::board::update_port(self);
        self.widgets.backdrop.queue_draw();
    }

    fn render(self: &Rc<Self>) {
        self.update_status();
        scene::board::render(self);
        panels::sync(self);
        scene::board::update_port(self);
    }

    fn signal(self: &Rc<Self>, id: Uuid) {
        motion::signal(self, id);
    }

    fn node(&self, id: Uuid) -> Option<NodeView> {
        self.session
            .nodes
            .borrow()
            .iter()
            .find(|n| n.id == id)
            .cloned()
    }

    fn shown_rect(&self, id: Uuid) -> Option<Rect> {
        self.session
            .shown
            .borrow()
            .get(&id)
            .map(|p| port::rect_at(*p))
    }

    fn selected_nodes(&self) -> Vec<layout::Node> {
        let selection = self.session.selection.borrow();
        self.session
            .nodes
            .borrow()
            .iter()
            .filter(|n| selection.contains(n.id))
            .map(|n| (n.id, n.rect()))
            .collect()
    }

    fn reveal(self: &Rc<Self>) {
        let rects: Vec<Rect> = self
            .selected_nodes()
            .into_iter()
            .map(|node| node.1)
            .collect();
        if let Some(area) = layout::bounds(rects) {
            motion::ensure_visible(self, area);
        }
    }

    fn fit(self: &Rc<Self>) {
        let rects = self
            .session
            .nodes
            .borrow()
            .iter()
            .map(|n| n.rect())
            .collect::<Vec<_>>();
        self.fit_rects(rects);
    }

    fn fit_selection(self: &Rc<Self>) {
        let rects: Vec<Rect> = self
            .selected_nodes()
            .into_iter()
            .map(|node| node.1)
            .collect();
        self.fit_rects(rects);
    }

    fn fit_rects(self: &Rc<Self>, rects: Vec<Rect>) {
        let Some(area) = layout::bounds(rects) else {
            return;
        };
        let viewport = (
            f64::from(self.widgets.surface.width()),
            f64::from(self.widgets.surface.height()),
        );
        motion::camera_to(self, Camera::fitting(area, viewport, FIT_PADDING));
    }

    fn apply_moves(self: &Rc<Self>, moves: Moves) {
        if moves.is_empty() {
            return;
        }
        self.mutate(|notes| port::move_nodes(notes, &moves));
        self.reveal();
    }

    fn toggle_done(self: &Rc<Self>, id: Uuid) {
        let Some(node) = self.node(id) else {
            return;
        };
        let done = !node.done;
        self.mutate(|notes| port::set_done(notes, id, done));
        if done && !node.next.is_empty() {
            self.signal(id);
        }
    }

    fn connect_nodes(self: &Rc<Self>, from: Uuid, to: Uuid) {
        if !port::can_connect(&self.session.nodes.borrow(), from, to) {
            return;
        }
        self.mutate(|notes| {
            port::connect(notes, from, to);
        });
        motion::link_added(self, from, to);
    }

    fn open_note(self: &Rc<Self>, id: Uuid) {
        if let Some(st) = self.state.upgrade() {
            commands::open_note(&st, &self.app, id);
        }
    }

    fn restore_cursor(&self) {
        self.widgets
            .surface
            .set_cursor_from_name(self.session.space.get().then_some("grab"));
    }

    fn resized(self: &Rc<Self>) {
        scene::board::set_camera(self, self.session.camera.get());
        panels::sync(self);
    }

    fn first_layout(self: &Rc<Self>) {
        if self.session.fitted.replace(true) {
            return;
        }
        let rects = self
            .session
            .nodes
            .borrow()
            .iter()
            .map(|n| n.rect())
            .collect::<Vec<_>>();
        let selected: Vec<Rect> = self
            .selected_nodes()
            .into_iter()
            .map(|node| node.1)
            .collect();
        let viewport = (
            f64::from(self.widgets.surface.width()),
            f64::from(self.widgets.surface.height()),
        );
        if let Some(area) = layout::bounds(rects) {
            let mut camera = Camera::fitting(area, viewport, FIT_PADDING);
            if let Some(focus) = layout::bounds(selected) {
                if !camera.shows(focus, viewport, 24.) {
                    camera = Camera::centered_on(focus, viewport, 1.);
                }
            }
            scene::board::set_camera(self, camera);
        }
    }
}

pub(in crate::app) fn vacant_position(notebook: &port::Notebook) -> [f64; 2] {
    port::vacant(notebook)
}

pub(in crate::app) fn initialize_positions(notebook: &mut port::Notebook) {
    port::initialize_positions(notebook);
}
