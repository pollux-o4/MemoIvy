use super::*;

fn child(id: Uuid, text: &str, position: [f64; 2], next: Vec<Uuid>, parent: Option<Uuid>) -> Note {
    serde_json::from_value(serde_json::json!({
        "id": id, "text": text, "color": "blue", "modified": 1,
        "canvas": { "position": position, "next": next, "done": false, "parent": parent }
    }))
    .unwrap()
}

fn ids(v: &CanvasView) -> Vec<Uuid> {
    let mut ids: Vec<_> = v.session.nodes.borrow().iter().map(|n| n.id).collect();
    ids.sort();
    ids
}

fn sorted(mut ids: Vec<Uuid>) -> Vec<Uuid> {
    ids.sort();
    ids
}

fn parent(state: &State, id: Uuid) -> Option<Uuid> {
    state.borrow().notebook.note(id).unwrap().canvas().parent
}

fn arrive(v: &Rc<CanvasView>, scope: Option<Uuid>) {
    until(|| v.session.scope.get() == scope && !v.session.diving.get());
}

/// PROJECT-001: 들어가기·나오기, 투영 연결, 묶기·되돌리기, 끌어 넣기 대체 경로, 고정 바로가기, 프로젝트 삭제.
pub(in crate::app) fn project_001_levels_grouping_and_shortcuts() {
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let app = Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.ProjectTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let (p, a, b, c) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let notes = vec![
        child(p, "# 취업하기", [64., 64.], Vec::new(), None),
        child(
            a,
            "# 안쪽 A\n- [x] 하나\n- [ ] 둘",
            [100., 100.],
            vec![b],
            Some(p),
        ),
        child(b, "# 바깥 B", [420., 64.], Vec::new(), None),
        child(c, "# 바깥 C", [64., 420.], Vec::new(), None),
    ];
    let state = AppState::fixture(path, &app, notes);
    open(&state, &app, None);
    let v = state.borrow().views.canvas.clone().unwrap();
    until(|| v.session.fitted.get() && v.widgets.surface.width() > 0);

    // 최상위: 프로젝트 카드 하나와 바깥 메모 둘. 안쪽 A→B 연결은 P→B 점선으로 보인다.
    assert_eq!(ids(&v), sorted(vec![p, b, c]));
    let card = v.node(p).unwrap();
    assert_eq!(card.project.map(|s| (s.notes, s.tasks)), Some((1, (1, 2))));
    assert_eq!(card.through, vec![b]);
    assert!(
        card.next.is_empty(),
        "projected links are not editable links"
    );
    assert!(v.widgets.navigator.root.is_visible());
    capture(&v);

    // 들어가기: 떠나는 화면을 텍스처로 굳히고, 새 단계는 모션이 시작되기 전에 이미 배치되어 있다.
    v.enter(p);
    assert!(v.session.diving.get(), "fixture must animate");
    assert!(v.session.dive.borrow().is_some() && v.widgets.freeze.is_visible());
    assert_eq!(v.session.scope.get(), Some(p));
    assert_eq!(ids(&v), vec![a]);
    arrive(&v, Some(p));
    assert!(v.session.dive.borrow().is_none() && !v.widgets.freeze.is_visible());
    // 방문 저장은 모션이 끝난 뒤로 미룬다.
    until(|| {
        state
            .borrow()
            .navigation
            .ranked(crate::app::shared::unix_now(), |_| true)
            .contains(&p)
    });
    assert_eq!(ids(&v), vec![a]);
    assert!(
        v.node(a).unwrap().through.is_empty(),
        "outside targets stay hidden"
    );
    assert!(state
        .borrow()
        .navigation
        .ranked(crate::app::shared::unix_now(), |_| true)
        .contains(&p));
    // 안에서 만든 메모는 이 프로젝트에 담긴다.
    crate::app::shell::commands::new_note(&state, v.session.scope.get());
    until(|| v.session.nodes.borrow().len() == 2 && !state.borrow().notebook.busy());
    let made = ids(&v).into_iter().find(|id| *id != a).unwrap();
    assert_eq!(parent(&state, made), Some(p));

    // 나오기: 선택이 없으면 Esc가 한 단계 위로, 방금 있던 프로젝트를 선택해 둔다.
    v.session.selection.borrow_mut().set([]);
    assert!(v.leave());
    arrive(&v, None);
    assert_eq!(v.session.selection.borrow().single(), Some(p));
    assert!(!v.leave(), "top level has nowhere to go");

    // 반복 왕복해도 카드 위젯·화면 기억이 쌓이지 않는다. 전환 모션은 위에서 확인했으니 여기선 끈다.
    let settings = v.widgets.window.settings();
    let animated = settings.is_gtk_enable_animations();
    settings.set_gtk_enable_animations(false);
    for _ in 0..20 {
        v.enter(p);
        arrive(&v, Some(p));
        v.leave();
        arrive(&v, None);
    }
    let mut widgets = 0;
    let mut child = v.widgets.world.first_child();
    while let Some(widget) = child {
        widgets += 1;
        child = widget.next_sibling();
    }
    // 다른 단계 카드는 숨겨 다시 쓰므로 위젯은 메모 수 + 연결 핸들 하나를 넘지 않는다.
    let notes = state.borrow().notebook.notes().len();
    assert_eq!(widgets, notes + 1);
    assert_eq!(
        v.widgets.cards.borrow().len(),
        v.session.nodes.borrow().len()
    );
    assert_eq!(
        v.widgets.cards.borrow().len() + v.widgets.parked.borrow().len(),
        notes
    );
    assert!(v.session.cameras.borrow().len() <= 2);
    settings.set_gtk_enable_animations(animated);

    // 묶기: 새 프로젝트 메모가 만들어지면 선택이 그 안으로 들어가고, 되돌리기로 풀린다.
    v.session.selection.borrow_mut().set([b, c]);
    v.sync_selection();
    v.group_selected();
    until(|| state.borrow().pending_group.is_none() && !state.borrow().notebook.busy());
    let group = parent(&state, b).expect("grouped");
    assert_eq!(parent(&state, c), Some(group));
    until(|| ids(&v) == sorted(vec![p, group]));
    v.undo();
    assert_eq!((parent(&state, b), parent(&state, c)), (None, None));

    // 프로젝트로 옮기기(끌어 놓기의 대체 경로)와 깊이·순환 보호.
    v.move_into(vec![c], Some(p));
    assert_eq!(parent(&state, c), Some(p));
    assert!(!ids(&v).contains(&c));
    v.move_into(vec![p], Some(p));
    assert_eq!(parent(&state, p), None, "a project cannot hold itself");

    // 고정하면 다른 단계에서도 바로가기 맨 앞에 나온다.
    v.toggle_pin(p);
    let shortcuts = port::shortcuts(&state.borrow(), Some(group), 5);
    assert_eq!(
        shortcuts.first().map(|(place, pinned)| (place.id, *pinned)),
        Some((p, true))
    );
    assert!(
        port::shortcuts(&state.borrow(), Some(p), 5)
            .iter()
            .all(|(place, _)| place.id != p),
        "the current place is not a shortcut"
    );

    // 보던 프로젝트를 지우면 최상위로 돌아오고 안의 메모는 남는다.
    v.enter(p);
    arrive(&v, Some(p));
    crate::app::shell::commands::delete_notes(&state, vec![p]);
    until(|| state.borrow().notebook.note(p).is_none() && v.session.scope.get().is_none());
    assert_eq!(parent(&state, a), None);
    assert_eq!(parent(&state, c), None);
    until(|| ids(&v).contains(&a) && ids(&v).contains(&c));
}
