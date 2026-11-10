//! UI-001: 문자 없이 상태를 전달하는 공통 펼침 제어. 문구는 보조 설명에만 쓴다.
use crate::presentation::icons;
use gtk::{prelude::*, Button};

pub(super) fn button(context: &str) -> Button {
    let button = Button::new();
    let mark = icons::image(icons::CHEVRON);
    mark.add_css_class("reveal-mark");
    button.set_child(Some(&mark));
    button.add_css_class("card-reveal");
    button.set_halign(gtk::Align::Center);
    reflect(&button, false, context);
    button
}

pub(super) fn reflect(button: &Button, expanded: bool, context: &str) {
    if expanded {
        button.add_css_class("expanded");
    } else {
        button.remove_css_class("expanded");
    }
    let languages = gtk::glib::language_names();
    let action = action(
        expanded,
        languages.first().map(|s| s.as_str()).unwrap_or("en"),
    );
    button.set_tooltip_text(Some(action));
    button.update_property(&[gtk::accessible::Property::Label(&format!(
        "{context}: {action}"
    ))]);
    button.update_state(&[gtk::accessible::State::Expanded(Some(expanded))]);
}

fn action(expanded: bool, language: &str) -> &'static str {
    match (
        expanded,
        language.split(['_', '-', '.', '@']).next() == Some("ko"),
    ) {
        (false, true) => "펼치기",
        (true, true) => "접기",
        (false, false) => "Expand",
        (true, false) => "Collapse",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ui_001_disclosure_actions_follow_locale_with_english_fallback() {
        assert_eq!(action(false, "ko_KR.UTF-8"), "펼치기");
        assert_eq!(action(true, "ko"), "접기");
        assert_eq!(action(false, "en_US"), "Expand");
        assert_eq!(action(true, "ja_JP"), "Collapse");
        assert_eq!(action(false, "kok_IN"), "Expand");
    }
}
