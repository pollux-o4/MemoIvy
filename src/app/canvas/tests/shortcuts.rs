use super::*;
use crate::presentation::markdown::{LineMarker, PreviewLine, PreviewRow, TaskState};

pub(in crate::app) fn canvas_m1_preview_marker_ink_regression() {
    crate::presentation::install();
    let preview = scene::preview::Preview::new();
    preview.set(
        &(1..=3)
            .map(|depth| PreviewRow {
                line: PreviewLine {
                    marker: Some(LineMarker::Bullet),
                    depth,
                    continuation: false,
                    text: "가나다".into(),
                },
                max_lines: 1,
            })
            .chain([PreviewRow {
                line: PreviewLine {
                    marker: Some(LineMarker::Task(TaskState::Done)),
                    depth: 3,
                    continuation: false,
                    text: "가나다".into(),
                },
                max_lines: 1,
            }])
            .collect::<Vec<_>>(),
    );
    let window = gtk::Window::builder().child(&preview.root).build();
    window.present();
    until(|| preview.root.is_mapped() && preview.root.width() > 0);
    settle(20);

    let scale = f64::from(gtk::pango::SCALE);
    let mut failures = Vec::new();
    for (index, (marker_label, text_label)) in preview.ink_test_rows().iter().enumerate() {
        let (marker_ink, _) = marker_label.layout().extents();
        let marker_top = marker_label
            .compute_point(&preview.root, &gtk::graphene::Point::new(0., 0.))
            .unwrap()
            .y();
        let text_top = text_label
            .compute_point(&preview.root, &gtk::graphene::Point::new(0., 0.))
            .unwrap()
            .y();
        let anchor = crate::presentation::typography::line_anchor(
            &text_label.layout(),
            &text_label.create_pango_layout(Some("x")),
            None,
        );
        let marker_center =
            f64::from(marker_top) + f64::from(marker_ink.y() + marker_ink.height() / 2) / scale;
        let target = f64::from(text_top) + anchor.line_top + anchor.baseline - anchor.x_center;
        let delta = (marker_center - target).abs();
        println!("M1 canvas_row={index} marker_ink_center={marker_center:.2}px first_line_x_height_center={target:.2}px delta={delta:.2}px");
        if delta > 1.0 {
            failures.push(format!("canvas preview row {index}: {delta}px"));
        }
    }
    window.destroy();
    drain();
    assert!(failures.is_empty(), "{}", failures.join("; "));
}

fn press(v: &CanvasView, key: gtk::gdk::Key, modifiers: gtk::gdk::ModifierType) -> bool {
    let controllers = v.widgets.window.observe_controllers();
    (0..controllers.n_items())
        .filter_map(|i| {
            controllers
                .item(i)
                .and_downcast::<gtk::EventControllerKey>()
        })
        .any(|keys| keys.emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers]))
}

fn confirm_dialog(v: &CanvasView) -> Option<gtk::Window> {
    gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Window>().ok())
        .find(|w| {
            w.has_css_class("memoivy-confirm")
                && w.is_visible()
                && w.transient_for().as_ref() == Some(v.widgets.window.upcast_ref())
        })
}

fn run_shell(state: &State) {
    crate::app::shell::lifecycle::wake(state);
    until(|| !state.borrow().timer_active);
}

pub(in crate::app) fn canvas_004_markers_shortcuts_and_delete() {
    use gtk::gdk::{Key, ModifierType};
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let app = Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.CanvasMarkersTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
    let texts = [
        "할 일 제목\n- [ ] 첫째\n- [x] 둘째\n1. 번호\n- 기호",
        "- [x] 머리 할 일\n문단",
        "그냥 문단\n이어지는 문단",
        "지울 메모 A",
        "지울 메모 B",
    ];
    let notes = texts
        .iter()
        .zip(&ids)
        .enumerate()
        .map(|(i, (text, id))| {
            let next = if i == 3 { vec![ids[0]] } else { Vec::new() };
            note((*text).to_owned(), [64. + i as f64 * 300., 64.], next, *id)
        })
        .collect();
    let state = AppState::fixture(path.clone(), &app, notes);
    open(&state, &app, None);
    let v = state.borrow().views.canvas.clone().unwrap();
    until(|| v.session.fitted.get() && v.widgets.surface.width() > 0);
    scene::board::set_camera(&v, Camera::default());

    // 카드 미리보기가 표식과 상태를 잃지 않는다.
    {
        let cards = v.widgets.cards.borrow();
        let rows = scene::card::preview_rows(&cards[&ids[0]]);
        assert_eq!(
            rows,
            [
                ("".to_owned(), "첫째".to_owned()),
                ("✓".into(), "둘째".into()),
                ("1.".into(), "번호".into()),
            ]
        );
        assert_eq!(
            scene::card::more_text(&cards[&ids[0]]).as_deref(),
            Some("외 1줄")
        );
        // 첫 줄이 할 일이면 제목 앞에 체크 상태를 함께 보인다.
        let classes = scene::card::title_marker_classes(&cards[&ids[1]]);
        assert!(classes.iter().any(|c| c == "checked"), "{classes:?}");
        // 일반 문단은 한 행으로 합쳐 지금처럼 이어 읽는다.
        assert_eq!(
            scene::card::preview_rows(&cards[&ids[2]]),
            [("".to_owned(), "이어지는 문단".to_owned())]
        );
        assert_eq!(scene::card::more_text(&cards[&ids[2]]), None);
    }

    // 선택이 없으면 Delete는 다른 위젯에게 돌려준다.
    assert!(!press(&v, Key::Delete, ModifierType::empty()));
    assert!(confirm_dialog(&v).is_none());

    // 취소가 기본이며 Esc·취소는 아무것도 지우지 않는다.
    v.select_only(ids[3]);
    assert!(press(&v, Key::Delete, ModifierType::empty()));
    settle(30);
    let dialog = confirm_dialog(&v).expect("Delete asks first");
    let cancel = button_with_class(dialog.upcast_ref(), "confirm-cancel").unwrap();
    assert!(cancel.is_focus(), "cancel is the default response");
    cancel.emit_clicked();
    settle(30);
    assert!(confirm_dialog(&v).is_none());
    assert_eq!(state.borrow().notebook.notes().len(), 5);
    assert!(
        v.session.selection.borrow().contains(ids[3]),
        "cancel keeps selection"
    );

    // 입력 위젯이 없는 보드에서 Backspace도 같은 동작이다. 여러 개는 한 번의 확인으로 지운다.
    v.session.selection.borrow_mut().set([ids[3], ids[4]]);
    v.sync_selection();
    assert!(press(&v, Key::BackSpace, ModifierType::empty()));
    settle(30);
    let dialog = confirm_dialog(&v).expect("Backspace asks first");
    // 확인창이 떠 있는 동안 또 눌러도 창이 쌓이지 않는다: 모달이 입력을 가져간다.
    button_with_class(dialog.upcast_ref(), "confirm-accept")
        .unwrap()
        .emit_clicked();
    run_shell(&state);
    settle(30);
    assert_eq!(state.borrow().notebook.notes().len(), 3);
    assert!(state.borrow().delete_queue.is_empty());
    assert!(
        v.session.selection.borrow().is_empty(),
        "deleted notes leave the selection"
    );
    assert_eq!(v.widgets.cards.borrow().len(), 3);
    assert!(!press(&v, Key::Delete, ModifierType::empty()));
    let saved = Notebook::open(path.clone()).unwrap();
    assert_eq!(saved.notes().len(), 3);
    // 대기열은 실패하면 멈춘다. 남은 메모는 그대로이고 사용자가 다시 판단한다.
    let remaining: Vec<Uuid> = state
        .borrow()
        .notebook
        .notes()
        .iter()
        .map(|n| n.id())
        .collect();
    // 완료 단축키: 한 번의 되돌리기로 전체 복구, 입력 위젯 없이도 동작.
    v.session
        .selection
        .borrow_mut()
        .set([remaining[0], remaining[1]]);
    v.sync_selection();
    assert!(press(&v, Key::Return, ModifierType::CONTROL_MASK));
    settle(20);
    assert!(state.borrow().notebook.notes()[0].canvas().done);
    assert!(state.borrow().notebook.notes()[1].canvas().done);
    assert!(press(&v, Key::z, ModifierType::CONTROL_MASK));
    settle(20);
    assert!(!state.borrow().notebook.notes()[0].canvas().done);
    assert!(!state.borrow().notebook.notes()[1].canvas().done);
    // 연결 단축키는 후보 목록을 연다.
    v.select_only(remaining[0]);
    settle(30);
    assert!(press(&v, Key::l, ModifierType::CONTROL_MASK));
    settle(30);
    let menu = named(v.widgets.inspector.upcast_ref(), "canvas-control-connect")
        .and_downcast::<gtk::MenuButton>()
        .unwrap();
    assert!(menu.is_active());
    menu.popdown();
    settle(20);
    // 패널의 삭제 항목도 같은 확인을 거친다.
    let delete = named(v.widgets.inspector.upcast_ref(), "canvas-control-delete")
        .and_downcast::<Button>()
        .unwrap();
    delete.emit_clicked();
    settle(30);
    let dialog = confirm_dialog(&v).expect("panel delete asks first");
    button_with_class(dialog.upcast_ref(), "confirm-cancel")
        .unwrap()
        .emit_clicked();
    settle(30);
    assert_eq!(state.borrow().notebook.notes().len(), 3);

    // 저장이 실패하면 남은 삭제를 멈춘다.
    let ids_left: Vec<Uuid> = remaining.clone();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    crate::app::shell::commands::delete_notes(&state, ids_left);
    run_shell(&state);
    assert!(
        state.borrow().delete_queue.is_empty(),
        "failure stops the queue"
    );
    assert_eq!(
        state.borrow().notebook.notes().len(),
        3,
        "failed delete keeps notes"
    );
    std::fs::remove_dir(&path).unwrap();

    v.destroy();
    state.borrow().views.library.window.destroy();
    drain();
}
