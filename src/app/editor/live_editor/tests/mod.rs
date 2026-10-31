use super::*;
use crate::app::test_support::{count, drain, find_checkbox, settle, until};
use std::time::Instant;

fn text(buffer: &TextBuffer) -> String {
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

fn tagged(buffer: &TextBuffer, offset: i32, name: &str) -> bool {
    let tag = buffer.tag_table().lookup(name).unwrap();
    buffer.iter_at_offset(offset).has_tag(&tag)
}

fn shown_window(text: &str) -> (gtk::Window, TextView, CheckButton, Rc<LiveEditor>) {
    let view = TextView::new();
    configure(&view);
    view.buffer().set_text(text);
    // 실제 편집 창과 같은 순서: 스크롤 뷰에 넣은 뒤 붙인다.
    let scroll = gtk::ScrolledWindow::builder().child(&view).build();
    let (source, editor) = attach_editor(&view);
    let window = gtk::Window::builder()
        .default_width(420)
        .default_height(320)
        .child(&scroll)
        .build();
    window.present();
    until(|| {
        view.is_mapped()
            && view.width() > 0
            && view.height() > 0
            && !editor.pending.get()
            && editor.layout.frames.get() == 0
    });
    (window, view, source, editor)
}

mod editing;
/// 숨긴 표식이 있는 줄의 모든 지점을 눌러도 GTK가 중단되지 않아야 한다.
/// 줄 아래 간격을 다시 쓰면 이 검사가 프로세스 중단으로 실패한다.
mod layout;
mod links;
mod typing;

pub(in crate::app) use editing::fmt_001_live_editor_preserves_source_selection_undo_and_preedit;
pub(in crate::app) use layout::fmt_003_list_layout_and_hit_testing;
pub(in crate::app) use links::fmt_003_link_preview_lifecycle;
pub(in crate::app) use typing::fmt_003_typing_flow;
