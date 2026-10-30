#[test]
fn fmt_001_live_editor_preserves_source_selection_undo_and_preedit() {
    gtk::init().expect("GTK tests require a display; run under xvfb-run");
    super::shell::lifecycle::save_002_close_failure_and_idle_timer();
    super::shared::focus::tests::focus_001_releases_and_restores_input_owner();
    super::canvas::tests::canvas_001_widget_flow_and_recovery();
    super::canvas::tests::perf_002_frames();
    super::editor::live_editor::tests::fmt_001_live_editor_preserves_source_selection_undo_and_preedit();
}
