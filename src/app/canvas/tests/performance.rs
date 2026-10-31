use super::*;

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
    let initial_camera = v.session.camera.get();
    let initial_pointer = v.session.pointer.get();
    scene::board::set_camera(&v, Camera::default());
    let controllers = v.widgets.surface.observe_controllers();
    let pointer = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerMotion>()
        })
        .unwrap();
    for zoom in [1., 0.2] {
        scene::board::set_camera(
            &v,
            Camera {
                zoom,
                ..Camera::default()
            },
        );
        for burst in [200, 1000] {
            settle(40);
            let pointer_started = std::time::Instant::now();
            for event in 0..burst {
                pointer.emit_by_name::<()>("motion", &[&(event as f64 / 10.), &-100.]);
            }
            println!(
                "PERF-002 pointer-input-{burst} zoom={zoom:.1}: {:.3}ms",
                pointer_started.elapsed().as_secs_f64() * 1000.
            );
        }
    }
    settle(40);
    v.session.pointer.set(initial_pointer);
    scene::board::set_camera(&v, initial_camera);
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
