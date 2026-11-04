use super::*;
use crate::app::test_support::{button_with_class, drain, named, settle, until};
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
    until(|| v.session.fitted.get());
    motion::stop_camera(v);
    scene::board::set_camera(v, Camera::default());
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
    // Surface 입력의 프레임 병합 경로. 단위별 분기는 pointer 테스트에서 확인한다.
    for (dx, dy, expected) in [
        (0.25, 0.125, (25., 12.5)),
        (-0.25, 0.125, (0., 25.)),
        (-0.25, -0.125, (-25., 12.5)),
        (0.25, -0.125, (0., 0.)),
    ] {
        updates.set(0);
        let start = std::time::Instant::now();
        for _ in 0..100 {
            v.queue_camera(v.session.camera.get().panned(dx, dy));
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
        until(|| !v.session.camera_queued.get());
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
    until(|| !v.session.camera_queued.get());
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
    v.queue_camera(v.session.camera.get().panned(0.25, 0.125));
    scene::board::set_camera(v, Camera::default());
    settle(50);
    assert_eq!(v.session.camera.get(), Camera::default());
    child.disconnect(handler);
    scene::board::set_camera(v, Camera::default());
}

fn pointer_hit_testing(v: &Rc<CanvasView>, id: Uuid, other: Uuid) {
    until(|| v.session.fitted.get() && v.widgets.surface.is_mapped());
    let selected = v.session.selection.borrow().ids().to_vec();
    // 창 재매핑의 기본 카드 포커스가 선택을 되살리지 않게 툴바로 옮긴다.
    v.widgets.zoom_label.grab_focus();
    until(|| v.widgets.zoom_label.is_focus());
    v.session.selection.borrow_mut().set([]);
    v.session.pointer.set((-10_000., -10_000.));
    scene::board::update_port(v);
    let controllers = v.widgets.surface.observe_controllers();
    let pointer = (0..controllers.n_items())
        .find_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerMotion>()
        })
        .unwrap();
    let position = v.session.shown.borrow()[&id];
    let point = v
        .session
        .camera
        .get()
        .to_screen((position[0] + 10., position[1] + 10.));
    for event in 0..1000 {
        let x = if event == 999 { point.0 } else { -1000. };
        pointer.emit_by_name::<()>("motion", &[&x, &point.1]);
    }
    assert_eq!(
        v.session.pointer.get(),
        point,
        "input coordinates stay immediate"
    );
    assert_eq!(
        v.session.port_owner.get(),
        Some(id),
        "latest hover owner stays immediate"
    );
    until(|| {
        v.session.port_owner.get() == Some(id)
            && v.widgets.port_handle.is_mapped()
            && v.widgets.port_handle.width() > 0
    });
    // 이탈은 즉시 반영되고 다음 프레임에도 되살아나지 않는다.
    pointer.emit_by_name::<()>("motion", &[&point.0, &point.1]);
    pointer.emit_by_name::<()>("leave", &[]);
    assert_eq!(v.session.port_owner.get(), None);
    settle(40);
    assert_eq!(v.session.port_owner.get(), None);
    // GTK는 숨긴 위젯의 크기를 0으로 만든다. 배치 후 실제 hit target을 검증한다.
    let handle_point = v.session.camera.get().to_screen((
        position[0] + port::CARD_WIDTH - scene::card::PORT_SIZE / 4.,
        position[1] + port::CARD_HEIGHT / 2.,
    ));
    pointer.emit_by_name::<()>("motion", &[&handle_point.0, &handle_point.1]);
    until(|| {
        v.widgets
            .surface
            .pick(handle_point.0, handle_point.1, gtk::PickFlags::DEFAULT)
            .as_ref()
            == Some(v.widgets.port_handle.upcast_ref())
    });
    let controllers = v.widgets.port_handle.observe_controllers();
    let link = (0..controllers.n_items())
        .find_map(|i| controllers.item(i).and_downcast::<gtk::GestureDrag>())
        .unwrap();
    link.emit_by_name::<()>("drag-begin", &[&0., &0.]);
    assert!(
        matches!(*v.session.interaction.borrow(), Interaction::Link { from, .. } if from == id)
    );
    link.emit_by_name::<()>("cancel", &[&None::<gtk::gdk::EventSequence>]);
    assert!(v.session.interaction.borrow().is_idle());
    let camera = v.session.camera.get();
    scene::board::set_camera(
        v,
        Camera {
            zoom: 0.2,
            ..camera
        },
    );
    let tiny_handle = v.session.camera.get().to_screen((
        position[0] + port::CARD_WIDTH,
        position[1] + port::CARD_HEIGHT / 2.,
    ));
    pointer.emit_by_name::<()>("motion", &[&tiny_handle.0, &tiny_handle.1]);
    assert_ne!(
        v.widgets
            .surface
            .pick(tiny_handle.0, tiny_handle.1, gtk::PickFlags::DEFAULT)
            .as_ref(),
        Some(v.widgets.port_handle.upcast_ref()),
        "tiny zoom must not route a press to a hidden handle"
    );
    settle(40);
    assert_eq!(
        v.session.port_owner.get(),
        None,
        "tiny zoom keeps handles hidden"
    );
    scene::board::set_camera(v, camera);
    // 겹친 두 카드에서는 GTK의 실제 그리기 순서를 따르고, 맨 위로 올리면 즉시 바뀐다.
    let other_position = v.session.shown.borrow()[&other];
    scene::board::show_card(v, other, position);
    let cards = v.widgets.cards.borrow();
    cards[&other]
        .root
        .insert_before(&v.widgets.world, Some(&v.widgets.port_handle));
    pointer.emit_by_name::<()>("motion", &[&point.0, &point.1]);
    assert_eq!(v.session.port_owner.get(), Some(other));
    cards[&id]
        .root
        .insert_before(&v.widgets.world, Some(&v.widgets.port_handle));
    pointer.emit_by_name::<()>("motion", &[&point.0, &point.1]);
    assert_eq!(v.session.port_owner.get(), Some(id));
    // 실제 카드 이동 경로처럼 순서를 바꾼 뒤 원래 위치로 돌린다.
    cards[&other]
        .root
        .insert_before(&v.widgets.world, Some(&v.widgets.port_handle));
    drop(cards);
    scene::board::show_card(v, other, other_position);
    v.session.selection.borrow_mut().set(selected);
    v.sync_selection();
}

fn wait_notebook(state: &State) {
    until(|| {
        state.borrow_mut().notebook.tick(std::time::Instant::now());
        !state.borrow().notebook.needs_tick()
    });
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
mod flow;
mod performance;
mod projects;
mod shortcuts;

pub(in crate::app) use flow::canvas_001_widget_flow_and_recovery;
pub(in crate::app) use performance::perf_002_frames;
pub(in crate::app) use projects::project_001_levels_grouping_and_shortcuts;
pub(in crate::app) use shortcuts::canvas_004_markers_shortcuts_and_delete;
