use super::{
    link_preview::{self, LinkPreview},
    list_edit,
};
use crate::presentation::markdown::{self, Document, ListKind, Style};
use gtk::{gdk, glib, pango, prelude::*, CheckButton, TextBuffer, TextTag, TextView};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap},
    ops::Range,
    rc::Rc,
};

/// 목록 한 단계가 차지하는 표식 칸. 표식은 칸 오른쪽에 붙여 본문과 늘 MARKER_GAP만큼 띄운다.
const GUTTER: i32 = 24;
const MARKER_GAP: i32 = 6;
/// CSS `.md-checkbox check` 13px + 테두리 1.5px × 2.
const CHECK_BOX: i32 = 16;
const HANG_FIX_LIMIT: i32 = 6;

struct TaskWidget {
    button: gtk::CheckButton,
    offset: Rc<Cell<i32>>,
}

/// 표식 칸의 왼쪽. 태그 변경 직후의 낡은 글자 위치에 끌려가지 않도록 여백으로 계산한다.
fn marker_x(view: &TextView, depth: usize) -> i32 {
    view.left_margin() + GUTTER * depth as i32
}

/// 표식 칸의 오른쪽 끝(본문에서 MARKER_GAP 앞).
fn marker_end(view: &TextView, depth: usize) -> i32 {
    marker_x(view, depth) + GUTTER - MARKER_GAP
}

/// (왼쪽 여백, 첫 줄 들여쓰기) 조합마다 하나씩 재사용한다. 값이 몇 가지뿐이라 늘어나지 않는다.
type ParagraphTags = HashMap<(i32, i32), TextTag>;

struct LiveEditor {
    view: glib::WeakRef<TextView>,
    document: RefCell<Document>,
    tags: EditorTags,
    widgets: MarkerWidgets,
    input: InputState,
    pointer: PointerState,
    layout: LayoutState,
    preview: Rc<LinkPreview>,
    dirty: Cell<bool>,
    restyle: Cell<bool>,
    pending: Cell<bool>,
    source: Cell<bool>,
}

impl LiveEditor {
    fn schedule(self: &Rc<Self>) {
        if self.pending.replace(true) {
            return;
        }
        let editor = self.clone();
        glib::idle_add_local_once(move || {
            editor.pending.set(false);
            editor.refresh();
        });
    }

    /// 글이 바뀌면 그리기(GDK_PRIORITY_REDRAW)보다 먼저 맞춘다. 늦으면 낡은 배치로 한 프레임 그려졌다가 바뀐다.
    fn schedule_edit(self: &Rc<Self>) {
        self.schedule();
        let editor = self.clone();
        glib::idle_add_local_full(glib::Priority::HIGH_IDLE, move || {
            editor.refresh();
            glib::ControlFlow::Break
        });
    }

    fn refresh(self: &Rc<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        if self.input.composing.get() {
            // 조합 중 확정된 글자로 빈 항목이 채워지면 표식 폭이 달라진다. 글자·커서는 건드리지 않고 들여쓰기만 맞춘다.
            if self.dirty.replace(false) {
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                *self.document.borrow_mut() = markdown::parse(&text);
                self.restyle.set(true);
            }
            let document = self.document.borrow();
            let selected = selected_lines(&buffer);
            self.sync_paragraphs(&view, &document, &selected);
            self.settle_paragraphs(&view, &document, &selected);
            return;
        }
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        if self.dirty.replace(false) {
            let text = buffer.text(&start, &end, true);
            *self.document.borrow_mut() = markdown::parse(&text);
            self.restyle.set(true);
        }
        if self.input.commit_newline.replace(false) && !self.source.get() {
            list_edit::continue_after_commit(&view, &self.document.borrow());
            if self.dirty.replace(false) {
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                *self.document.borrow_mut() = markdown::parse(&text);
                self.restyle.set(true);
            }
        }
        let document = self.document.borrow();
        if self.restyle.replace(false) {
            for (style, tag) in &self.tags.styles {
                let ranges = if self.source.get() {
                    Vec::new()
                } else {
                    document
                        .spans
                        .iter()
                        .filter(|span| span.style == *style)
                        .map(|span| span.range.clone())
                        .collect()
                };
                reconcile(&buffer, tag, ranges);
            }
            let syntax = if self.source.get() {
                Vec::new()
            } else {
                document
                    .markers
                    .iter()
                    .map(|marker| marker.range.clone())
                    .chain(document.items.iter().map(|item| item.marker.clone()))
                    .collect()
            };
            reconcile(&buffer, &self.tags.syntax, syntax);
        }
        self.leave_marker(&buffer, &document);
        let active = active_lines(&buffer);
        let hidden = if self.source.get() {
            Vec::new()
        } else {
            document
                .markers
                .iter()
                .filter(|marker| {
                    marker.owner.end <= active.start || marker.owner.start >= active.end
                })
                .map(|marker| marker.range.clone())
                .collect()
        };
        // 선택이 닿은 줄은 표식 글자를 레이아웃에서 뺀다. 안 그러면 선택 색이 표식 칸까지 번지고 글자가 비친다.
        let selected = selected_lines(&buffer);
        let hidden = hidden
            .into_iter()
            .chain(
                document
                    .items
                    .iter()
                    .filter(|item| !self.source.get() && hides_marker(item, &selected))
                    .map(|item| item.marker.clone()),
            )
            .collect();
        reconcile(&buffer, &self.tags.hidden, hidden);
        // 기호·체크는 위젯으로 그리고 표식 글자는 보이지 않게 한다. 선택 중에도 같고 복사는 원문이다.
        let markers = if self.source.get() {
            Vec::new()
        } else {
            document
                .items
                .iter()
                .filter(|item| item.kind != ListKind::Ordered)
                .map(|item| item.marker.clone())
                .collect()
        };
        reconcile(&buffer, &self.tags.marker, markers);
        self.sync_paragraphs(&view, &document, &selected);
        self.settle_paragraphs(&view, &document, &selected);
        drop(document);
        self.sync_theme(&view);
        self.sync_tasks();
        self.layout.frames.set(2);
        // 입력으로 움직인 커서는 링크 안에 있어도 카드를 띄우지 않는다.
        let edited = self.input.edited.replace(false);
        if self.input.caret_moved.replace(false) && !edited {
            self.sync_caret_preview();
        }
    }
}

mod input;
mod layout;
mod markers;
mod tags;

use input::{connect_gestures, connect_keys, connect_updates, source_toggle};
use input::{InputState, PointerState};
use layout::LayoutState;
use layout::{active_lines, hides_marker, selected_lines};
use markers::MarkerWidgets;
use tags::{create_tags, reconcile, EditorTags};

/// 본문 여백과 줄 간격. 목록 표식 칸과 원문 내어쓰기가 이 값을 기준으로 계산된다.
pub(in crate::app) fn configure(view: &TextView) {
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_left_margin(20);
    view.set_right_margin(20);
    view.set_top_margin(14);
    view.set_bottom_margin(24);
    // 줄 간격은 위쪽에 둔다. GTK는 줄 아래 간격을 누르면 숨긴 글자까지 센 위치를 계산해 중단된다.
    view.set_pixels_above_lines(4);
    view.set_pixels_inside_wrap(2);
    // 줄 높이를 고정한다. 기본 글꼴에 한글이 없으면 빈 줄이 한글 줄보다 낮아 입력할 때 아랫줄이 흔들린다.
    let css = gtk::CssProvider::new();
    css.load_from_data("textview { line-height: 1.5em; }");
    #[allow(deprecated)]
    view.style_context()
        .add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

pub(in crate::app) fn attach(view: &TextView) -> CheckButton {
    attach_editor(view).0
}

fn attach_editor(view: &TextView) -> (CheckButton, Rc<LiveEditor>) {
    let buffer = view.buffer();
    buffer.set_enable_undo(true);
    let tags = create_tags(&buffer);
    let preview = LinkPreview::new(view, tags.link_hover.clone());
    let editor = Rc::new(LiveEditor {
        view: view.downgrade(),
        document: RefCell::new(Document::default()),
        tags,
        widgets: MarkerWidgets::default(),
        input: InputState::default(),
        pointer: PointerState::default(),
        layout: LayoutState::default(),
        preview,
        dirty: Cell::new(true),
        restyle: Cell::new(true),
        pending: Cell::new(false),
        source: Cell::new(false),
    });
    editor.refresh();
    connect_updates(view, &buffer, &editor);
    let source = source_toggle(&editor);
    connect_gestures(view, &editor);
    connect_keys(view, editor.clone(), &source);
    (source, editor)
}

#[cfg(test)]
pub(in crate::app) mod tests;
