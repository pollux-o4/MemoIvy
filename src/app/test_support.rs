use gtk::{prelude::*, Widget};
use std::time::{Duration, Instant};

pub(super) fn drain() {
    let context = gtk::glib::MainContext::default();
    let end = Instant::now() + Duration::from_millis(25);
    while context.pending() && Instant::now() < end {
        context.iteration(false);
    }
}

/// Keep pumping GTK for assertions about an interval with no state change.
pub(super) fn settle(ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    loop {
        drain();
        if Instant::now() >= end {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[track_caller]
pub(super) fn until(mut done: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        drain();
        if done() {
            return;
        }
        assert!(Instant::now() < end, "GTK state timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn find(widget: &Widget, matches: &impl Fn(&Widget) -> bool) -> Option<Widget> {
    if matches(widget) {
        return Some(widget.clone());
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        if let Some(found) = find(&widget, matches) {
            return Some(found);
        }
        child = widget.next_sibling();
    }
    None
}

pub(super) fn named(widget: &Widget, name: &str) -> Option<Widget> {
    crate::app::shared::find_named(widget, name)
}

pub(super) fn button_with_class(widget: &Widget, class: &str) -> Option<gtk::Button> {
    find(widget, &|w| w.has_css_class(class) && w.is::<gtk::Button>()).and_downcast::<gtk::Button>()
}

pub(super) fn find_checkbox(widget: &Widget) -> Option<gtk::CheckButton> {
    find(widget, &|w| w.is::<gtk::CheckButton>()).and_downcast::<gtk::CheckButton>()
}

pub(super) fn count(widget: &Widget, class: &str) -> usize {
    let mut total = usize::from(widget.is_visible() && widget.has_css_class(class));
    let mut child = widget.first_child();
    while let Some(widget) = child {
        total += count(&widget, class);
        child = widget.next_sibling();
    }
    total
}

/// 실제 CSS로 그린 창을 PNG로 남긴다. 화면 검토용 산출물이며 판정에 쓰지 않는다.
pub(super) fn save_png(widget: &impl IsA<Widget>, path: &std::path::Path) {
    let widget = widget.as_ref();
    let paintable = gtk::WidgetPaintable::new(Some(widget));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, widget.width() as f64, widget.height() as f64);
    let (Some(node), Some(renderer)) = (
        snapshot.to_node(),
        widget.native().and_then(|n| n.renderer()),
    ) else {
        return;
    };
    renderer
        .render_texture(&node, None)
        .save_to_png(path)
        .unwrap();
}

pub(super) fn find_class(widget: &Widget, class: &str) -> Option<Widget> {
    find(widget, &|w| w.has_css_class(class))
}
