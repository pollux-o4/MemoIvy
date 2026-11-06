//! FMT-004 크게 보기. Quick Look처럼 살짝 작게 시작해 펴지고, 닫을 때는 짧게 사라진다.
use gtk::{gdk, glib, prelude::*};
use std::time::Duration;

/// 화면 맞춤과 원래 크기 사이를 클릭으로 오간다. Esc·배경 클릭·닫기 버튼으로 닫는다.
pub(in crate::app) fn show_image(
    parent: &impl IsA<gtk::Window>,
    texture: &gdk::Texture,
    title: &str,
) {
    let parent = parent.as_ref();
    let (fit_w, fit_h) = fitted(parent, texture);
    let window = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(if title.is_empty() { "이미지" } else { title })
        .default_width(fit_w)
        .default_height(fit_h)
        .build();
    if let Some(app) = parent.application() {
        window.set_application(Some(&app));
    }
    window.add_css_class("image-viewer");
    // Quick Look처럼 제목 표시줄 없이 이미지만 띄운다. 닫기는 버튼·Esc·배경 클릭이다.
    window.set_decorated(false);

    let picture = gtk::Picture::for_paintable(texture);
    picture.set_content_fit(gtk::ContentFit::Contain);
    picture.set_can_shrink(true);
    picture.set_cursor_from_name(Some("zoom-in"));
    picture.update_property(&[gtk::accessible::Property::Label(if title.is_empty() {
        "이미지"
    } else {
        title
    })]);
    let scroll = gtk::ScrolledWindow::builder()
        .child(&picture)
        .hexpand(true)
        .vexpand(true)
        .build();
    scroll.add_css_class("viewer-stage");
    scroll.add_css_class("entering");

    let close = gtk::Button::new();
    close.set_child(Some(&crate::presentation::icons::image(
        crate::presentation::icons::CLOSE,
    )));
    close.add_css_class("viewer-close");
    close.set_tooltip_text(Some("닫기 · Esc"));
    close.update_property(&[gtk::accessible::Property::Label("닫기")]);
    close.set_halign(gtk::Align::End);
    close.set_valign(gtk::Align::Start);

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&scroll));
    overlay.add_overlay(&close);
    window.set_child(Some(&overlay));

    let zoom = gtk::GestureClick::new();
    let shown = picture.downgrade();
    zoom.connect_released(move |gesture, _, _, _| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        if let Some(picture) = shown.upgrade() {
            let original = picture.can_shrink();
            picture.set_can_shrink(!original);
            picture.set_cursor_from_name(Some(if original { "zoom-out" } else { "zoom-in" }));
        }
    });
    picture.add_controller(zoom);

    let backdrop = gtk::GestureClick::new();
    let weak = window.downgrade();
    backdrop.connect_released(move |_, _, _, _| {
        if let Some(window) = weak.upgrade() {
            dismiss(&window);
        }
    });
    scroll.add_controller(backdrop);
    let weak = window.downgrade();
    close.connect_clicked(move |_| {
        if let Some(window) = weak.upgrade() {
            dismiss(&window);
        }
    });
    let keys = gtk::EventControllerKey::new();
    let weak = window.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(window) = weak.upgrade() {
                dismiss(&window);
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(keys);

    window.present();
    // 시작 클래스를 다음 프레임에 지워 CSS 전환을 일으킨다(menu_popover와 같은 방식).
    scroll.add_tick_callback(|stage, _| {
        stage.remove_css_class("entering");
        glib::ControlFlow::Break
    });
}

/// 원래 크기가 화면의 85%를 넘으면 비율을 지켜 줄인다.
fn fitted(parent: &gtk::Window, texture: &gdk::Texture) -> (i32, i32) {
    let (max_w, max_h) = parent
        .surface()
        .and_then(|surface| WidgetExt::display(parent).monitor_at_surface(&surface))
        .map(|monitor| {
            let area = monitor.geometry();
            (area.width() * 85 / 100, area.height() * 85 / 100)
        })
        .unwrap_or((1200, 800));
    let (w, h) = (texture.width().max(1), texture.height().max(1));
    let scale = (f64::from(max_w) / f64::from(w))
        .min(f64::from(max_h) / f64::from(h))
        .min(1.0);
    (
        ((f64::from(w) * scale) as i32).max(240),
        ((f64::from(h) * scale) as i32).max(160),
    )
}

fn dismiss(window: &gtk::Window) {
    if !super::animations_enabled() {
        window.close();
        return;
    }
    if let Some(stage) = window.child().and_then(|overlay| overlay.first_child()) {
        stage.add_css_class("leaving");
    }
    let weak = window.downgrade();
    glib::timeout_add_local_once(Duration::from_millis(120), move || {
        if let Some(window) = weak.upgrade() {
            window.close();
        }
    });
}
