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
    let st = Rc::downgrade(state);
    add.connect_clicked(move |_| {
        if let Some(st) = st.upgrade() {
            shell_commands::new_note(&st);
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
            connect_menu: RefCell::new(None),
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

pub(in crate::app) fn update_status(state: &State) {
    let view = state.borrow().views.canvas.clone();
    if let Some(view) = view {
        view.update_status();
    }
}
