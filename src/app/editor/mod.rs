mod formatting;
mod link_preview;
mod list_edit;
pub(in crate::app) mod live_editor;
use crate::app::{
    shared::{header, menu_for, COLORS},
    shell::{
        commands,
        lifecycle::{close_editor, flush, mark},
        state::{EditorWindow, State},
        views::refresh_all as refresh,
    },
};
use crate::presentation::icons;
use gtk::{
    prelude::*, Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation,
    ScrolledWindow,
};
use uuid::Uuid;

pub(in crate::app) fn open_note(state: &State, app: &Application, id: Uuid) {
    if let Some(editor) = state.borrow().views.windows.get(&id) {
        editor.window.present();
        return;
    }
    let Some(note) = state.borrow().notebook.note(id).cloned() else {
        return;
    };
    let kind = if note.canvas().project {
        "프로젝트"
    } else {
        "메모"
    };
    let window = ApplicationWindow::builder()
        .application(app)
        .title(kind)
        .default_width(380)
        .default_height(420)
        .build();
    crate::app::shared::focus::watch(&window);
    crate::app::shared::shortcuts::attach(&window, state);
    let (header_bar, actions) = header(&window, kind);
    header_bar.add_css_class(note.color());
    let outer = GtkBox::new(Orientation::Vertical, 0);
    outer.add_css_class("memo");
    outer.add_css_class(note.color());
    let palette = palette(state, app, id, note.color(), &outer, &header_bar);
    let canvas = crate::app::shared::canvas_button("캔버스에서 메모 보기");
    let st = state.clone();
    let a = app.clone();
    canvas.connect_clicked(move |_| commands::open_canvas(&st, &a, Some(id)));
    actions.append(&canvas);
    actions.append(&palette);
    let view = live_editor::new_view();
    view.buffer().set_text(note.text());
    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&view)
        .build();
    let st = state.clone();
    let media = live_editor::MediaHost {
        store: state.borrow().attachments.store(),
        notice: Box::new(move |text| {
            if let Some(editor) = st.borrow().views.windows.get(&id) {
                editor.show_status(text, false);
            }
        }),
    };
    let source = live_editor::attach(&view, media);
    let menu = editor_menu(state, app, id, &window, &source);
    actions.append(&menu);
    outer.append(&scroll);
    let status = Label::new(Some("저장됨"));
    status.set_xalign(0.);
    status.set_wrap(true);
    status.add_css_class("status");
    let status_row = GtkBox::new(Orientation::Horizontal, 4);
    status.set_hexpand(true);
    status_row.append(&status);
    let retry = Button::with_label("다시 시도");
    retry.add_css_class("memoivy-action");
    retry.set_visible(false);
    let retry_state = state.clone();
    retry.connect_clicked(move |_| {
        flush(&retry_state);
    });
    status_row.append(&retry);
    outer.append(&status_row);
    window.set_child(Some(&outer));
    let st = state.clone();
    view.buffer().connect_changed(move |buffer| {
        let text = live_editor::plain_text(buffer, &buffer.start_iter(), &buffer.end_iter());
        if st
            .borrow()
            .notebook
            .note(id)
            .is_some_and(|note| note.text() == text)
        {
            return;
        }
        st.borrow_mut().notebook.set_text(id, text);
        mark(&st);
    });
    let st = state.clone();
    window.connect_close_request(move |_| close_editor(&st, id));
    state.borrow_mut().views.windows.insert(
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

fn palette(
    state: &State,
    app: &Application,
    id: Uuid,
    color: &str,
    outer: &GtkBox,
    header_bar: &GtkBox,
) -> gtk::MenuButton {
    let palette = gtk::MenuButton::new();
    palette.set_child(Some(&icons::image(icons::PALETTE)));
    palette.update_property(&[gtk::accessible::Property::Label("메모 색상")]);
    palette.set_tooltip_text(Some("메모 색상"));
    palette.add_css_class("icon-button");
    let pop = gtk::Popover::new();
    let choices = GtkBox::new(Orientation::Vertical, 8);
    choices.add_css_class("menu-body");
    choices.add_css_class("palette-body");
    let choices_title = Label::new(Some("메모 색상"));
    choices_title.set_xalign(0.);
    choices_title.add_css_class("menu-hint");
    choices.append(&choices_title);
    let colors = GtkBox::new(Orientation::Horizontal, 6);
    let mut group: Option<gtk::ToggleButton> = None;
    for (name, title) in COLORS {
        let b = gtk::ToggleButton::with_label(if name == color { "✓" } else { "" });
        b.add_css_class("swatch");
        b.add_css_class(name);
        b.set_tooltip_text(Some(title));
        b.update_property(&[gtk::accessible::Property::Label(title)]);
        if let Some(g) = &group {
            b.set_group(Some(g));
        } else {
            group = Some(b.clone());
        }
        b.set_active(name == color);
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
            for (c, _) in COLORS {
                root.remove_css_class(c);
                header_bar.remove_css_class(c);
            }
            root.add_css_class(name);
            header_bar.add_css_class(name);
            st.borrow_mut().notebook.set_color(id, name);
            mark(&st);
            refresh(&st, &a);
            popup.popdown();
        });
        colors.append(&b);
    }
    choices.append(&colors);
    pop.set_child(Some(&choices));
    crate::app::shared::menu_popover(&pop);
    palette.set_popover(Some(&pop));
    palette
}

fn editor_menu(
    state: &State,
    app: &Application,
    id: Uuid,
    window: &ApplicationWindow,
    source: &gtk::CheckButton,
) -> gtk::MenuButton {
    let menu = menu_for(state, app, id);
    let popover = menu.popover().unwrap();
    let options = popover.child().and_downcast::<GtkBox>().unwrap();
    crate::app::shared::source_row(source);
    let shortcuts = crate::app::shared::menu_row(icons::KEYBOARD, "단축키·서식 도움말", Some("F1"));
    let weak = window.downgrade();
    let popup = popover.downgrade();
    shortcuts.connect_clicked(move |_| {
        if let Some(popover) = popup.upgrade() {
            popover.popdown();
        }
        if let Some(window) = weak.upgrade() {
            crate::app::shared::shortcuts::help(&window);
        }
    });
    options.insert_child_after(source, None::<&gtk::Widget>);
    options.insert_child_after(&shortcuts, Some(source));
    let divider = gtk::Separator::new(Orientation::Horizontal);
    divider.add_css_class("menu-divider");
    options.insert_child_after(&divider, Some(&shortcuts));
    let popup = popover.downgrade();
    source.connect_toggled(move |_| {
        if let Some(popover) = popup.upgrade() {
            popover.popdown();
        }
    });
    menu
}

fn source_slice(buffer: &gtk::TextBuffer, range: std::ops::Range<i32>) -> String {
    live_editor::plain_text(
        buffer,
        &buffer.iter_at_offset(range.start),
        &buffer.iter_at_offset(range.end),
    )
}
