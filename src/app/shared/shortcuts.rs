use crate::app::shell::state::State;
use gtk::{gdk, glib, prelude::*, ApplicationWindow, Box as GtkBox, Label, Orientation};

const HELP_MAX_HEIGHT: i32 = 680;

/// 안내 한 줄: 뜻, Markdown 입력 문법(서식만), 키.
type Row = (&'static str, &'static str, &'static [&'static str]);
type Section = (&'static str, &'static [Row]);

const NOTES: &[Section] = &[
    (
        "앱",
        &[
            ("새 메모", "", &["Ctrl+N"]),
            ("메모 목록 검색", "", &["Ctrl+F"]),
            ("현재 창 닫기", "", &["Ctrl+W"]),
            ("이 안내", "", &["F1"]),
        ],
    ),
    (
        "메모 서식",
        &[
            ("제목", "# 제목", &["Ctrl+1"]),
            ("소제목", "## 소제목", &["Ctrl+2"]),
            ("굵게", "**글자**", &["Ctrl+B"]),
            ("기울임", "*글자*", &["Ctrl+I"]),
            ("취소선", "~~글자~~", &["Ctrl+Shift+X"]),
            ("링크", "[이름](주소)", &["Ctrl+K"]),
            ("번호 목록", "1. 항목", &["Ctrl+Shift+7"]),
            ("글머리표", "- 항목", &["Ctrl+Shift+8"]),
            ("체크 목록", "- [ ] 할 일", &["Ctrl+Shift+9"]),
        ],
    ),
    (
        "메모 편집",
        &[
            ("체크 변경 · 링크 열기", "", &["Ctrl+Enter"]),
            ("원문 보기", "", &["Ctrl+Shift+M"]),
            ("실행 취소", "", &["Ctrl+Z"]),
            ("다시 실행", "", &["Ctrl+Shift+Z"]),
        ],
    ),
];

const CANVAS: &[Section] = &[
    (
        "캔버스 선택",
        &[
            ("메모 열기", "", &["두 번 누르기", "Enter"]),
            ("선택에 더하기", "", &["Shift+누르기"]),
            ("모두 선택", "", &["Ctrl+A"]),
            ("선택 해제", "", &["Esc"]),
            ("선택한 메모 삭제", "", &["Delete", "Backspace"]),
            ("완료 표시", "", &["Ctrl+Enter"]),
            ("다음 메모 연결", "", &["Ctrl+L"]),
            ("이동", "", &["방향키"]),
            ("크게 이동", "", &["Shift+방향키"]),
        ],
    ),
    (
        "캔버스 보기",
        &[
            ("전체 보기", "", &["Shift+1"]),
            ("선택 영역 보기", "", &["Shift+2"]),
            ("확대 · 축소", "", &["Ctrl++", "Ctrl+-"]),
            ("100%로 보기", "", &["Ctrl+0"]),
            ("보드 이동", "", &["Space+끌기"]),
        ],
    ),
    (
        "캔버스 정렬",
        &[
            ("왼쪽 · 가운데 · 오른쪽", "", &["Alt+A", "Alt+H", "Alt+D"]),
            ("위쪽 · 가운데 · 아래쪽", "", &["Alt+W", "Alt+V", "Alt+S"]),
            (
                "간격 같게 (가로 · 세로)",
                "",
                &["Alt+Shift+H", "Alt+Shift+V"],
            ),
            ("격자로 정리", "", &["Alt+T"]),
            ("연결 순서로 배치", "", &["Alt+F"]),
        ],
    ),
];

fn column(sections: &[Section]) -> GtkBox {
    let column = GtkBox::new(Orientation::Vertical, 18);
    column.set_hexpand(true);
    for (title, rows) in sections {
        let section = GtkBox::new(Orientation::Vertical, 6);
        let heading = Label::new(Some(title));
        heading.set_xalign(0.);
        heading.add_css_class("help-section");
        section.append(&heading);
        // 키 칸 폭을 맞춰 문법 칸이 세로로 정렬되게 한다.
        let key_column = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
        for (meaning, syntax, keys) in *rows {
            let row = GtkBox::new(Orientation::Horizontal, 10);
            let what = Label::new(Some(meaning));
            what.set_xalign(0.);
            what.set_hexpand(true);
            row.append(&what);
            if !syntax.is_empty() {
                let sample = Label::new(Some(syntax));
                sample.set_xalign(0.);
                sample.set_width_chars(13);
                sample.add_css_class("help-syntax");
                row.append(&sample);
            }
            let caps = GtkBox::new(Orientation::Horizontal, 3);
            for key in *keys {
                let cap = Label::new(Some(key));
                cap.add_css_class("help-key");
                caps.append(&cap);
            }
            caps.set_halign(gtk::Align::End);
            if !syntax.is_empty() {
                let cell = GtkBox::new(Orientation::Horizontal, 0);
                cell.append(&caps);
                cell.set_halign(gtk::Align::End);
                key_column.add_widget(&cell);
                row.append(&cell);
            } else {
                row.append(&caps);
            }
            section.append(&row);
        }
        column.append(&section);
    }
    column
}

pub(in crate::app) fn help(window: &ApplicationWindow) {
    let dialog = gtk::Window::builder()
        .transient_for(window)
        .modal(true)
        .title("단축키와 서식")
        .default_width(860)
        .resizable(false)
        .build();
    dialog.add_css_class("lsm-help");
    let bar = gtk::CenterBox::new();
    bar.add_css_class("help-bar");
    let close = gtk::Button::new();
    close.set_child(Some(&crate::presentation::icons::image(
        crate::presentation::icons::CLOSE,
    )));
    close.add_css_class("window-action");
    close.set_tooltip_text(Some("닫기 · Esc"));
    close.update_property(&[gtk::accessible::Property::Label("닫기")]);
    let weak = dialog.downgrade();
    close.connect_clicked(move |_| {
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    bar.set_start_widget(Some(&close));
    let bar_title = Label::new(Some("단축키와 서식"));
    bar_title.add_css_class("header-title");
    bar.set_center_widget(Some(&bar_title));
    let handle = gtk::WindowHandle::new();
    handle.set_child(Some(&bar));
    dialog.set_titlebar(Some(&handle));
    let body = GtkBox::new(Orientation::Horizontal, 40);
    body.set_homogeneous(true);
    body.add_css_class("help-sheet");
    body.append(&column(NOTES));
    body.append(&column(CANVAS));
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(HELP_MAX_HEIGHT)
        .child(&body)
        .build();
    dialog.set_child(Some(&scroll));
    super::close_on_escape(&dialog);
    dialog.present();
    // 읽는 창이다. 닫기 버튼에 저절로 포커스 링을 띄우지 않는다.
    gtk::prelude::GtkWindowExt::set_focus(&dialog, None::<&gtk::Widget>);
}

pub(in crate::app) fn attach(window: &ApplicationWindow, state: &State) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = window.downgrade();
    let state = std::rc::Rc::downgrade(state);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        let Some(window) = weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        if key == gdk::Key::F1 {
            help(&window);
            return glib::Propagation::Stop;
        }
        let mods = modifiers
            & (gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::SHIFT_MASK
                | gdk::ModifierType::ALT_MASK);
        if mods != gdk::ModifierType::CONTROL_MASK {
            return glib::Propagation::Proceed;
        }
        let Some(state) = state.upgrade() else {
            return glib::Propagation::Proceed;
        };
        match key.to_lower() {
            gdk::Key::n => {
                crate::app::shell::commands::new_note(&state);
            }
            gdk::Key::f => {
                let (library, search) = {
                    let s = state.borrow();
                    (
                        s.views.library.window.clone(),
                        s.views.library.search.clone(),
                    )
                };
                library.present();
                search.grab_focus();
                search.select_region(0, -1);
            }
            gdk::Key::w => window.close(),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    window.add_controller(keys);
}
