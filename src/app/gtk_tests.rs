use std::{process::Command, sync::Mutex};

// Each GTK scenario owns a fresh process; parent tests serialize display use.
static DISPLAY: Mutex<()> = Mutex::new(());

fn isolated(name: &str, run: fn()) {
    let exact = format!("app::gtk_tests::{name}");
    if std::env::var("LSM_GTK_SCENARIO").as_deref() == Ok(exact.as_str()) {
        gtk::init().expect("GTK tests require a display; run under xvfb-run");
        run();
        return;
    }
    let _display = DISPLAY.lock().unwrap_or_else(|poison| poison.into_inner());
    let exe = std::env::current_exe().expect("test executable");
    // 사용자 화면에 창을 띄우지 않는다. xvfb-run 밖에서 실행되면 가상 화면과 격리된 D-Bus로 감싼다.
    let headless = std::env::var("XAUTHORITY").is_ok_and(|auth| auth.contains("xvfb-run"));
    let mut command = if headless {
        Command::new(&exe)
    } else {
        let mut wrapped = Command::new("dbus-run-session");
        wrapped.args(["--", "xvfb-run", "-a"]).arg(&exe);
        wrapped
    };
    let output = command
        .args(["--exact", &exact, "--nocapture"])
        // xvfb-run sets DISPLAY, but GTK otherwise prefers the host Wayland socket.
        .env(
            "GDK_BACKEND",
            std::env::var("GDK_BACKEND").unwrap_or_else(|_| "x11".into()),
        )
        .env("LSM_GTK_SCENARIO", &exact)
        .output()
        .expect("run isolated GTK scenario");
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{name}: {}", output.status);
}

macro_rules! scenario {
    ($name:ident, $run:path) => {
        #[test]
        fn $name() {
            if std::env::var("LSM_CANVAS_PERF").is_err() {
                isolated(stringify!($name), $run);
            }
        }
    };
}

#[test]
fn fmt_001_live_editor_preserves_source_selection_undo_and_preedit() {
    isolated(
        "fmt_001_live_editor_preserves_source_selection_undo_and_preedit",
        if std::env::var("LSM_CANVAS_PERF").is_ok() {
            super::canvas::tests::perf_002_frames
        } else {
            super::editor::live_editor::tests::fmt_001_live_editor_preserves_source_selection_undo_and_preedit
        },
    );
}

scenario!(
    save_002_close_failure_and_idle_timer,
    super::shell::lifecycle::save_002_close_failure_and_idle_timer
);
scenario!(
    focus_001_releases_and_restores_input_owner,
    super::shared::focus::tests::focus_001_releases_and_restores_input_owner
);
scenario!(
    canvas_001_widget_flow_and_recovery,
    super::canvas::tests::canvas_001_widget_flow_and_recovery
);
scenario!(
    project_001_levels_grouping_and_shortcuts,
    super::canvas::tests::project_001_levels_grouping_and_shortcuts
);
scenario!(
    canvas_004_markers_shortcuts_and_delete,
    super::canvas::tests::canvas_004_markers_shortcuts_and_delete
);
scenario!(
    fmt_003_list_layout_and_hit_testing,
    super::editor::live_editor::tests::fmt_003_list_layout_and_hit_testing
);
scenario!(
    fmt_003_typing_flow,
    super::editor::live_editor::tests::fmt_003_typing_flow
);
scenario!(
    fmt_003_link_preview_lifecycle,
    super::editor::live_editor::tests::fmt_003_link_preview_lifecycle
);
