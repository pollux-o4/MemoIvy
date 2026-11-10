use super::{confirm_delete, menu_popover, menu_row, rows::more_button};
use crate::app::shell::{commands, state::State};
use crate::presentation::icons;
use gtk::{prelude::*, Application, ApplicationWindow, Box as GtkBox, Orientation};
use uuid::Uuid;

fn popover_for(state: &State, app: &Application, id: Uuid) -> gtk::Popover {
    let pop = gtk::Popover::new();
    let body = GtkBox::new(Orientation::Vertical, 0);
    body.add_css_class("menu-body");
    let files = menu_row(icons::FILES, "파일", None);
    files.set_tooltip_text(Some("메모에 넣은 이미지와 사용처"));
    let (st, a, close) = (state.clone(), app.clone(), pop.downgrade());
    files.connect_clicked(move |_| {
        if let Some(popover) = close.upgrade() {
            popover.popdown();
        }
        commands::open_files(&st, &a);
    });
    body.append(&files);
    let divider = gtk::Separator::new(Orientation::Horizontal);
    divider.add_css_class("menu-divider");
    body.append(&divider);
    let delete = menu_row(icons::TRASH, "삭제…", None);
    delete.add_css_class("destructive");
    let st = state.clone();
    let close = pop.downgrade();
    delete.connect_clicked(move |b| {
        let Some(parent) = b.root().and_downcast::<ApplicationWindow>() else {
            return;
        };
        if let Some(popover) = close.upgrade() {
            popover.popdown();
        }
        confirm_delete(&parent, &st, vec![id], None);
    });
    body.append(&delete);
    pop.set_child(Some(&body));
    menu_popover(&pop);
    pop
}

pub(in crate::app) fn menu_for(state: &State, app: &Application, id: Uuid) -> gtk::MenuButton {
    let menu = more_button();
    menu.set_popover(Some(&popover_for(state, app, id)));
    menu
}

/// 메뉴를 처음 열 때 만든다. 긴 목록의 항목마다 팝오버를 미리 두지 않는다.
pub(in crate::app) fn lazy_menu_for(state: &State, app: &Application, id: Uuid) -> gtk::MenuButton {
    let menu = more_button();
    let state = state.clone();
    let app = app.clone();
    menu.set_create_popup_func(move |menu| {
        if menu.popover().is_none() {
            menu.set_popover(Some(&popover_for(&state, &app, id)));
        }
    });
    menu
}
