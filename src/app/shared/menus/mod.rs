//! Shared menu construction, menu behavior, and modal responses.

mod dialogs;
mod note;
mod rows;

pub(in crate::app) use dialogs::{confirm, confirm_delete, message};
pub(in crate::app) use note::{lazy_menu_for, menu_for};
pub(in crate::app) use rows::{menu_row, source_row};

use gtk::prelude::*;

/// 모든 팝오버의 공통 모양과 등장 모션. 시작 클래스를 다음 프레임에 지워 CSS 전환을 일으킨다.
pub(in crate::app) fn menu_popover(popover: &gtk::Popover) {
    popover.set_has_arrow(false);
    popover.add_css_class("memoivy-menu");
    popover.connect_closed(|popover| {
        if let Some(parent) = popover.parent() {
            parent.set_has_tooltip(true);
        }
    });
    popover.connect_show(|popover| {
        // 메뉴가 열린 동안 버튼 툴팁이 항목을 가리지 않게 한다.
        if let Some(parent) = popover.parent() {
            parent.set_has_tooltip(false);
        }
        popover.add_css_class("opening");
        popover.add_tick_callback(|widget, _| {
            widget.remove_css_class("opening");
            gtk::glib::ControlFlow::Break
        });
    });
}
