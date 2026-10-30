#[test]
fn fmt_001_live_editor_preserves_source_selection_undo_and_preedit() {
    gtk::init().expect("GTK tests require a display; run under xvfb-run");
    // 프레임 표본을 다른 GTK 시나리오의 창·렌더러 캐시와 분리한다.
    if std::env::var("LSM_CANVAS_PERF").is_ok() {
        super::canvas::tests::perf_002_frames();
        return;
    }
    super::shell::lifecycle::save_002_close_failure_and_idle_timer();
    super::shared::focus::tests::focus_001_releases_and_restores_input_owner();
    super::canvas::tests::canvas_001_widget_flow_and_recovery();
    super::editor::live_editor::tests::fmt_001_live_editor_preserves_source_selection_undo_and_preedit();
}
