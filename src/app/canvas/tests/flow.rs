use super::*;

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
    until(|| (760..=840).contains(&v.widgets.window.width()) && v.widgets.surface.width() > 0);
    pointer_hit_testing(&v, a, b);
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
    until(|| {
        v.widgets.cards.borrow().values().all(|card| {
            !card.root.get_visible()
                || card
                    .root
                    .compute_bounds(&v.widgets.world)
                    .is_some_and(|bounds| {
                        f64::from(bounds.height()) == port::CARD_HEIGHT
                            && f64::from(bounds.width()) == port::CARD_WIDTH
                    })
        })
    });
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
    input::pointer::test_pinch_interrupts_scroll(&v);
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
    until(|| {
        gtk::prelude::GtkWindowExt::focus(&v.widgets.window)
            .is_some_and(|w| w.widget_name() == "canvas-control-done")
    });
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
    until(|| {
        gtk::prelude::GtkWindowExt::focus(&v.widgets.window)
            .is_some_and(|w| w.widget_name() == a.to_string())
    });
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
