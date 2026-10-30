use super::*;
use crate::{
    app::shell::state::AppState,
    persistence::{Note, Notebook, SaveStatus},
};
use model::layout::Align;

fn note(text: String, position: [f64; 2], next: Vec<Uuid>, id: Uuid) -> Note {
    serde_json::from_value(serde_json::json!({
        "id": id, "text": text, "color": "yellow", "modified": 1,
        "canvas": { "position": position, "next": next, "done": false }
    }))
    .unwrap()
}

fn scroll_controller(v: &CanvasView) -> gtk::EventControllerScroll {
    let controllers = v.widgets.surface.observe_controllers();
    (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerScroll>()
        })
        .expect("canvas scroll controller")
}

fn touchpad_burst(v: &Rc<CanvasView>) {
    scene::board::set_camera(v, Camera::default());
    let scroll = scroll_controller(v);
    let child = v
        .widgets
        .stage
        .layout_manager()
        .unwrap()
        .layout_child(&v.widgets.world);
    let updates = Rc::new(Cell::new(0));
    let count = updates.clone();
    let handler =
        child.connect_notify_local(Some("transform"), move |_, _| count.set(count.get() + 1));
    // shortcut: GTK Wheel을 60으로 정규화해 연속 입력 재현, 실제 Wayland 터치패드 환경에서 실측 재검증.
    for (dx, dy, expected) in [
        (0.25, 0.125, (25., 12.5)),
        (-0.25, 0.125, (0., 25.)),
        (-0.25, -0.125, (-25., 12.5)),
        (0.25, -0.125, (0., 0.)),
    ] {
        updates.set(0);
        let start = std::time::Instant::now();
        for _ in 0..100 {
            scroll.emit_by_name::<bool>("scroll", &[&(dx / 60.), &(dy / 60.)]);
        }
        println!(
            "TOUCHPAD burst=100 viewport_updates={} handler_ms={:.3}",
            updates.get(),
            start.elapsed().as_secs_f64() * 1000.
        );
        assert_eq!(
            updates.get(),
            0,
            "scroll must wait for the frame before updating widgets"
        );
        settle(50);
        assert!((v.session.camera.get().x - expected.0).abs() < 0.001);
        assert!((v.session.camera.get().y - expected.1).abs() < 0.001);
        assert_eq!(updates.get(), 1);
        assert!(
            !v.session.camera_queued.get(),
            "one-shot callback must stop at idle"
        );
    }
    v.queue_camera(Camera {
        x: 0.25,
        y: 0.125,
        zoom: 1.,
    });
    settle(50);
    let (tx, ty) = v
        .widgets
        .stage
        .child_transform(&v.widgets.world)
        .unwrap()
        .to_translate();
    assert!(
        (tx + 0.25).abs() < 0.001 && (ty + 0.125).abs() < 0.001,
        "high-refresh slow movement must preserve subpixel translation: {tx}, {ty}"
    );
    // 대기 중 다른 조작이 카메라를 바꿔도 오래된 스크롤 위치를 재생하지 않는다.
    scroll.emit_by_name::<bool>("scroll", &[&0.25, &0.125]);
    scene::board::set_camera(v, Camera::default());
    settle(50);
    assert_eq!(v.session.camera.get(), Camera::default());
    child.disconnect(handler);
    scene::board::set_camera(v, Camera::default());
}

fn wait_notebook(state: &State) {
    for _ in 0..1000 {
        state.borrow_mut().notebook.tick(std::time::Instant::now());
        if !state.borrow().notebook.needs_tick() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("notebook writer timed out");
}

fn drain() {
    let context = gtk::glib::MainContext::default();
    while context.pending() {
        context.iteration(false);
    }
}

fn settle(ms: u64) {
    for _ in 0..ms / 5 {
        drain();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn named(widget: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
    if widget.widget_name() == name {
        return Some(widget.clone());
    }
    let mut child = widget.first_child();
    while let Some(w) = child {
        if let Some(found) = named(&w, name) {
            return Some(found);
        }
        child = w.next_sibling();
    }
    None
}

fn connect_through_menu(v: &Rc<CanvasView>, target: Uuid) {
    let menu = named(v.widgets.inspector.upcast_ref(), "canvas-control-connect")
        .and_downcast::<gtk::MenuButton>()
        .expect("connection menu");
    let popover = menu.popover().expect("connection list");
    let row = named(popover.upcast_ref(), &format!("canvas-connect-{target}"))
        .and_downcast::<Button>()
        .expect("candidate row");
    row.emit_clicked();
}

fn capture(v: &Rc<CanvasView>) {
    let Ok(dir) = std::env::var("LSM_CANVAS_ARTIFACT_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let settings = v.widgets.window.settings();
    let animated = settings.is_gtk_enable_animations();
    let dark = settings.is_gtk_application_prefer_dark_theme();
    let theme = settings.gtk_theme_name();
    let scheme = settings
        .find_property("gtk-interface-color-scheme")
        .map(|_| settings.property_value("gtk-interface-color-scheme"));
    settings.set_gtk_theme_name(Some("Adwaita"));
    settings.set_gtk_enable_animations(false);
    v.fit();
    for (name, is_dark) in [("light", false), ("dark", true)] {
        settings.set_gtk_application_prefer_dark_theme(is_dark);
        if let Some(scheme) = &scheme {
            let class = gtk::glib::EnumClass::with_type(scheme.type_()).unwrap();
            let value = class
                .to_value_by_nick(if is_dark { "dark" } else { "light" })
                .unwrap();
            settings.set_property_from_value("gtk-interface-color-scheme", &value);
        }
        if let Some(id) = v.session.selection.borrow().single() {
            v.signal(id);
        }
        settle(100);
        let paintable = gtk::WidgetPaintable::new(Some(&v.widgets.window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(
            &snapshot,
            v.widgets.window.width() as f64,
            v.widgets.window.height() as f64,
        );
        let node = snapshot.to_node().expect("rendered canvas scene");
        let texture = v
            .widgets
            .window
            .renderer()
            .unwrap()
            .render_texture(&node, None);
        texture
            .save_to_png(std::path::Path::new(&dir).join(format!("canvas-{name}.png")))
            .unwrap();
    }
    v.motion.borrow_mut().pulse = None;
    settings.set_gtk_application_prefer_dark_theme(dark);
    settings.set_gtk_theme_name(theme.as_deref());
    if let Some(scheme) = scheme {
        settings.set_property_from_value("gtk-interface-color-scheme", &scheme);
    }
    settings.set_gtk_enable_animations(animated);
}

// Run on the existing GTK test thread; GTK cannot be initialized on parallel test threads.
pub(in crate::app) fn canvas_001_widget_flow_and_recovery() {
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let app = Application::builder()
        .application_id("io.github.lsm.CanvasTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let notes = (0..3)
        .map(|i| {
            note(
                format!("# 메모 {i}\n한글과 긴 제목도 보존합니다"),
                [64. + i as f64 * 320., 64. + i as f64 * 7.],
                Vec::new(),
                Uuid::new_v4(),
            )
        })
        .collect::<Vec<_>>();
    let (a, b, c) = (notes[0].id(), notes[1].id(), notes[2].id());
    let state = AppState::fixture(path.clone(), &app, notes);
    let library = state.borrow().views.library.window.clone();
    open(&state, &app, Some(a));
    drain();
    let v = state.borrow().views.canvas.clone().unwrap();
    v.widgets.window.set_visible(false);
    gtk::prelude::WidgetExt::unrealize(&v.widgets.window);
    v.widgets.window.set_default_size(820, 620);
    v.widgets.window.present();
    for _ in 0..200 {
        drain();
        if (760..=840).contains(&v.widgets.window.width()) && v.widgets.surface.width() > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    settle(50);
    touchpad_burst(&v);
    // 화면 밖 카드는 측정에서 빠지고 다시 들어오면 나타난다.
    scene::board::set_camera(
        &v,
        Camera {
            x: 5000.,
            y: 5000.,
            zoom: 1.,
        },
    );
    scene::board::set_camera(&v, Camera::default());
    assert!(v.widgets.cards.borrow()[&a].root.get_visible());
    settle(50);
    assert!(
        (760..=840).contains(&v.widgets.window.width()),
        "small window must not be forced wide: {}",
        v.widgets.window.width()
    );
    assert_eq!(v.widgets.summary.text(), "메모 3 · 연결 0");
    assert!(
        scene::card::title_text(&v.widgets.cards.borrow()[&a]) == "메모 0",
        "cards hide markdown markers"
    );
    for card in v.widgets.cards.borrow().values() {
        if !card.root.get_visible() {
            continue;
        }
        let Some(box_size) = card.root.compute_bounds(&v.widgets.world) else {
            continue;
        };
        assert_eq!(f64::from(box_size.height()), port::CARD_HEIGHT);
        assert_eq!(f64::from(box_size.width()), port::CARD_WIDTH);
    }
    assert!(
        v.widgets.inspector.width() < 350,
        "selection must not consume the board: {}",
        v.widgets.inspector.width()
    );

    // 확대 전환은 기존 위젯을 유지하고 저장을 만들지 않는다.
    super::motion::tests::canvas_002_viewport(&v);
    scene::board::set_camera(&v, Camera::default());

    connect_through_menu(&v, b);
    assert_eq!(state.borrow().notebook.notes()[0].canvas().next, vec![b]);
    let before_noop = v.history.borrow().len();
    v.connect_nodes(a, b);
    v.connect_nodes(a, a);
    assert_eq!(
        v.history.borrow().len(),
        before_noop,
        "rejected links leave no undo step"
    );
    capture(&v);

    // 완료는 대상에 전파되지 않고, 포커스를 유지하며, 되돌릴 수 있다.
    let done = named(v.widgets.inspector.upcast_ref(), "canvas-control-done")
        .and_downcast::<gtk::Switch>()
        .expect("completion control");
    done.grab_focus();
    done.set_active(true);
    settle(10);
    assert_eq!(
        gtk::prelude::GtkWindowExt::focus(&v.widgets.window)
            .unwrap()
            .widget_name(),
        "canvas-control-done",
        "rebuilding the panel keeps keyboard focus"
    );
    assert!(state.borrow().notebook.notes()[0].canvas().done);
    assert!(
        !state.borrow().notebook.notes()[1].canvas().done,
        "signal never completes the target"
    );
    state
        .borrow_mut()
        .notebook
        .set_text(a, "원본 수정 📝".into());
    v.undo();
    assert!(!state.borrow().notebook.notes()[0].canvas().done);
    assert_eq!(
        state.borrow().notebook.notes()[0].text(),
        "원본 수정 📝",
        "canvas undo preserves source edits"
    );
    v.redo();
    assert!(
        state.borrow().notebook.notes()[0].canvas().done,
        "redo reapplies"
    );
    v.undo();
    assert_eq!(state.borrow().notebook.notes()[0].canvas().next, vec![b]);

    // 다중 선택 정렬: 배치만 바꾸고 한 번의 되돌리기로 복구한다.
    let settings = v.widgets.window.settings();
    let animated = settings.is_gtk_enable_animations();
    settings.set_gtk_enable_animations(false);
    let widget = v.widgets.cards.borrow()[&a].root.clone();
    v.session.selection.borrow_mut().set([a, b, c]);
    v.sync_selection();
    assert!(v.widgets.align_bar.reveals_child());
    assert!(
        !v.widgets.panel.reveals_child(),
        "multi-selection hides the single-note panel"
    );
    assert_eq!(v.widgets.align_count.text(), "3개 선택");
    assert!(v
        .widgets
        .distribute
        .iter()
        .all(|button| button.is_sensitive()));
    input::keyboard::run(&v, input::keyboard::Command::Align(Align::Top));
    {
        let s = state.borrow();
        assert!(s
            .notebook
            .notes()
            .iter()
            .all(|n| n.canvas().position.unwrap()[1] == 64.));
    }
    assert_eq!(
        v.widgets.cards.borrow()[&a].root,
        widget,
        "layout changes keep the same widget"
    );
    v.undo();
    assert_eq!(
        state.borrow().notebook.notes()[2].canvas().position,
        Some([704., 78.])
    );
    settings.set_gtk_enable_animations(animated);

    // 방향키 이동은 선택 전체에 적용되고 스프링 중에도 목표는 즉시 저장된다.
    v.session.selection.borrow_mut().set([a]);
    v.sync_selection();
    v.widgets.cards.borrow()[&a].root.grab_focus();
    let controllers = v.widgets.window.observe_controllers();
    let stopped = (0..controllers.n_items())
        .filter_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .filter(|c| c.propagation_phase() == gtk::PropagationPhase::Capture)
        .any(|keys| {
            keys.emit_by_name::<bool>(
                "key-pressed",
                &[
                    &gtk::gdk::Key::Right,
                    &0u32,
                    &gtk::gdk::ModifierType::empty(),
                ],
            )
        });
    assert!(stopped);
    assert_eq!(
        state.borrow().notebook.notes()[0].canvas().position,
        Some([80., 64.])
    );
    settle(20);
    assert_eq!(
        gtk::prelude::GtkWindowExt::focus(&v.widgets.window).map(|w| w.widget_name().to_string()),
        Some(a.to_string()),
        "the card itself holds keyboard focus while it moves"
    );
    let stopped = (0..controllers.n_items())
        .filter_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .filter(|c| c.propagation_phase() == gtk::PropagationPhase::Capture)
        .any(|keys| {
            keys.emit_by_name::<bool>(
                "key-pressed",
                &[&gtk::gdk::Key::Tab, &0u32, &gtk::gdk::ModifierType::empty()],
            )
        });
    assert!(stopped, "Tab stays on the board while cards remain");
    settle(20);
    assert_eq!(
        v.session.selection.borrow().single(),
        Some(b),
        "Tab moves focus and selection to the next card"
    );
    v.widgets.cards.borrow()[&a].root.grab_focus();
    settle(20);
    v.undo();
    assert_eq!(
        state.borrow().notebook.notes()[0].canvas().position,
        Some([64., 64.])
    );
    settings.set_gtk_enable_animations(false);
    settle(20);
    settings.set_gtk_enable_animations(animated);

    state.borrow_mut().notebook.flush();
    wait_notebook(&state);
    let loaded = Notebook::open(path.clone()).unwrap();
    assert_eq!(loaded.notes().len(), 3);
    assert_eq!(loaded.notes()[0].canvas().next, vec![b]);

    // 삭제된 메모는 되살리지 않는다.
    state.borrow_mut().notebook.request_delete(b);
    state.borrow_mut().notebook.flush();
    wait_notebook(&state);
    v.render();
    assert_eq!(v.widgets.cards.borrow().len(), 2);
    v.undo();
    assert_eq!(state.borrow().notebook.notes().len(), 2);
    assert!(
        state.borrow().notebook.notes()[0].canvas().next.is_empty(),
        "undo must not restore deleted targets"
    );

    let current = state.borrow().notebook.notes().to_vec();
    let failed_path = dir.path().join("blocked").join("notes.json");
    std::fs::create_dir(failed_path.parent().unwrap()).unwrap();
    std::fs::write(&failed_path, serde_json::to_vec(&current).unwrap()).unwrap();
    state.borrow_mut().notebook = Notebook::open(failed_path.clone()).unwrap();
    std::fs::remove_file(&failed_path).unwrap();
    std::fs::remove_dir(failed_path.parent().unwrap()).unwrap();
    std::fs::write(failed_path.parent().unwrap(), b"blocked").unwrap();
    state.borrow_mut().notebook.set_color(a, "pink");
    state.borrow_mut().notebook.flush();
    wait_notebook(&state);
    assert!(matches!(
        state.borrow().notebook.status(),
        SaveStatus::Failed(_)
    ));
    v.update_status();
    assert!(v.widgets.retry.is_visible());
    assert!(v.widgets.status.text().contains("실패"));
    assert_eq!(state.borrow().notebook.notes()[0].text(), "원본 수정 📝");

    let empty_path = dir.path().join("empty.json");
    state.borrow_mut().notebook = Notebook::open(empty_path).unwrap();
    v.render();
    assert_eq!(v.widgets.summary.text(), "메모 0 · 연결 0");
    assert!(v.widgets.empty.is_visible());
    assert!(v.widgets.cards.borrow().is_empty());
    assert_eq!(
        v.widgets.world.first_child().as_ref(),
        Some(v.widgets.port_handle.upcast_ref())
    );
    v.widgets.window.close();
    assert!(!v.widgets.window.is_visible());
    v.destroy();
    library.destroy();
    drain();
    assert!(app.windows().is_empty());
}

/// PERF-002 프레임 측정. 릴리스 빌드에서 `LSM_CANVAS_PERF=1`일 때만 실행한다.
pub(in crate::app) fn perf_002_frames() {
    if std::env::var("LSM_CANVAS_PERF").is_err() {
        return;
    }
    use std::time::Instant;
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let app = Application::builder()
        .application_id("io.github.lsm.CanvasPerf")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let ids: Vec<Uuid> = (0..300).map(|_| Uuid::new_v4()).collect();
    let notes = (0..300)
        .map(|i| {
            note(
                format!("# 메모 {i}\n- [x] 완료\n{}", "본문 내용 ".repeat(90)),
                [(i % 20) as f64 * 300., (i / 20) as f64 * 200.],
                if i % 3 != 0 && i < 299 {
                    vec![ids[i + 1]]
                } else {
                    Vec::new()
                },
                ids[i],
            )
        })
        .collect();
    let state = AppState::fixture(path, &app, notes);
    let library = state.borrow().views.library.window.clone();
    // 실제 시작처럼 목록을 먼저 준비해 첫 카드 생성 비용이 프레임 표본에 섞이지 않게 한다.
    crate::app::shell::views::refresh_all(&state, &app);
    open(&state, &app, None);
    settle(600);
    let v = state.borrow().views.canvas.clone().unwrap();
    // shortcut: Xvfb 소프트웨어 렌더링은 변경 전에도 PERF-002 절대 기준 미달, 실제 GPU 환경 확보 시 재측정.
    let measure = |name: &str, seconds: f64, step: &mut dyn FnMut(f64)| {
        let frames = Rc::new(RefCell::new(Vec::<Instant>::new()));
        let log = frames.clone();
        let tick = v.widgets.surface.add_tick_callback(move |_, _| {
            log.borrow_mut().push(Instant::now());
            gtk::glib::ControlFlow::Continue
        });
        let begin = Instant::now();
        let context = gtk::glib::MainContext::default();
        while begin.elapsed().as_secs_f64() < seconds {
            step(begin.elapsed().as_secs_f64());
            context.iteration(true);
        }
        tick.remove();
        let frames = frames.borrow();
        let mut gaps: Vec<f64> = frames
            .windows(2)
            .map(|w| w[1].duration_since(w[0]).as_secs_f64() * 1000.)
            .collect();
        gaps.sort_by(f64::total_cmp);
        let p = |q: f64| gaps[((gaps.len() - 1) as f64 * q) as usize];
        println!(
            "PERF-002 {name}: frames={} fps={:.0} p50={:.1}ms p95={:.1}ms max={:.1}ms",
            gaps.len(),
            gaps.len() as f64 / seconds,
            p(0.5),
            p(0.95),
            p(1.)
        );
    };
    scene::board::set_camera(&v, Camera::default());
    settle(100);
    let scroll = scroll_controller(&v);
    let burst_started = Instant::now();
    for _ in 0..100 {
        scroll.emit_by_name::<bool>("scroll", &[&(0.25 / 60.), &(0.125 / 60.)]);
    }
    println!(
        "PERF-002 touchpad-input-100: {:.3}ms",
        burst_started.elapsed().as_secs_f64() * 1000.
    );
    settle(50);
    scene::board::set_camera(&v, Camera::default());
    measure("touchpad-burst", 2., &mut |t| {
        let direction = if t < 1. { 1. } else { -1. };
        for _ in 0..16 {
            scroll.emit_by_name::<bool>("scroll", &[&(direction / 60.), &(direction * 0.5 / 60.)]);
        }
    });
    scene::board::set_camera(&v, Camera::default());
    measure("pan-100%", 2., &mut |t| {
        scene::board::set_camera(
            &v,
            Camera {
                x: t * 300.,
                y: t * 120.,
                zoom: 1.,
            },
        );
    });
    measure("zoom-sweep", 2., &mut |t| {
        let zoom = 0.1 + (t / 2.) * 0.9;
        scene::board::set_camera(&v, Camera { x: 0., y: 0., zoom });
    });
    let fit = Camera {
        x: 0.,
        y: 0.,
        zoom: 0.2,
    };
    measure("pan-20%", 2., &mut |t| {
        scene::board::set_camera(&v, Camera { x: t * 300., ..fit });
    });
    scene::board::set_camera(&v, Camera::default());
    v.session.selection.borrow_mut().set(ids.iter().copied());
    v.sync_selection();
    input::keyboard::run(&v, input::keyboard::Command::Flow);
    measure("flow-300-cards", 1.2, &mut |_| {});
    v.undo();
    let camera = v.session.camera.get();
    let surface = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, 1920, 1080).unwrap();
    let cr = gtk::cairo::Context::new(&surface).unwrap();
    let began = Instant::now();
    for _ in 0..30 {
        scene::paint::draw_backdrop(&v, &cr, 1920., 1080.);
    }
    println!(
        "PERF-002 backdrop-1080p: {:.2}ms per paint (zoom {:.2})",
        began.elapsed().as_secs_f64() * 1000. / 30.,
        camera.zoom
    );
    v.destroy();
    library.destroy();
    drain();
}
