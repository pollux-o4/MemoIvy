//! FILE-001 위젯 시나리오.
use super::*;
use crate::app::shell::state::AppState;
use crate::app::test_support::{button_with_class, save_png, settle, until};
use crate::persistence::{Attachments, Note};
use uuid::Uuid;

fn note(text: String, id: Uuid) -> Note {
    serde_json::from_value(serde_json::json!({
        "id": id, "text": text, "color": "yellow", "modified": 1
    }))
    .unwrap()
}

fn counts(view: &FilesView) -> (String, String) {
    (
        view.used.label().unwrap().to_string(),
        view.unused.label().unwrap().to_string(),
    )
}

pub(in crate::app) fn file_001_files_view() {
    let app = gtk::Application::builder()
        .application_id("io.github.pollux_o4.MemoIvy.FilesTest")
        .build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");
    let store = Attachments::beside(&path).store();
    let used = store.import(b"\x89PNG\r\n\x1a\nused").unwrap();
    let unused = store.import(b"\x89PNG\r\n\x1a\nunused").unwrap();
    let owner = Uuid::new_v4();
    let reference = format!("# 계획\n![](asset:{used})");
    let state = AppState::fixture(path, &app, vec![note(reference, owner)]);

    open(&state, &app);
    let view = state.borrow().views.files.clone().unwrap();
    assert_eq!(counts(&view), ("사용 중 1".into(), "사용 안 함 1".into()));
    assert_eq!(view.selected.borrow().as_ref(), Some(&used));
    assert!(!view.detail.delete.is_visible(), "사용 중은 삭제할 수 없다");
    assert_eq!(view.detail.heading.text(), "사용 중인 메모");
    let link = view
        .detail
        .notes
        .first_child()
        .and_downcast::<gtk::Button>()
        .unwrap();
    assert_eq!(link.label().as_deref(), Some("계획"));

    // 같은 창을 다시 쓴다.
    open(&state, &app);
    assert!(Rc::ptr_eq(
        &view,
        state.borrow().views.files.as_ref().unwrap()
    ));

    // 사용 안 함: 마지막 사용처가 없고 삭제할 수 있다.
    view.unused.set_active(true);
    assert_eq!(view.selected.borrow().as_ref(), Some(&unused));
    assert!(view.detail.delete.is_visible());
    assert!(view.purge.is_visible());

    // 삭제는 확인창을 거친다. 취소가 기본이고 수락하면 파일이 사라진다.
    view.detail.delete.emit_clicked();
    let dialog = || {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Window>().ok())
            .find(|w| w.has_css_class("memoivy-confirm") && w.is_visible())
    };
    until(|| dialog().is_some());
    assert!(store.path(&unused).is_some());
    button_with_class(dialog().unwrap().upcast_ref(), "confirm-accept")
        .unwrap()
        .emit_clicked();
    until(|| dialog().is_none());
    assert!(store.path(&unused).is_none());
    assert_eq!(counts(&view).1, "사용 안 함 0");

    // 본문에서 참조를 지우면 저장 뒤 사용 안 함으로 옮겨지고 마지막 메모를 기억한다.
    state.borrow_mut().notebook.set_text(owner, "# 계획".into());
    crate::app::shell::commands::reconcile_attachments(&state);
    assert_eq!(counts(&view), ("사용 중 0".into(), "사용 안 함 1".into()));
    assert_eq!(view.selected.borrow().as_ref(), Some(&used));
    assert!(view.detail.heading.text().starts_with("마지막 사용"));

    // 확인 사이에 다시 쓰이면 삭제를 거부한다.
    let notes = vec![note(format!("![](asset:{used})"), Uuid::new_v4())];
    assert!(state
        .borrow_mut()
        .attachments
        .delete(&used, &notes)
        .is_err());

    capture(&state, &app, &store);
    view.window.close();
    until(|| state.borrow().views.files.is_none());
}

/// MEMOIVY_MEDIA_ARTIFACT_DIR이 있으면 실제 편집 창·파일 창·크게 보기를 앱 CSS로 그려 남긴다.
fn capture(
    state: &crate::app::shell::state::State,
    app: &Application,
    store: &crate::persistence::AssetStore,
) {
    let Ok(dir) = std::env::var("MEMOIVY_MEDIA_ARTIFACT_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    crate::presentation::install();
    let (width, height) = (640, 360);
    let mut pixels = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend([
                (40 + x * 120 / width) as u8,
                (90 + y * 100 / height) as u8,
                200,
            ]);
        }
    }
    let shot = gtk::gdk::MemoryTexture::new(
        width,
        height,
        gtk::gdk::MemoryFormat::R8g8b8,
        &gtk::glib::Bytes::from(&pixels),
        (width * 3) as usize,
    );
    let id = store.import(&shot.save_to_png_bytes()).unwrap();
    // 새 메모는 저장 계층을 거쳐야 해 기존 메모 본문을 바꿔 쓴다.
    let owner = state.borrow().notebook.notes()[0].id();
    state.borrow_mut().notebook.set_text(
        owner,
        format!("# 출시 계획\n지난주 화면 캡처:\n![대시보드|300](asset:{id})\n다음 단계는 리뷰."),
    );
    crate::app::shell::commands::open_note(state, app, owner);
    let editor = state
        .borrow()
        .views
        .windows
        .get(&owner)
        .unwrap()
        .window
        .clone();
    editor.set_default_size(420, 520);
    settle(600);
    if let Some(image) = crate::app::test_support::find_class(editor.upcast_ref(), "md-image") {
        image.add_css_class("hover");
    }
    settle(300);
    save_png(&editor, &dir.join("memo-image.png"));
    crate::app::shell::commands::reconcile_attachments(state);
    open(state, app);
    let files = state.borrow().views.files.clone().unwrap();
    files.used.set_active(true);
    settle(600);
    save_png(&files.window, &dir.join("files-used.png"));
    files.unused.set_active(true);
    settle(400);
    save_png(&files.window, &dir.join("files-unused.png"));
    crate::app::widgets::show_image(&editor, shot.upcast_ref(), "대시보드");
    settle(500);
    if let Some(viewer) = gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|w| w.downcast::<gtk::Window>().ok())
        .find(|w| w.has_css_class("image-viewer"))
    {
        save_png(&viewer, &dir.join("viewer.png"));
        viewer.close();
    }
    editor.destroy();
}
