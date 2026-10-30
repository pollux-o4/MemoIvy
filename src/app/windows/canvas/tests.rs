use super::*;
use crate::{
    app::state::AppState,
    persistence::{
        save_queue::SaveQueue,
        store::{Note, Store},
    },
};
use model::layout::Align;
use std::collections::HashMap;

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
    let menu = named(v.inspector.upcast_ref(), "canvas-control-connect")
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
    let settings = v.window.settings();
    let animated = settings.is_gtk_enable_animations();
    let dark = settings.is_gtk_application_prefer_dark_theme();
    settings.set_gtk_enable_animations(false);
    v.fit();
    for (name, is_dark) in [("light", false), ("dark", true)] {
        settings.set_gtk_application_prefer_dark_theme(is_dark);
        if let Some(id) = v.selection.borrow().single() {
            v.signal(id);
        }
        settle(100);
        let paintable = gtk::WidgetPaintable::new(Some(&v.window));
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(&snapshot, v.window.width() as f64, v.window.height() as f64);
        let node = snapshot.to_node().expect("rendered canvas scene");
        let texture = v.window.renderer().unwrap().render_texture(&node, None);
        texture
            .save_to_png(std::path::Path::new(&dir).join(format!("canvas-{name}.png")))
            .unwrap();
    }
    v.motion.borrow_mut().pulse = None;
    settings.set_gtk_application_prefer_dark_theme(dark);
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
    let library = ApplicationWindow::new(&app);
    let mut notes = vec![Note::new(), Note::new(), Note::new()];
    for (i, n) in notes.iter_mut().enumerate() {
        n.text = format!("# 메모 {i}\n한글과 긴 제목도 보존합니다");
        n.canvas.position = Some([64. + i as f64 * 320., 64. + i as f64 * 7.]);
    }
    let (a, b, c) = (notes[0].id, notes[1].id, notes[2].id);
    let state = Rc::new(RefCell::new(AppState {
        store: Store {
            path: path.clone(),
            notes,
        },
        queue: SaveQueue::new(path.clone()),
        canvas: None,
        windows: HashMap::new(),
        list: GtkBox::new(Orientation::Vertical, 0),
        count: Label::new(None),
        search: gtk::SearchEntry::new(),
        error: Label::new(None),
        library: library.clone(),
        changed_at: None,
        first_change: None,
        refresh_at: None,
        closing: Vec::new(),
        closing_library: false,
        bypass: false,
        operation: None,
        add: Button::new(),
    }));
    open(&state, &app, Some(a));
    drain();
    let v = state.borrow().canvas.clone().unwrap();
    v.window.set_visible(false);
    gtk::prelude::WidgetExt::unrealize(&v.window);
    v.window.set_default_size(820, 620);
    v.window.present();
    for _ in 0..200 {
        drain();
        if (760..=840).contains(&v.window.width()) && v.surface.width() > 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    settle(50);
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
    assert!(v.cards.borrow()[&a].root.get_visible());
    settle(50);
    assert!(
        (760..=840).contains(&v.window.width()),
        "small window must not be forced wide: {}",
        v.window.width()
    );
    assert_eq!(v.summary.text(), "메모 3 · 연결 0");
    assert!(
        scene::card::title_text(&v.cards.borrow()[&a]) == "메모 0",
        "cards hide markdown markers"
    );
    for card in v.cards.borrow().values() {
        let Some(box_size) = card.root.compute_bounds(&v.world) else {
            continue;
        };
        assert_eq!(f64::from(box_size.height()), port::CARD_HEIGHT);
        assert_eq!(f64::from(box_size.width()), port::CARD_WIDTH);
    }
    assert!(
        v.inspector.width() < 350,
        "selection must not consume the board: {}",
        v.inspector.width()
    );

    // 확대 전환은 기존 위젯을 유지하고 저장을 만들지 않는다.
    super::motion::tests::canvas_002_viewport(&v);
    scene::board::set_camera(&v, Camera::default());

    connect_through_menu(&v, b);
    assert_eq!(state.borrow().store.notes[0].canvas.next, vec![b]);
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
    let done = named(v.inspector.upcast_ref(), "canvas-control-done")
        .and_downcast::<gtk::Switch>()
        .expect("completion control");
    done.grab_focus();
    done.set_active(true);
    settle(10);
    assert_eq!(
        gtk::prelude::GtkWindowExt::focus(&v.window)
            .unwrap()
            .widget_name(),
        "canvas-control-done",
        "rebuilding the panel keeps keyboard focus"
    );
    assert!(state.borrow().store.notes[0].canvas.done);
    assert!(
        !state.borrow().store.notes[1].canvas.done,
        "signal never completes the target"
    );
    state.borrow_mut().store.notes[0].text = "원본 수정 📝".into();
    v.undo();
    assert!(!state.borrow().store.notes[0].canvas.done);
    assert_eq!(
        state.borrow().store.notes[0].text,
        "원본 수정 📝",
        "canvas undo preserves source edits"
    );
    v.redo();
    assert!(state.borrow().store.notes[0].canvas.done, "redo reapplies");
    v.undo();
    assert_eq!(state.borrow().store.notes[0].canvas.next, vec![b]);

    // 다중 선택 정렬: 배치만 바꾸고 한 번의 되돌리기로 복구한다.
    let settings = v.window.settings();
    let animated = settings.is_gtk_enable_animations();
    settings.set_gtk_enable_animations(false);
    let widget = v.cards.borrow()[&a].root.clone();
    v.selection.borrow_mut().set([a, b, c]);
    v.sync_selection();
    assert!(v.align_bar.reveals_child());
    assert!(
        !v.panel.reveals_child(),
        "multi-selection hides the single-note panel"
    );
    assert_eq!(v.align_count.text(), "3개 선택");
    assert!(v.distribute.iter().all(|button| button.is_sensitive()));
    input::keyboard::run(&v, input::keyboard::Command::Align(Align::Top));
    {
        let s = state.borrow();
        assert!(s
            .store
            .notes
            .iter()
            .all(|n| n.canvas.position.unwrap()[1] == 64.));
    }
    assert_eq!(
        v.cards.borrow()[&a].root,
        widget,
        "layout changes keep the same widget"
    );
    v.undo();
    assert_eq!(
        state.borrow().store.notes[2].canvas.position,
        Some([704., 78.])
    );
    settings.set_gtk_enable_animations(animated);

    // 방향키 이동은 선택 전체에 적용되고 스프링 중에도 목표는 즉시 저장된다.
    v.selection.borrow_mut().set([a]);
    v.sync_selection();
    v.cards.borrow()[&a].root.grab_focus();
    let controllers = v.window.observe_controllers();
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
        state.borrow().store.notes[0].canvas.position,
        Some([80., 64.])
    );
    settle(20);
    assert_eq!(
        gtk::prelude::GtkWindowExt::focus(&v.window).map(|w| w.widget_name().to_string()),
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
        v.selection.borrow().single(),
        Some(b),
        "Tab moves focus and selection to the next card"
    );
    v.cards.borrow()[&a].root.grab_focus();
    settle(20);
    v.undo();
    assert_eq!(
        state.borrow().store.notes[0].canvas.position,
        Some([64., 64.])
    );
    settings.set_gtk_enable_animations(false);
    settle(20);
    settings.set_gtk_enable_animations(animated);

    controller::dispatch(&mut state.borrow_mut());
    for _ in 0..200 {
        if let Some((_, result)) = state.borrow_mut().queue.poll() {
            result.unwrap();
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let loaded = Store::load(path).unwrap();
    assert_eq!(loaded.notes.len(), 3);
    assert_eq!(loaded.notes[0].canvas.next, vec![b]);

    // 삭제된 메모는 되살리지 않는다.
    crate::persistence::canvas::remove(&mut state.borrow_mut().store.notes, b);
    v.render();
    assert_eq!(v.cards.borrow().len(), 2);
    v.undo();
    assert_eq!(state.borrow().store.notes.len(), 2);
    assert!(
        state.borrow().store.notes[0].canvas.next.is_empty(),
        "undo must not restore deleted targets"
    );

    let failed_path = dir.path().join("blocked");
    std::fs::create_dir(&failed_path).unwrap();
    state.borrow_mut().queue = SaveQueue::new(failed_path);
    controller::mark(&state);
    controller::dispatch(&mut state.borrow_mut());
    for _ in 0..200 {
        if let Some((_, result)) = state.borrow_mut().queue.poll() {
            assert!(result.is_err());
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    v.update_status();
    assert!(v.retry.is_visible());
    assert!(v.status.text().contains("실패"));
    assert_eq!(state.borrow().store.notes[0].text, "원본 수정 📝");

    state.borrow_mut().store.notes.clear();
    v.render();
    assert_eq!(v.summary.text(), "메모 0 · 연결 0");
    assert!(v.empty.is_visible());
    assert!(v.cards.borrow().is_empty());
    assert_eq!(
        v.world.first_child().as_ref(),
        Some(v.port_handle.upcast_ref())
    );
    v.window.close();
    assert!(!v.window.is_visible());
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
    let library = ApplicationWindow::new(&app);
    let mut notes: Vec<Note> = (0..300).map(|_| Note::new()).collect();
    let ids: Vec<Uuid> = notes.iter().map(|n| n.id).collect();
    for (i, n) in notes.iter_mut().enumerate() {
        n.text = format!("# 메모 {i}\n- [x] 완료\n{}", "본문 내용 ".repeat(90));
        n.canvas.position = Some([(i % 20) as f64 * 300., (i / 20) as f64 * 200.]);
        if i % 3 != 0 && i < 299 {
            n.canvas.next.push(ids[i + 1]);
        }
    }
    let state = Rc::new(RefCell::new(AppState {
        store: Store {
            path: path.clone(),
            notes,
        },
        queue: SaveQueue::new(path),
        canvas: None,
        windows: HashMap::new(),
        list: GtkBox::new(Orientation::Vertical, 0),
        count: Label::new(None),
        search: gtk::SearchEntry::new(),
        error: Label::new(None),
        library: library.clone(),
        changed_at: None,
        first_change: None,
        refresh_at: None,
        closing: Vec::new(),
        closing_library: false,
        bypass: false,
        operation: None,
        add: Button::new(),
    }));
    open(&state, &app, None);
    settle(600);
    let v = state.borrow().canvas.clone().unwrap();
    let measure = |name: &str, seconds: f64, step: &mut dyn FnMut(f64)| {
        let frames = Rc::new(RefCell::new(Vec::<Instant>::new()));
        let log = frames.clone();
        let tick = v.surface.add_tick_callback(move |_, _| {
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
    v.selection.borrow_mut().set(ids.iter().copied());
    v.sync_selection();
    input::keyboard::run(&v, input::keyboard::Command::Flow);
    measure("flow-300-cards", 1.2, &mut |_| {});
    v.undo();
    let camera = v.camera.get();
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
