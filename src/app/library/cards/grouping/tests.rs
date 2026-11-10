use crate::app::shell::state::AppState;
use crate::app::test_support::{button_with_class, until};
use gtk::{prelude::*, Application, Button};
use uuid::Uuid;

pub(in crate::app) fn project_002_sections_toggle_in_place_and_keep_preview() {
    crate::presentation::install();
    let dir = tempfile::tempdir().unwrap();
    let app = Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.LibrarySections")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let (parent, nested, leaf) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let notes = [
        (parent, None, true, "프로젝트".to_owned()),
        (nested, Some(parent), true, "하위 프로젝트".to_owned()),
        (
            leaf,
            Some(nested),
            false,
            format!("메모\n{}", "본문\n".repeat(8)),
        ),
    ]
    .into_iter()
    .map(|(id, parent, project, text)| {
        serde_json::from_value(serde_json::json!({
            "id":id,"text":text,"color":"yellow","modified":1,
            "canvas":{"parent":parent,"project":project,"position":[64,64]}
        }))
        .unwrap()
    })
    .collect();
    let state = AppState::fixture(dir.path().join("notes.json"), &app, notes);
    let (window, list) = {
        let s = state.borrow();
        let library = &s.views.library;
        library.grouped.set_active(true);
        (library.window.clone(), library.list.clone())
    };
    window.set_child(Some(&list));
    window.present();
    super::super::refresh(&state, &app);
    let section = list.first_child().unwrap();
    assert!(section.has_css_class("project-section"));
    let reveal = section
        .first_child()
        .unwrap()
        .next_sibling()
        .and_downcast::<gtk::Revealer>()
        .unwrap();
    assert!(!reveal.reveals_child(), "new projects default to collapsed");
    assert!(reveal.child().is_none(), "collapsed branches must be lazy");
    let find = |id: Uuid| {
        crate::app::shared::find_named(list.upcast_ref(), &id.to_string()).and_downcast::<Button>()
    };
    let header = find(parent).unwrap();
    assert!(find(nested).is_none());
    header.grab_focus();
    header.emit_clicked();
    until(|| reveal.is_child_revealed());
    let actions = section
        .first_child()
        .unwrap()
        .first_child()
        .unwrap()
        .last_child()
        .unwrap();
    let canvas = actions.first_child().unwrap();
    let bounds = canvas.compute_bounds(&section).unwrap();
    assert!(
        canvas.is_mapped()
            && bounds.width() > 20.
            && bounds.x() + bounds.width() <= section.width() as f32,
        "project actions must remain inside the revealed card: {bounds:?}"
    );
    assert!(
        state.borrow().views.windows.is_empty(),
        "project disclosure reveals cards rather than opening an editor"
    );
    let nested_header = find(nested).unwrap();
    nested_header.emit_clicked();
    let more = button_with_class(list.upcast_ref(), "card-reveal").unwrap();
    more.emit_clicked();
    assert!(more.has_css_class("expanded"));
    until(|| more.width() > 0);
    let owner = more.parent().unwrap();
    let bounds = more.compute_bounds(&owner).unwrap();
    assert!(
        (bounds.x() + bounds.width() / 2. - owner.width() as f32 / 2.).abs() <= 1.,
        "content disclosure must share its card's center axis"
    );
    header.emit_clicked();
    until(|| !reveal.is_child_revealed());
    assert!(state
        .borrow()
        .views
        .library
        .preferences
        .borrow()
        .expanded()
        .contains(&nested));
    header.emit_clicked();
    until(|| reveal.is_child_revealed());
    assert_eq!(
        find(nested).unwrap(),
        nested_header,
        "retain the same child widgets"
    );
    assert_eq!(
        button_with_class(list.upcast_ref(), "card-reveal").unwrap(),
        more
    );
    assert!(more.has_css_class("expanded"));
    super::super::refresh(&state, &app);
    assert_eq!(
        find(parent).unwrap(),
        header,
        "unrelated refresh must not rebuild disclosure"
    );
    button_with_class(&section.first_child().unwrap(), "card-open")
        .unwrap()
        .emit_clicked();
    until(|| state.borrow().views.windows.contains_key(&parent));
    assert!(
        reveal.reveals_child(),
        "editing a description must not toggle its section"
    );
    let editor = state.borrow().views.windows[&parent].window.clone();
    editor.close();
    window.destroy();
}
