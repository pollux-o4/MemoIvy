use gtk::prelude::*;

pub(crate) fn is_dark(settings: &gtk::Settings) -> bool {
    std::env::var("GTK_THEME").is_ok_and(|v| v.to_lowercase().contains("dark"))
        || settings.is_gtk_application_prefer_dark_theme()
        || settings
            .gtk_theme_name()
            .is_some_and(|n| n.to_lowercase().contains("dark"))
}
pub(crate) fn watch(root: &gtk::Widget) {
    fn apply(settings: &gtk::Settings, root: &gtk::Widget) {
        let is_dark = self::is_dark(settings);
        if is_dark {
            root.add_css_class("dark");
        } else {
            root.remove_css_class("dark");
        }
    }
    let Some(settings) = gtk::Settings::default() else {
        return;
    };
    apply(&settings, root);
    for property in ["gtk-application-prefer-dark-theme", "gtk-theme-name"] {
        let root = root.downgrade();
        settings.connect_notify_local(Some(property), move |settings, _| {
            if let Some(root) = root.upgrade() {
                apply(settings, &root);
            }
        });
    }
}
