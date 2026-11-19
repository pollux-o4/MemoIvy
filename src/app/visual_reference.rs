//! Opt-in captures of actual GTK widgets for UI-REF-001 comparison.
use crate::app::{
    shell::commands,
    test_support::{save_png, settle, until},
};
use gtk::{prelude::*, Application};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const FIXTURE: &str = include_str!("../../rooms/desktop-migration/visual-fixture.json");
const FIXTURE_PATH: &str = "rooms/desktop-migration/visual-fixture.json";
const APP_ID: &str = "io.github.pollux_o4.MemoIvy.GtkReferenceCapture";
const COMMAND: &str = "mise run test:gtk-reference";
const RENDERER: &str =
    "GTK WidgetPaintable + native renderer via app::test_support::save_png under Xvfb";

/// Captures only when explicitly enabled with MEMOIVY_GTK_REFERENCE_DIR.
pub(in crate::app) fn capture_gtk_reference_assets() {
    let Ok(output) = std::env::var("MEMOIVY_GTK_REFERENCE_DIR") else {
        return;
    };
    let output = PathBuf::from(output);
    fs::create_dir_all(&output).expect("create GTK reference output directory");
    let canonical_dir = output.join("fixtures/canonical");
    fs::create_dir_all(&canonical_dir).unwrap();
    fs::write(canonical_dir.join("notes.json"), FIXTURE).unwrap();
    crate::presentation::install();

    let app = Application::builder().application_id(APP_ID).build();
    app.register(None::<&gtk::gio::Cancellable>).unwrap();
    let temp = tempfile::tempdir().expect("isolated GTK reference fixture");
    let notes_path = temp.path().join("notes.json");
    let canonical_notes: Vec<Value> =
        serde_json::from_str(FIXTURE).expect("migration fixture JSON");
    let ids: Vec<Uuid> = canonical_notes
        .iter()
        .map(|n| Uuid::parse_str(n["id"].as_str().unwrap()).unwrap())
        .collect();
    fs::write(&notes_path, FIXTURE).unwrap();
    let (library_window, state) = crate::app::library::capture_build(&app, &notes_path);
    commands::open_note(&state, &app, ids[0]);
    commands::open_canvas(&state, &app, None);
    commands::open_files(&state, &app);
    let editor = find_window("메모");
    let canvas = find_window("메모 캔버스");
    let files = find_window("파일");
    library_window.set_default_size(440, 600);
    editor.set_default_size(380, 420);
    canvas.set_default_size(1000, 720);
    files.set_default_size(720, 520);
    settle(600);

    let fixture_hash = sha256(FIXTURE.as_bytes());
    let mut captures = Vec::new();
    let settings = library_window.settings();
    let old_dark = settings.is_gtk_application_prefer_dark_theme();
    let old_theme = settings.gtk_theme_name();
    let old_animations = settings.is_gtk_enable_animations();
    let old_scheme = settings
        .find_property("gtk-interface-color-scheme")
        .map(|_| settings.property_value("gtk-interface-color-scheme"));
    settings.set_gtk_theme_name(Some("Adwaita"));
    settings.set_gtk_enable_animations(false);

    // Canonical fixture: all four source notes and their original canvas graph are unchanged.
    for (theme, dark) in [("light", false), ("dark", true)] {
        set_theme(&settings, dark);
        settle(350);
        capture(
            &library_window,
            &output,
            &format!("library-{theme}.png"),
            440,
            600,
            "library",
            theme,
            "canonical",
            &fixture_hash,
            &mut captures,
        );
        capture(
            &editor,
            &output,
            &format!("editor-{theme}.png"),
            380,
            420,
            "editor",
            theme,
            "canonical",
            &fixture_hash,
            &mut captures,
        );
        capture(
            &canvas,
            &output,
            &format!("canvas-{theme}.png"),
            1000,
            720,
            "canvas",
            theme,
            "canonical",
            &fixture_hash,
            &mut captures,
        );
        capture(
            &files,
            &output,
            &format!("files-empty-used-{theme}.png"),
            720,
            520,
            "files-empty-used",
            theme,
            "canonical",
            &fixture_hash,
            &mut captures,
        );
        click_toggle(&files, "사용 안 함");
        settle(160);
        capture(
            &files,
            &output,
            &format!("files-empty-unused-{theme}.png"),
            720,
            520,
            "files-empty-unused",
            theme,
            "canonical",
            &fixture_hash,
            &mut captures,
        );
        click_toggle(&files, "사용 중");
    }

    // Add generated image states only after canonical captures; export the exact derived notes and blobs.
    let used_image = sample_png(0);
    let used_id = crate::persistence::Attachments::beside(&notes_path)
        .store()
        .import(&used_image.save_to_png_bytes())
        .unwrap();
    let unused_id = crate::persistence::Attachments::beside(&notes_path)
        .store()
        .import(&sample_png(57).save_to_png_bytes())
        .unwrap();
    let updated_text = format!(
        "{}\n\n![대시보드 미리보기|240](asset:{used_id})",
        state.borrow().notebook.notes()[0].text()
    );
    let editor_view = state.borrow().views.windows[&ids[0]].view.clone();
    editor_view.buffer().set_text(&updated_text);
    commands::reconcile_attachments(&state);
    state.borrow().views.files.as_ref().unwrap().refresh(&state);
    let image_notes = notes_json(&state);
    let image_dataset_hash = export_dataset(&output, "image-used", &notes_path, &image_notes, None);

    for (theme, dark) in [("light", false), ("dark", true)] {
        set_theme(&settings, dark);
        settle(350);
        capture(
            &editor,
            &output,
            &format!("editor-image-{theme}.png"),
            380,
            420,
            "editor-image",
            theme,
            "image-used",
            &image_dataset_hash,
            &mut captures,
        );
        capture(
            &files,
            &output,
            &format!("files-used-{theme}.png"),
            720,
            520,
            "files-used",
            theme,
            "image-used",
            &image_dataset_hash,
            &mut captures,
        );
        click_toggle(&files, "사용 안 함");
        settle(180);
        capture(
            &files,
            &output,
            &format!("files-unused-{theme}.png"),
            720,
            520,
            "files-unused",
            theme,
            "image-used",
            &image_dataset_hash,
            &mut captures,
        );
        click_toggle(&files, "사용 중");
    }

    // Expanded nested-library variant from the same source notes plus the generated used/unused image data.
    {
        let mut app_state = state.borrow_mut();
        app_state.notebook.edit_canvas(|edit| {
            assert!(edit.set_parent(ids[1], Some(ids[0])));
            assert!(edit.set_parent(ids[2], Some(ids[1])));
        });
        app_state
            .views
            .library
            .preferences
            .borrow_mut()
            .toggle(ids[0], |_| true)
            .unwrap();
        app_state
            .views
            .library
            .preferences
            .borrow_mut()
            .toggle(ids[1], |_| true)
            .unwrap();
    }
    crate::app::library::refresh(&state, &app);
    settle(320);
    let expanded_notes = notes_json(&state);
    let expanded_dataset_hash = export_dataset(
        &output,
        "library-expanded",
        &notes_path,
        &expanded_notes,
        Some(json!([ids[0], ids[1]])),
    );
    for (theme, dark) in [("light", false), ("dark", true)] {
        set_theme(&settings, dark);
        settle(300);
        capture(
            &library_window,
            &output,
            &format!("library-expanded-{theme}.png"),
            440,
            600,
            "library-expanded",
            theme,
            "library-expanded",
            &expanded_dataset_hash,
            &mut captures,
        );
    }

    settings.set_gtk_application_prefer_dark_theme(old_dark);
    settings.set_gtk_theme_name(old_theme.as_deref());
    settings.set_gtk_enable_animations(old_animations);
    if let Some(scheme) = old_scheme {
        settings.set_property_from_value("gtk-interface-color-scheme", &scheme);
    }
    write_manifest(
        &output,
        captures,
        fixture_hash,
        used_id.as_str(),
        unused_id.as_str(),
    );
    library_window.close();
    editor.close();
    canvas.close();
    files.close();
    until(|| {
        gtk::Window::list_toplevels()
            .into_iter()
            .all(|w| !w.is_visible())
    });
}

fn sample_png(seed: u8) -> gtk::gdk::MemoryTexture {
    let (width, height) = (320usize, 180usize);
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            pixels.extend([
                (35 + x * 145 / width + usize::from(seed)) as u8,
                (90 + y * 100 / height + usize::from(seed) / 2) as u8,
                (175 + x * 40 / width).saturating_sub(usize::from(seed)) as u8,
            ]);
        }
    }
    gtk::gdk::MemoryTexture::new(
        width as i32,
        height as i32,
        gtk::gdk::MemoryFormat::R8g8b8,
        &gtk::glib::Bytes::from(&pixels),
        (width * 3) as usize,
    )
}

fn find_window(title: &str) -> gtk::Window {
    let find = || {
        gtk::Window::list_toplevels()
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Window>().ok())
            .find(|w| w.title().as_deref() == Some(title))
    };
    until(|| find().is_some());
    find().expect("window title")
}

fn click_toggle(window: &gtk::Window, label: &str) {
    fn find(w: &gtk::Widget, label: &str) -> Option<gtk::ToggleButton> {
        if w.is::<gtk::ToggleButton>() {
            let b = w.clone().downcast::<gtk::ToggleButton>().ok()?;
            if b.label().is_some_and(|v| v.starts_with(label)) {
                return Some(b);
            }
        }
        let mut child = w.first_child();
        while let Some(c) = child {
            if let Some(found) = find(&c, label) {
                return Some(found);
            }
            child = c.next_sibling();
        }
        None
    }
    find(window.upcast_ref(), label)
        .expect("files state toggle")
        .emit_clicked();
}

fn capture<W: IsA<gtk::Window>>(
    window: &W,
    dir: &Path,
    name: &str,
    width: i32,
    height: i32,
    screen: &str,
    theme: &str,
    dataset: &str,
    dataset_hash: &str,
    records: &mut Vec<Value>,
) {
    let window = window.as_ref();
    // GtkWindow's measured allocation excludes Adwaita's 8px frame on this Xvfb setup.
    window.set_default_size(width + 8, height + 8);
    window.present();
    settle(250);
    until(|| window.is_visible() && window.width() > 0 && window.height() > 0);
    settle(160);
    let path = dir.join(name);
    let _ = fs::remove_file(&path);
    save_png(window, &path);
    assert!(
        path.metadata().unwrap().len() > 0,
        "native GTK renderer must write a PNG"
    );
    records.push(json!({"file":name,"screen":screen,"theme":theme,"requested_width":width,"requested_height":height,"width":window.width(),"height":window.height(),"dataset":dataset,"dataset_sha256":dataset_hash}));
}

fn set_theme(settings: &gtk::Settings, dark: bool) {
    settings.set_gtk_theme_name(Some("Adwaita"));
    settings.set_gtk_application_prefer_dark_theme(dark);
    if let Some(property) = settings.find_property("gtk-interface-color-scheme") {
        let value = settings.property_value("gtk-interface-color-scheme");
        if let Some(class) = gtk::glib::EnumClass::with_type(value.type_()) {
            if let Some(value) = class.to_value_by_nick(if dark { "dark" } else { "light" }) {
                settings.set_property_from_value("gtk-interface-color-scheme", &value)
            }
        }
        drop(property)
    }
}

fn notes_json(state: &crate::app::shell::state::State) -> Vec<Value> {
    state
        .borrow()
        .notebook
        .notes()
        .iter()
        .map(|n| serde_json::to_value(n).unwrap())
        .collect()
}
fn export_dataset(
    output: &Path,
    name: &str,
    notes_path: &Path,
    notes: &[Value],
    expanded: Option<Value>,
) -> String {
    let dir = output.join("fixtures").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir.join("assets")).unwrap();
    let notes_bytes = serde_json::to_vec_pretty(notes).unwrap();
    fs::write(dir.join("notes.json"), &notes_bytes).unwrap();
    let source_assets = notes_path.with_file_name("assets");
    if source_assets.is_dir() {
        for entry in fs::read_dir(&source_assets).unwrap().flatten() {
            if entry.path().is_file() {
                fs::copy(entry.path(), dir.join("assets").join(entry.file_name())).unwrap();
            }
        }
    }
    let index = notes_path.with_file_name("assets.json");
    if index.exists() {
        fs::copy(index, dir.join("assets.json")).unwrap();
    }
    if let Some(expanded) = expanded {
        fs::write(
            dir.join("library.json"),
            serde_json::to_vec_pretty(&expanded).unwrap(),
        )
        .unwrap();
    }
    hash_dataset(&dir)
}
fn hash_dataset(dir: &Path) -> String {
    let mut files = Vec::new();
    fn gather(root: &Path, path: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let child = entry.path();
            if child.is_dir() {
                gather(root, &child, out);
            } else if child.is_file() {
                out.push(child.strip_prefix(root).unwrap().to_owned());
            }
        }
    }
    gather(dir, dir, &mut files);
    files.sort();
    let mut hash = Sha256::new();
    for relative in files {
        let rel = relative.to_string_lossy();
        hash.update((rel.len() as u64).to_le_bytes());
        hash.update(rel.as_bytes());
        let bytes = fs::read(dir.join(&relative)).unwrap();
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn write_manifest(
    dir: &Path,
    captures: Vec<Value>,
    fixture_hash: String,
    used_id: &str,
    unused_id: &str,
) {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_else(|| "unavailable".into())
    };
    let status = git(&["status", "--short"]);
    let revision = git(&["rev-parse", "HEAD"]);
    let gtk_version = Command::new("pkg-config")
        .args(["--modversion", "gtk4"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".into());
    let settings = gtk::Settings::default().expect("GTK settings");
    let font = settings
        .property_value("gtk-font-name")
        .get::<String>()
        .unwrap_or_else(|_| "system default".into());
    let mut captures = captures;
    for c in &mut captures {
        c["fixture_source"] = json!(FIXTURE_PATH);
        c["source_fixture_sha256"] = json!(fixture_hash);
        c["capture_command"] = json!(COMMAND);
        c["renderer"] = json!(RENDERER);
        c["gtk"] = json!(gtk_version);
        c["font"] = json!(font);
        c["source_git_revision"] = json!(revision);
        c["source_dirty"] = json!(!status.is_empty());
    }
    let manifest = json!({"baseline_id":"UI-REF-001","capture_command":COMMAND,"renderer":RENDERER,"captured_unix_seconds":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),"fixture":{"source":FIXTURE_PATH,"sha256":fixture_hash},"derived_assets":{"used_id":used_id,"unused_id":unused_id},"source":{"git_revision":revision,"dirty":!status.is_empty(),"git_status_short":status},"runtime":{"gtk":gtk_version,"font":font,"theme":"Adwaita light/dark; GTK animations disabled","display":"Xvfb x11"},"captures":captures});
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}
