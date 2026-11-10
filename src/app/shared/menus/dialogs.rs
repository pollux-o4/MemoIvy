use crate::app::shell::{commands, state::State};
use gtk::{prelude::*, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};
use uuid::Uuid;

/// NOTE-002 삭제 확인. 기본 동작은 취소이며 Esc·닫기도 취소다.
/// `name`을 주면 어떤 메모를 지우는지 함께 보여 준다.
pub(in crate::app) fn confirm_delete(
    parent: &ApplicationWindow,
    state: &State,
    ids: Vec<Uuid>,
    name: Option<String>,
) {
    let question = match ids.len() {
        1 => "이 메모를 삭제할까요?".to_owned(),
        count => format!("메모 {count}개를 삭제할까요?"),
    };
    let detail = if ids.len() > 1 {
        "삭제한 메모는 복구할 수 없습니다. 내용과 연결이 모두 사라져요."
    } else {
        "삭제한 메모는 복구할 수 없습니다."
    };
    let st = state.clone();
    confirm(
        parent,
        &question,
        name.as_deref(),
        detail,
        "삭제",
        move || commands::delete_notes(&st, ids.clone()),
    );
}

/// 되돌릴 수 없는 동작의 공용 확인창. 취소가 기본 응답이다.
pub(in crate::app) fn confirm(
    parent: &impl IsA<gtk::Window>,
    question: &str,
    subject: Option<&str>,
    detail: &str,
    accept_label: &str,
    on_accept: impl Fn() + 'static,
) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(question)
        .default_width(340)
        .resizable(false)
        .build();
    dialog.add_css_class("memoivy-confirm");
    // 시스템 제목 표시줄 없이 내용만 보인다.
    dialog.set_titlebar(Some(&GtkBox::new(Orientation::Horizontal, 0)));
    let body = GtkBox::new(Orientation::Vertical, 6);
    body.add_css_class("confirm-body");
    let title = Label::new(Some(question));
    title.set_xalign(0.);
    title.add_css_class("confirm-title");
    body.append(&title);
    if let Some(name) = subject {
        let subject = Label::new(Some(name));
        subject.set_xalign(0.);
        subject.set_ellipsize(gtk::pango::EllipsizeMode::End);
        subject.set_max_width_chars(36);
        subject.add_css_class("confirm-subject");
        body.append(&subject);
    }
    let detail = Label::new(Some(detail));
    detail.set_wrap(true);
    detail.set_xalign(0.);
    detail.add_css_class("confirm-detail");
    body.append(&detail);
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_homogeneous(true);
    actions.set_margin_top(14);
    let cancel = Button::with_label("취소");
    cancel.add_css_class("confirm-cancel");
    let accept = Button::with_label(accept_label);
    accept.add_css_class("confirm-accept");
    actions.append(&cancel);
    actions.append(&accept);
    body.append(&actions);
    dialog.set_child(Some(&body));
    dialog.set_default_widget(Some(&cancel));
    let weak = dialog.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    let weak = dialog.downgrade();
    accept.connect_clicked(move |_| {
        on_accept();
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    super::super::close_on_escape(&dialog);
    dialog.present();
    cancel.grab_focus();
}

/// 복구 안내 등 확인 응답만 필요한 공용 창.
pub(in crate::app) fn message(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .default_width(380)
        .build();
    let body = GtkBox::new(Orientation::Vertical, 16);
    body.set_margin_start(20);
    body.set_margin_end(20);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    let label = Label::new(Some(message));
    label.set_wrap(true);
    body.append(&label);
    let close = Button::with_label("닫기");
    body.append(&close);
    dialog.set_child(Some(&body));
    let weak = dialog.downgrade();
    close.connect_clicked(move |_| {
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    super::super::close_on_escape(&dialog);
    dialog.present();
}
