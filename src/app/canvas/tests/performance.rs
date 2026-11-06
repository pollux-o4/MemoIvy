use super::*;

/// PERF-002 프레임 측정. 릴리스 빌드에서 `MEMOIVY_CANVAS_PERF=1`일 때만 실행한다.
pub(in crate::app) fn perf_002_frames() {
    if std::env::var("MEMOIVY_CANVAS_PERF").is_err() {
        return;
    }
    use std::time::Instant;
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let app = Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.CanvasPerf")
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
    println!(
        "PERF-002 renderer={}",
        v.widgets.window.renderer().unwrap().type_().name()
    );
    let display = gtk::gdk::Display::default().unwrap();
    let surface = v.widgets.window.surface().unwrap();
    if let Some(monitor) = display.monitor_at_surface(&surface) {
        println!("PERF-002 monitor-refresh={}mHz", monitor.refresh_rate());
    }
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
    // shortcut: Surface 이벤트 생성 API가 없어 병합 경로를 직접 측정, 실제 Wayland 터치패드 확보 시 입력 포함 재측정.
    for _ in 0..100 {
        v.queue_camera(v.session.camera.get().panned(0.25, 0.125));
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
            v.queue_camera(v.session.camera.get().panned(direction, direction * 0.5));
        }
    });
    scene::board::set_camera(&v, Camera::default());
    let mut notch = 0;
    measure("wheel-notches", 2., &mut |t| {
        let next = (t / 0.08) as usize;
        while notch < next {
            let direction = if notch < 12 { 1. } else { -1. };
            scroll.emit_by_name::<bool>("scroll", &[&0., &direction]);
            notch += 1;
        }
    });
    motion::stop_camera(&v);
    scene::board::set_camera(&v, Camera::default());
    // 1000Hz 마우스 휠 클릭 끌기: 프레임마다 여러 움직임이 몰려도 한 번만 갱신한다.
    *v.session.interaction.borrow_mut() = Interaction::Pan { last: (0., 0.) };
    let mut moved = 0;
    measure("middle-pan", 2., &mut |t| {
        let next = (t * 1000.) as usize;
        while moved < next {
            let direction = if moved < 1000 { 1. } else { -1. };
            input::pointer::pan_by(&v, (direction * 0.6, direction * 0.3));
            moved += 1;
        }
    });
    v.session.interaction.replace(Interaction::Idle);
    scene::paint::sync_hud(&v);
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
    // 프로젝트 들어가기·나오기 전환. 처음 들어갈 때(카드 생성)와 다시 들어갈 때(재사용)를 나눠 본다.
    v.move_into(ids[1..40].to_vec(), Some(ids[0]));
    drain();
    if let Ok(dir) = std::env::var("MEMOIVY_DIVE_FRAMES") {
        dive_frames(&v, ids[0], std::path::Path::new(&dir));
    }
    for name in ["dive-first", "dive-again"] {
        motion::stop_camera(&v);
        let (mut entered, mut left) = (false, false);
        measure(name, 1.6, &mut |t| {
            if !entered {
                entered = true;
                let began = Instant::now();
                v.enter(ids[0]);
                println!(
                    "PERF-002 {name}-click: {:.1}ms",
                    began.elapsed().as_secs_f64() * 1000.
                );
            } else if t > 0.8 && !left && !v.session.diving.get() {
                left = true;
                v.leave();
            }
        });
        until(|| v.session.scope.get().is_none() && !v.session.diving.get());
    }
    v.undo();
    let renderer = v.widgets.window.renderer().unwrap();
    let viewport = gtk::graphene::Rect::new(0., 0., 1920., 1080.);
    for zoom in [1., 0.2, 0.05] {
        scene::board::set_camera(
            &v,
            Camera {
                zoom,
                ..Camera::default()
            },
        );
        let camera = v.session.camera.get();
        let began = Instant::now();
        for frame in 0..30 {
            scene::board::set_camera(&v, camera.panned(frame as f64 * 0.37, frame as f64 * 0.21));
            let snapshot = gtk::Snapshot::new();
            scene::paint::draw_backdrop(&v, &snapshot, 1920., 1080.);
            let _ = renderer.render_texture(snapshot.to_node().unwrap(), Some(&viewport));
        }
        println!(
            "PERF-002 backdrop-1080p: {:.2}ms per frame (zoom {:.2})",
            began.elapsed().as_secs_f64() * 1000. / 30.,
            camera.zoom
        );
    }
    v.destroy();
    library.destroy();
    drain();
}

/// 전환을 고정 지점마다 창 전체로 렌더링해 PNG로 남긴다. 모션 모양을 눈으로 검토할 때 쓴다.
fn dive_frames(v: &Rc<CanvasView>, project: Uuid, dir: &std::path::Path) {
    std::fs::create_dir_all(dir).unwrap();
    let renderer = v.widgets.window.renderer().unwrap();
    let shot = |name: String| {
        let (w, h) = (v.widgets.window.width(), v.widgets.window.height());
        let paintable = gtk::WidgetPaintable::new(Some(&v.widgets.window));
        let node = loop {
            settle(40);
            let snapshot = gtk::Snapshot::new();
            paintable.snapshot(&snapshot, f64::from(w), f64::from(h));
            if let Some(node) = snapshot.to_node() {
                break node;
            }
        };
        let texture = renderer.render_texture(
            node,
            Some(&gtk::graphene::Rect::new(0., 0., w as f32, h as f32)),
        );
        texture.save_to_png(dir.join(name)).unwrap();
    };
    // 누른 카드가 화면 안에 있어야 카드에서 화면으로 이어지는 모양을 볼 수 있다.
    if let Some(rect) = v.shown_rect(project) {
        let size = motion::viewport(v);
        scene::board::set_camera(v, Camera::centered_on(rect, size, 0.35));
        settle(100);
    }
    scene::dive::HOLD.with(|hold| hold.set(true));
    for (name, enter) in [("enter", true), ("leave", false)] {
        if enter {
            v.enter(project);
        } else {
            v.leave();
        }
        for step in 0..=8 {
            scene::dive::pose(v, f64::from(step) / 8.);
            shot(format!("{name}-{step}.png"));
        }
        scene::dive::HOLD.with(|hold| hold.set(false));
        until(|| !v.session.diving.get());
        scene::dive::HOLD.with(|hold| hold.set(true));
        shot(format!("{name}-end.png"));
        settle(100);
    }
    scene::dive::HOLD.with(|hold| hold.set(false));
}
