use super::*;
use crate::app::test_support::{count, drain, find_checkbox, settle, until};
use std::time::Instant;

fn text(buffer: &TextBuffer) -> String {
    plain_text(buffer, &buffer.start_iter(), &buffer.end_iter())
}

fn source_iter(buffer: &TextBuffer, offset: i32) -> gtk::TextIter {
    let mut iter = buffer.start_iter();
    let mut logical = 0;
    loop {
        if iter.child_anchor().is_some() {
            if !iter.forward_char() {
                break;
            }
            continue;
        }
        if logical == offset {
            return iter;
        }
        logical += 1;
        if !iter.forward_char() {
            break;
        }
    }
    buffer.end_iter()
}

fn tagged(buffer: &TextBuffer, offset: i32, name: &str) -> bool {
    let tag = buffer.tag_table().lookup(name).unwrap();
    source_iter(buffer, offset).has_tag(&tag)
}

fn shown_window(text: &str) -> (gtk::Window, TextView, CheckButton, Rc<LiveEditor>) {
    let view = new_view();
    view.buffer().set_text(text);
    // 실제 편집 창과 같은 순서: 스크롤 뷰에 넣은 뒤 붙인다.
    let scroll = gtk::ScrolledWindow::builder().child(&view).build();
    let (source, editor) = attach_editor(&view, None);
    let window = gtk::Window::builder()
        .default_width(420)
        .default_height(320)
        .child(&scroll)
        .build();
    window.present();
    until(|| {
        view.is_mapped() && view.width() > 0 && view.height() > 0 && !editor.refresh_pending()
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
pub(in crate::app) use layout::fmt_003_empty_nested_item_keeps_next_paragraph_x;
pub(in crate::app) use layout::fmt_003_list_layout_and_hit_testing;
pub(in crate::app) use layout::fmt_003_m1_typography_ink_regression;
pub(in crate::app) use links::fmt_003_link_preview_lifecycle;
pub(in crate::app) use typing::fmt_003_typing_flow;

pub(in crate::app) use layout::fmt_003_wrapped_list_columns;

fn marker_ink(view: &TextView, at: i32) -> Vec<u8> {
    settle(30);
    let paintable = gtk::WidgetPaintable::new(Some(view));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, view.width() as f64, view.height() as f64);
    let texture = view
        .native()
        .unwrap()
        .renderer()
        .unwrap()
        .render_texture(&snapshot.to_node().unwrap(), None);
    let stride = texture.width() as usize * 4;
    let mut pixels = vec![0; stride * texture.height() as usize];
    texture.download(&mut pixels, stride);
    let left = view.left_margin() as usize;
    let (top, height) = view.line_yrange(&view.buffer().iter_at_offset(at));
    let (_, top) = view.buffer_to_window_coords(gtk::TextWindowType::Widget, 0, top);
    (top.max(0) as usize..(top + height).min(texture.height()) as usize)
        .flat_map(|y| {
            pixels[y * stride + left * 4..y * stride + (left + (GUTTER - MARKER_GAP) as usize) * 4]
                .to_vec()
        })
        .collect()
}
