use gtk::prelude::*;

const SCHEME: &str = "gtk-interface-color-scheme";

fn prefers_dark_scheme(settings: &gtk::Settings) -> bool {
    // GTK 4.20 이상의 시스템 색상 설정. 이전 버전에는 속성이 없다.
    settings.find_property(SCHEME).is_some()
        && gtk::glib::EnumValue::from_value(&settings.property_value(SCHEME))
            .is_some_and(|(_, value)| value.nick() == "dark")
}

pub(crate) fn is_dark(settings: &gtk::Settings) -> bool {
    std::env::var("GTK_THEME").is_ok_and(|v| v.to_lowercase().contains("dark"))
        || settings.is_gtk_application_prefer_dark_theme()
        || settings
            .gtk_theme_name()
            .is_some_and(|n| n.to_lowercase().contains("dark"))
        || prefers_dark_scheme(settings)
}

pub(crate) const WATCHED: [&str; 3] = [
    "gtk-application-prefer-dark-theme",
    "gtk-theme-name",
    SCHEME,
];
