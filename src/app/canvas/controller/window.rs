use super::*;

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
    actions.append(&add);

    let root = Overlay::new();
    root.add_css_class("canvas-root");
    let backdrop = scene::paint::backdrop::Backdrop::default();
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
    let freeze = scene::paint::backdrop::Backdrop::default();
    freeze.set_can_target(false);
    freeze.set_visible(false);
    root.add_overlay(&freeze);
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
    root.add_overlay(&floating.navigator.root);
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
            freeze,
            panel: floating.panel,
            inspector: floating.inspector,
            connect_menu: RefCell::new(None),
            align_bar: floating.align_bar,
            align_count: floating.align_count,
            distribute: floating.distribute,
            empty: floating.empty,
            empty_title: floating.empty_title,
            empty_sub: floating.empty_sub,
            status: floating.status,
            summary: floating.summary,
            retry: floating.retry,
            dock: floating.dock,
            undo: floating.undo,
            redo: floating.redo,
            zoom_label: floating.zoom_label,
            navigator: floating.navigator,
            project_actions: floating.project_actions,
            cards: RefCell::new(HashMap::new()),
            parked: RefCell::new(HashMap::new()),
        },
        session: BoardSession {
            scope: Cell::new(None),
            cameras: RefCell::new(HashMap::new()),
            diving: Cell::new(false),
            dive: RefCell::new(None),
            navigator_key: RefCell::new(String::new()),
            inspector_key: RefCell::new(String::new()),
            nodes: RefCell::new(Vec::new()),
            content: Cell::new(None),
            shown: RefCell::new(HashMap::new()),
            selection: RefCell::new(Selection::default()),
            camera: Cell::new(Camera::default()),
            camera_queued: Cell::new(false),
            scroll_panning: Cell::new(false),
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
            edge_tiles: RefCell::new(scene::paint::edge_tiles::EdgeTiles::default()),
        },
        motion: RefCell::new(motion::MotionState::default()),
        history: RefCell::new(model::history::History::new(HISTORY_LIMIT)),
    });
    if let Some(id) = selected {
        view.session.selection.borrow_mut().set([id]);
    }
    let weak = Rc::downgrade(&view);
    add.connect_clicked(move |_| {
        if let Some(v) = weak.upgrade() {
            if let Some(st) = v.state.upgrade() {
                shell_commands::new_note(&st, v.session.scope.get());
            }
        }
    });
    scene::paint::attach(&view);
    scene::dive::attach(&view);
    input::attach(&view);
    panels::wire(&view, floating.actions);
    let weak = Rc::downgrade(state);
    window.connect_close_request(move |w| {
        if let Some(st) = weak.upgrade() {
            st.borrow().views.library.window.present();
            lifecycle::flush(&st);
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

/// 열린 캔버스에서 프로젝트 안으로 들어간다. 첫 배치가 끝나기 전이면 전환 없이 바로 바꾼다.
pub(in crate::app) fn show_project(state: &State, project: Uuid) {
    let view = state.borrow().views.canvas.clone();
    let Some(view) = view else {
        return;
    };
    if view.session.fitted.get() {
        view.go_to(Some(project));
        return;
    }
    // 아직 창 크기가 없으니 단계만 바꾸고 첫 배치가 그 단계를 맞추게 한다.
    view.session.scope.set(Some(project));
    state
        .borrow_mut()
        .navigation
        .record(project, crate::app::shared::unix_now());
    view.render();
}

/// 단축키를 받은 창이 캔버스면 지금 보는 프로젝트를 돌려준다.
pub(in crate::app) fn scope_for(state: &State, window: &gtk::Window) -> Option<Uuid> {
    let view = state.borrow().views.canvas.clone()?;
    (view.widgets.window.upcast_ref::<gtk::Window>() == window)
        .then(|| view.session.scope.get())
        .flatten()
}

/// 새 프로젝트 메모가 만들어지면 묶을 메모를 그 안으로 옮긴다. 캔버스가 있으면 되돌리기 이력에 남긴다.
pub(in crate::app) fn adopt(state: &State, project: Uuid, children: Vec<Uuid>) {
    let view = state.borrow().views.canvas.clone();
    match view {
        Some(view) => view.adopt(project, children),
        None => {
            state.borrow_mut().notebook.edit_canvas(|notes| {
                for id in children {
                    port::set_parent(notes, id, Some(project));
                }
            });
        }
    }
}

pub(in crate::app) fn update_status(state: &State) {
    let view = state.borrow().views.canvas.clone();
    if let Some(view) = view {
        view.update_status();
    }
}
