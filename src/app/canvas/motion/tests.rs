use super::*;
use std::time::Duration;

#[test]
fn canvas_002_coast_is_short_bounded_and_refresh_independent() {
    let simulate = |hz: usize, jitter: bool| {
        let mut coast = Coast::new((1000., 0.));
        let (mut x, mut time) = (0., 0.);
        let mut frame = 0;
        while let Some(current) = coast {
            let factor = if jitter {
                if frame % 2 == 0 {
                    0.8
                } else {
                    1.2
                }
            } else {
                1.
            };
            let elapsed = factor / hz as f64;
            let (next, delta) = current.advance(elapsed);
            x += delta.0;
            time += elapsed;
            coast = next;
            frame += 1;
            assert!(time < 0.35, "coast must promptly stop at {hz}Hz");
        }
        x
    };
    for hz in [60, 120, 144] {
        for jitter in [false, true] {
            assert!((simulate(hz, jitter) - 80.).abs() < 1e-8);
        }
    }
    assert!(
        Coast::new((100., 0.)).is_none(),
        "gentle release has no added tail"
    );
    assert!(Coast::new((f64::NAN, 0.)).is_none());
    let (next, distance) = Coast::new((100_000., 0.)).unwrap().advance(1.);
    assert!(next.is_none());
    assert!(
        distance.0 < 100.,
        "fast flick or stalled frame cannot launch the board"
    );
}

#[test]
fn canvas_002_signal_is_bounded_and_reduced_motion_is_static() {
    assert_eq!(ease(-1.), 0.);
    assert_eq!(ease(2.), 1.);
    let samples: Vec<_> = (0..=100).map(|i| ease(i as f64 / 100.)).collect();
    assert!(samples.windows(2).all(|s| s[0] <= s[1]));
    assert_eq!(pulse_sample(0., true).0, 0.);
    assert_eq!(pulse_sample(SIGNAL_SECONDS, true), (1., 0.));
    assert_eq!(pulse_sample(0., false), (1., 1.));
    assert_eq!(link_progress(LINK_SECONDS), 1.);
}

pub(in crate::app) fn canvas_002_viewport(v: &Rc<CanvasView>) {
    let settings = v.widgets.window.settings();
    let original = settings.is_gtk_enable_animations();
    settings.set_gtk_enable_animations(true);
    let card = v.widgets.world.first_child().unwrap();
    let begin = Instant::now();
    let goal = |zoom| Camera { x: 0., y: 0., zoom };
    stop_camera(v);
    v.session.camera.set(goal(1.));
    camera_to(v, goal(0.6));
    v.motion.borrow_mut().last = Some(begin);
    assert!(advance(v, begin + Duration::from_millis(90)));
    let middle = v.session.camera.get().zoom;
    assert!(middle > 0.6 && middle < 1., "zoom interpolates: {middle}");
    camera_to(v, goal(0.8));
    assert!(!advance(v, begin + Duration::from_millis(1800)));
    assert!((v.session.camera.get().zoom - 0.8).abs() < 1e-6);
    assert_eq!(
        v.widgets.world.first_child().unwrap(),
        card,
        "zoom preserves native widgets and focus"
    );
    settings.set_gtk_enable_animations(false);
    camera_to(v, goal(1.));
    assert_eq!(v.session.camera.get().zoom, 1.);
    assert!(v.motion.borrow().camera.is_none());
    settings.set_gtk_enable_animations(true);
    scene::board::set_camera(v, goal(1.));
    inertia(v, (1000., 0.));
    let release = Instant::now();
    v.motion.borrow_mut().last = None;
    assert!(advance(v, release));
    assert_eq!(
        v.session.camera.get().x,
        0.,
        "no assumed 60Hz step on first frame"
    );
    let mut active = true;
    for frame in 1..=42 {
        active = advance(v, release + Duration::from_secs_f64(frame as f64 / 120.));
    }
    assert!(!active, "trackpad coast must finish within 350ms at 120Hz");
    assert!(
        v.session.camera.get().x < 110.,
        "trackpad coast must stay within 110 screen pixels"
    );
    inertia(v, (1000., 0.));
    let selected = v.session.selection.borrow().ids().to_vec();
    let keys = v.widgets.window.observe_controllers();
    let keys = (0..keys.n_items())
        .filter_map(|i| keys.item(i).and_downcast::<gtk::EventControllerKey>())
        .find(|keys| keys.propagation_phase() == gtk::PropagationPhase::Capture)
        .unwrap();
    let stopped: bool = keys.emit_by_name(
        "key-pressed",
        &[
            &gtk::gdk::Key::Escape,
            &0u32,
            &gtk::gdk::ModifierType::empty(),
        ],
    );
    assert!(
        stopped,
        "Escape consumes cancellation of active camera motion"
    );
    assert!(
        v.motion.borrow().inertia.is_none(),
        "Escape must stop touchpad coast before the next frame"
    );
    v.session.selection.borrow_mut().set([]);
    inertia(v, (1000., 0.));
    let stopped: bool = keys.emit_by_name(
        "key-pressed",
        &[
            &gtk::gdk::Key::Escape,
            &0u32,
            &gtk::gdk::ModifierType::empty(),
        ],
    );
    assert!(!stopped, "empty-board Escape keeps existing propagation");
    assert!(v.motion.borrow().inertia.is_none());
    assert!(v.widgets.window.is_visible());
    v.session.selection.borrow_mut().set(selected);
    v.sync_selection();
    inertia(v, (1000., 0.));
    let controllers = v.widgets.surface.observe_controllers();
    let scroll = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerScroll>()
        })
        .unwrap();
    scroll.emit_by_name::<()>("scroll-begin", &[]);
    assert!(
        v.motion.borrow().inertia.is_none(),
        "finger contact interrupts inertia before the next delta"
    );
    stop_camera(v);
    scene::board::set_camera(v, goal(1.).panned(1e9, 0.));
    inertia(v, (1000., 0.));
    let edge = Instant::now();
    v.motion.borrow_mut().last = Some(edge);
    assert!(
        !advance(v, edge + Duration::from_secs_f64(1. / 120.)),
        "blocked coast stops at the board boundary"
    );
    scene::board::set_camera(v, goal(1.));
    inertia(v, (1000., 0.));
    settings.set_gtk_enable_animations(false);
    v.motion.borrow_mut().last = Some(edge);
    assert!(!advance(v, edge + Duration::from_secs_f64(1. / 120.)));
    assert_eq!(
        v.session.camera.get().x,
        0.,
        "reduced motion cancels coast without one last jump"
    );
    stop_camera(v);
    settings.set_gtk_enable_animations(original);
}
