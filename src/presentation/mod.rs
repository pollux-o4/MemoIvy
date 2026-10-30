pub(crate) mod icons;
pub(crate) mod markdown;
pub(crate) mod theme;

use gtk::prelude::*;

const LIGHT: &str = include_str!("styles/tokens-light.css");
const DARK: &str = include_str!("styles/tokens-dark.css");
const STYLES: &str = concat!(
    include_str!("styles/base.css"),
    include_str!("styles/menus.css"),
    include_str!("styles/library.css"),
    include_str!("styles/memo.css"),
    include_str!("styles/canvas.css"),
);

/// 토큰만 테마별로 교체한다. 구성요소 스타일은 한 벌만 유지한다.
pub(crate) fn install() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let tokens = gtk::CssProvider::new();
    let provider = gtk::CssProvider::new();
    provider.load_from_data(STYLES);
    gtk::style_context_add_provider_for_display(
        &display,
        &tokens,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let Some(settings) = gtk::Settings::default() else {
        tokens.load_from_data(LIGHT);
        return;
    };
    let apply = move |settings: &gtk::Settings| {
        tokens.load_from_data(if theme::is_dark(settings) {
            DARK
        } else {
            LIGHT
        });
    };
    apply(&settings);
    for property in theme::WATCHED {
        if settings.find_property(property).is_some() {
            let apply = apply.clone();
            settings.connect_notify_local(Some(property), move |settings, _| apply(settings));
        }
    }
}
