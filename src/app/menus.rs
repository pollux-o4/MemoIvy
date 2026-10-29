use super::*;

pub(super) fn menu_for(state: &State, app: &Application, id: Uuid) -> gtk::MenuButton {
    let menu = gtk::MenuButton::new();
    menu.set_child(Some(&icons::image(icons::MORE, false)));
    menu.update_property(&[gtk::accessible::Property::Label("더 보기")]);
    menu.set_tooltip_text(Some("더 보기"));
    menu.add_css_class("icon-button");
    let pop = gtk::Popover::new();
    let delete = Button::with_label("삭제…");
    delete.add_css_class("lsm-action");
    let st = state.clone();
    let _a = app.clone();
    delete.connect_clicked(move |b| {
        let Some(parent) = b.root().and_downcast::<ApplicationWindow>() else {
            return;
        };
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&parent)
            .modal(true)
            .message_type(gtk::MessageType::Question)
            .buttons(gtk::ButtonsType::None)
            .text("이 메모를 삭제할까요?")
            .secondary_text("삭제한 메모는 복구할 수 없습니다.")
            .build();
        dialog.add_button("취소", gtk::ResponseType::Cancel);
        dialog.add_button("삭제", gtk::ResponseType::Accept);
        dialog.set_default_response(gtk::ResponseType::Cancel);
        let st = st.clone();
        dialog.connect_response(move |d, r| {
            if r == gtk::ResponseType::Accept {
                let mut s = st.borrow_mut();
                if s.operation.is_none() {
                    if let Some(editor) = s.windows.get(&id) {
                        editor.view.set_editable(false);
                    }
                    s.operation = Some((Operation::Delete(id), false));
                    s.add.set_sensitive(false);
                }
            }
            d.close();
        });
        dialog.present();
    });
    pop.set_child(Some(&delete));
    menu.set_popover(Some(&pop));
    menu
}
