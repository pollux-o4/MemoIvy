use super::*;
use std::time::{Duration, Instant};

fn until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    let context = glib::MainContext::default();
    while !condition() {
        while context.pending() {
            context.iteration(false);
        }
        assert!(Instant::now() < deadline, "preview state timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(in crate::app) fn note_004_preview_disclosure() {
    crate::presentation::install();
    let text = format!(
        "# 제목\n첫 줄\n두 번째 줄\n\n**강조**와 [링크](https://example.com)\n```\nfoo_bar = 1\n```\n{}\n마지막 문장",
        "긴 본문 ".repeat(100)
    );
    assert!(body(&text).starts_with("첫 줄\n두 번째 줄\n\n강조와 링크\nfoo_bar = 1"));
    assert!(
        body(&text).ends_with("마지막 문장"),
        "expanded text must exceed the old 240-character summary"
    );
    assert_eq!(body("제목만"), "");
    assert_eq!(body("\n제목\n\n한 줄\n"), "한 줄");

    let (widget, button) = build(&text, "제목");
    let preview = widget.downcast::<Preview>().unwrap();
    let contents = gtk::Box::new(gtk::Orientation::Vertical, 0);
    contents.append(&preview);
    contents.append(&button);
    let window = gtk::Window::builder()
        .default_width(360)
        .child(&contents)
        .build();
    window.present();
    until(|| preview.width() > 0 && button.is_visible());
    let imp = preview.imp();
    let settings = preview.settings();
    let original = settings.is_gtk_enable_animations();
    let (collapsed, full) = imp.heights(preview.width());
    assert!(full > collapsed);
    assert_eq!(preview.height(), collapsed);
    let passes = imp.layout_passes.get();
    for _ in 0..200 {
        assert_eq!(imp.heights(preview.width()), (collapsed, full));
    }
    assert_eq!(
        imp.layout_passes.get(),
        passes,
        "unchanged width/text/style must reuse shaping"
    );
    settings.set_gtk_enable_animations(true);
    button.emit_clicked();
    until(|| imp.progress.get() > 0. && imp.progress.get() < 1.);
    let mid = imp.progress.get();
    assert!(imp.tick.borrow().is_some());
    // Reverse an in-flight transition from its current height.
    button.emit_clicked();
    assert_eq!(imp.progress.get(), mid);
    until(|| imp.tick.borrow().is_none() && preview.height() == collapsed);
    assert!(!imp.expanded.get());
    assert_eq!(imp.progress.get(), 0.);
    button.emit_clicked();
    until(|| imp.tick.borrow().is_none() && preview.height() == full);
    assert_eq!(imp.progress.get(), 1.);
    // shortcut: Snap GTK 병렬 통합 검사에서 재배치 횟수 검증 1회 실패·단독/순차 재검사 통과, 병렬 CI 도입 전에 재현·원인 확인.
    assert!(
        imp.layout_passes.get() <= passes + 16,
        "animation must not reshape text every frame"
    );
    settings.set_gtk_enable_animations(false);
    button.emit_clicked();
    assert_eq!(imp.progress.get(), 0.);
    assert!(
        imp.tick.borrow().is_none(),
        "reduced motion must skip animation"
    );
    until(|| preview.height() == collapsed);

    imp.label.set_text("짧은 본문");
    preview.queue_resize();
    until(|| !button.is_visible());
    // Wrapping alone also needs a disclosure; resizing can remove it again.
    imp.label.set_text(&"가나다라 ".repeat(8));
    window.set_default_size(120, 1);
    preview.queue_resize();
    until(|| preview.width() < 200 && button.is_visible());
    window.set_default_size(1000, 1);
    until(|| preview.width() > 900 && !button.is_visible());
    let old = imp.heights(preview.width());
    let attributes = gtk::pango::AttrList::new();
    attributes.insert(gtk::pango::AttrFloat::new_scale(1.5));
    imp.label.set_attributes(Some(&attributes));
    preview.queue_resize();
    until(|| imp.heights(preview.width()).1 > old.1);
    assert!(
        !button.is_visible(),
        "font changes must invalidate measurements without inventing overflow"
    );
    settings.set_gtk_enable_animations(original);
    window.destroy();
}
