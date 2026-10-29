use crate::app::{
    controller::{dispatch, mark},
    state::{EditorWindow, State},
    widgets::{header, menu_for, refresh},
};
use crate::persistence::store;
use crate::presentation::{icons, theme::watch as dark};
use gtk::{
    glib, prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation,
    ScrolledWindow, TextView,
};
use uuid::Uuid;

pub(in crate::app) fn open_note(state: &State, app: &Application, id: Uuid) {
    if let Some(editor) = state.borrow().windows.get(&id) {
        editor.window.present();
        return;
    }
    let Some(note) = state
        .borrow()
        .store
        .notes
        .iter()
        .find(|n| n.id == id)
        .cloned()
    else {
        return;
    };
    let window = ApplicationWindow::builder()
        .application(app)
        .title("메모")
        .default_width(380)
        .default_height(420)
        .build();
    super::focus::watch(&window);
    super::shortcuts::attach(&window, state);
    let (header_bar, actions) = header(&window, "메모");
    header_bar.add_css_class(&note.color);
    dark(header_bar.upcast_ref());
    let outer = GtkBox::new(Orientation::Vertical, 0);
    outer.add_css_class("memo");
    outer.add_css_class(&note.color);
    dark(outer.upcast_ref());
    let palette = gtk::MenuButton::new();
    palette.set_child(Some(&icons::image(icons::PALETTE, false)));
    palette.update_property(&[gtk::accessible::Property::Label("메모 색상")]);
    palette.set_tooltip_text(Some("메모 색상"));
    palette.add_css_class("icon-button");
    let pop = gtk::Popover::new();
    let choices = GtkBox::new(Orientation::Vertical, 8);
    choices.set_margin_top(12);
    choices.set_margin_bottom(12);
    choices.set_margin_start(12);
    choices.set_margin_end(12);
    choices.append(&Label::new(Some("메모 색상")));
    let colors = GtkBox::new(Orientation::Horizontal, 6);
    let mut group: Option<gtk::ToggleButton> = None;
    for (name, title) in store::COLORS {
        let b = gtk::ToggleButton::with_label(if name == note.color { "✓" } else { "" });
        b.add_css_class("swatch");
        b.add_css_class(name);
        b.set_tooltip_text(Some(title));
        b.update_property(&[gtk::accessible::Property::Label(title)]);
        if let Some(g) = &group {
            b.set_group(Some(g));
        } else {
            group = Some(b.clone());
        }
        b.set_active(name == note.color);
        let st = state.clone();
        let a = app.clone();
        let root = outer.clone();
        let header_bar = header_bar.clone();
        let popup = pop.clone();
        b.connect_toggled(move |b| {
            b.set_label(if b.is_active() { "✓" } else { "" });
            if !b.is_active() {
                return;
            }
            for (c, _) in store::COLORS {
                root.remove_css_class(c);
                header_bar.remove_css_class(c);
            }
            root.add_css_class(name);
            header_bar.add_css_class(name);
            if let Some(n) = st.borrow_mut().store.notes.iter_mut().find(|n| n.id == id) {
                n.color = name.into();
                n.modified = store::now();
            }
            mark(&st);
            refresh(&st, &a);
            popup.popdown();
        });
        colors.append(&b);
    }
    choices.append(&colors);
    pop.set_child(Some(&choices));
    palette.set_popover(Some(&pop));
    actions.append(&palette);
    let view = TextView::new();
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_left_margin(20);
    view.set_right_margin(20);
    view.set_top_margin(18);
    view.set_bottom_margin(20);
    view.set_pixels_below_lines(4);
    view.buffer().set_text(&note.text);
    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&view)
        .build();
    let source = crate::app::widgets::format_editor(&view);
    let menu = menu_for(state, app, id);
    let popover = menu.popover().unwrap();
    let delete = popover.child().unwrap();
    popover.set_child(None::<&gtk::Widget>);
    let options = GtkBox::new(Orientation::Vertical, 8);
    options.set_margin_top(8);
    options.set_margin_bottom(8);
    options.set_margin_start(8);
    options.set_margin_end(8);
    options.append(&source);
    let shortcuts = Button::with_label("단축키 · F1");
    shortcuts.add_css_class("lsm-action");
    let weak = window.downgrade();
    shortcuts.connect_clicked(move |_| {
        if let Some(window) = weak.upgrade() {
            super::shortcuts::help(&window);
        }
    });
    options.append(&shortcuts);
    let help = gtk::Expander::new(Some("서식 도움말"));
    let help_text = Label::new(Some("# 제목 · ## 소제목\n**굵게** · *기울임* · ~~취소선~~\n- 목록 · 1. 번호 목록\n- [ ] 할 일 · - [x] 완료\n[링크 이름](https://example.com)\n체크: 클릭 / Ctrl+Enter\n링크: Ctrl+클릭\n원문: Ctrl+Shift+M"));
    help_text.set_xalign(0.);
    help_text.set_margin_top(8);
    help.set_child(Some(&help_text));
    options.append(&help);
    options.append(&gtk::Separator::new(Orientation::Horizontal));
    options.append(&delete);
    popover.set_child(Some(&options));
    let popup = popover.downgrade();
    source.connect_toggled(move |_| {
        if let Some(popover) = popup.upgrade() {
            popover.popdown();
        }
    });
    actions.append(&menu);
    outer.append(&scroll);
    let status = Label::new(Some("자동 저장됨"));
    status.set_xalign(0.);
    status.set_wrap(true);
    status.add_css_class("status");
    let status_row = GtkBox::new(Orientation::Horizontal, 4);
    status.set_hexpand(true);
    status_row.append(&status);
    let retry = Button::with_label("다시 시도");
    retry.add_css_class("lsm-action");
    retry.set_visible(false);
    let retry_state = state.clone();
    retry.connect_clicked(move |_| {
        let mut s = retry_state.borrow_mut();
        s.queue.retry();
        dispatch(&mut s);
    });
    status_row.append(&retry);
    outer.append(&status_row);
    window.set_child(Some(&outer));
    let st = state.clone();
    view.buffer().connect_changed(move |buffer| {
        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string();
        if let Some(n) = st.borrow_mut().store.notes.iter_mut().find(|n| n.id == id) {
            n.text = text;
            n.modified = store::now();
        }
        mark(&st);
    });
    let st = state.clone();
    window.connect_close_request(move |_| {
        let mut s = st.borrow_mut();
        if s.bypass || !s.store.notes.iter().any(|n| n.id == id) {
            s.windows.remove(&id);
            return glib::Propagation::Proceed;
        }
        if s.queue.clean() && s.operation.is_none() {
            s.windows.remove(&id);
            if s.windows.is_empty() {
                s.library.present();
            }
            return glib::Propagation::Proceed;
        }
        if !s.closing.contains(&id) {
            s.closing.push(id);
        }
        if let Some(editor) = s.windows.get(&id) {
            editor.view.set_editable(false);
        }
        s.queue.retry();
        if s.operation.is_none() {
            dispatch(&mut s);
        }
        glib::Propagation::Stop
    });
    state.borrow_mut().windows.insert(
        id,
        EditorWindow {
            window: window.clone(),
            view: view.clone(),
            status,
            retry,
        },
    );
    window.present();
    view.grab_focus();
}
