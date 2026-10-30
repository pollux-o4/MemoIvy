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

/// 목록 한 단계가 차지하는 표식 칸. 체크박스 16px과 본문 사이 8px이다.
const GUTTER: i32 = 24;
const CHECK_BOX: i32 = 20;
const BULLET_BOX: i32 = 16;
const BULLETS: [&str; 3] = ["•", "◦", "▪"];
const HANG_FIX_LIMIT: i32 = 6;

struct TaskWidget {
    button: gtk::CheckButton,
    offset: Rc<Cell<i32>>,
}

/// 표식 칸의 왼쪽. 태그 변경 직후의 낡은 글자 위치에 끌려가지 않도록 여백으로 계산한다.
fn marker_x(view: &TextView, depth: usize) -> i32 {
    view.left_margin() + GUTTER * depth as i32
}

/// (왼쪽 여백, 첫 줄 들여쓰기) 조합마다 하나씩 재사용한다. 값이 몇 가지뿐이라 늘어나지 않는다.
type ParagraphTags = HashMap<(i32, i32), TextTag>;

struct LiveEditor {
    view: glib::WeakRef<TextView>,
    document: RefCell<Document>,
    tags: Vec<(Style, TextTag)>,
    hidden: TextTag,
    syntax: TextTag,
    /// 표식 글자를 투명하게 한다. 줄을 접지 않아 빈 항목에도 커서가 놓이고 위치 계산이 정확하다.
    marker: TextTag,
    paragraphs: RefCell<ParagraphTags>,
    link_hover: TextTag,
    preview: Rc<LinkPreview>,
    dirty: Cell<bool>,
    restyle: Cell<bool>,
    pending: Cell<bool>,
    composing: Cell<bool>,
    source: Cell<bool>,
    task_widgets: RefCell<Vec<TaskWidget>>,
    bullet_widgets: RefCell<Vec<gtk::Label>>,
    syncing_tasks: Cell<bool>,
    layout_frames: Cell<u8>,
    /// 커서 이동과 입력을 구분해 타이핑 중에는 링크 카드를 띄우지 않는다.
    edited: Cell<bool>,
    caret_moved: Cell<bool>,
    hovered: RefCell<Option<Range<i32>>>,
    pointer: Cell<Option<(f64, f64)>>,
    /// 조합 중 Enter. 확정된 뒤 다음 항목 표식을 채운다.
    commit_newline: Cell<bool>,
    /// 원문 표식의 측정 폭과 실제 그려진 폭의 차이. 본문이 커서 줄에서 밀리지 않게 보정한다.
    hang_fix: RefCell<HashMap<String, i32>>,
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

    fn refresh(self: &Rc<Self>) {
        if self.composing.get() {
            return;
        }
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        if self.dirty.replace(false) {
            let text = buffer.text(&start, &end, true);
            *self.document.borrow_mut() = markdown::parse(&text);
            self.restyle.set(true);
        }
        if self.commit_newline.replace(false) && !self.source.get() {
            list_edit::continue_after_commit(&view, &self.document.borrow());
            if self.dirty.replace(false) {
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
                *self.document.borrow_mut() = markdown::parse(&text);
                self.restyle.set(true);
            }
        }
        let document = self.document.borrow();
        if self.restyle.replace(false) {
            for (style, tag) in &self.tags {
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
            reconcile(&buffer, &self.syntax, syntax);
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
        reconcile(&buffer, &self.hidden, hidden);
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
        reconcile(&buffer, &self.marker, markers);
        self.sync_paragraphs(&view, &document, &selected);
        drop(document);
        self.sync_theme(&view);
        self.sync_tasks();
        self.layout_frames.set(2);
        // 입력으로 움직인 커서는 링크 안에 있어도 카드를 띄우지 않는다.
        let edited = self.edited.replace(false);
        if self.caret_moved.replace(false) && !edited {
            self.sync_caret_preview();
        }
    }

    /// 표식은 한 글자처럼 다룬다. 커서가 그 안에 놓이면 본문 앞으로 옮겨 문법 앞에 글자가 들어가지 않게 한다.
    fn leave_marker(&self, buffer: &TextBuffer, document: &Document) {
        if buffer.has_selection() || self.source.get() {
            return;
        }
        let at = buffer.iter_at_mark(&buffer.get_insert()).offset();
        if let Some(item) = document.items.iter().find(|i| i.marker.contains(&at)) {
            buffer.place_cursor(&buffer.iter_at_offset(item.content.start));
        }
    }

    /// 표식 글자는 왼쪽 여백으로 내어 써 본문 시작 위치를 항상 같게 한다.
    fn sync_paragraphs(&self, view: &TextView, document: &Document, selected: &Range<i32>) {
        let buffer = view.buffer();
        let mut wanted: HashMap<(i32, i32), Vec<Range<i32>>> = HashMap::new();
        if !self.source.get() {
            let base = view.left_margin();
            let mut widths: HashMap<String, i32> = HashMap::new();
            for item in &document.items {
                let line = item.marker.start..item.content.end;
                let text_x = base + GUTTER * (item.depth as i32 + 1);
                let marker = buffer
                    .text(
                        &buffer.iter_at_offset(item.marker.start),
                        &buffer.iter_at_offset(item.marker.end),
                        true,
                    )
                    .to_string();
                // pixel_size는 올림이라 본문이 밀린다. 실제 폭에 가장 가까운 픽셀을 쓴다.
                let width = *widths.entry(marker).or_insert_with_key(|text| {
                    let units = view.create_pango_layout(Some(text)).size().0;
                    let width = (f64::from(units) / f64::from(pango::SCALE)).round() as i32;
                    width
                        + self
                            .hang_fix
                            .borrow()
                            .get(text.as_str())
                            .copied()
                            .unwrap_or(0)
                });
                // Pango의 음수 들여쓰기는 첫 줄이 여백에서 시작하고 나머지 줄이 그만큼 들어간다.
                let key = if hides_marker(item, selected) {
                    (text_x, 0)
                } else {
                    ((text_x - width).max(0), -width)
                };
                wanted.entry(key).or_default().push(line);
            }
        }
        let mut tags = self.paragraphs.borrow_mut();
        for key in wanted.keys() {
            tags.entry(*key).or_insert_with(|| {
                let tag = TextTag::builder().left_margin(key.0).indent(key.1).build();
                buffer.tag_table().add(&tag);
                tag
            });
        }
        for (key, tag) in tags.iter() {
            reconcile(&buffer, tag, wanted.remove(key).unwrap_or_default());
        }
    }

    /// 레이아웃이 안정된 뒤 커서 줄의 본문 위치를 재어, 어긋났으면 그 표식 폭을 고친다.
    fn correct_hang(self: &Rc<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        if self.source.get() {
            return;
        }
        let buffer = view.buffer();
        if buffer.has_selection() {
            return;
        }
        let active = active_lines(&buffer);
        let correction =
            {
                let document = self.document.borrow();
                let Some(item) = document.items.iter().find(|item| {
                    item.marker.start >= active.start && item.content.end < active.end
                }) else {
                    return;
                };
                let want = view.left_margin() + GUTTER * (item.depth as i32 + 1);
                let got = view
                    .iter_location(&buffer.iter_at_offset(item.content.start))
                    .x();
                let marker = buffer
                    .text(
                        &buffer.iter_at_offset(item.marker.start),
                        &buffer.iter_at_offset(item.marker.end),
                        true,
                    )
                    .to_string();
                (marker, got - want)
            };
        let (marker, delta) = correction;
        if delta == 0 {
            return;
        }
        let mut fixes = self.hang_fix.borrow_mut();
        let fix = fixes.entry(marker).or_insert(0);
        // 큰 차이는 측정 오류다. 여백 부족 등으로 맞출 수 없는 경우 흔들리지 않게 한다.
        if (*fix + delta).abs() <= HANG_FIX_LIMIT {
            *fix += delta;
            drop(fixes);
            self.schedule();
        }
    }

    /// 표시 전용 태그 색은 CSS 토큰에서 읽는다. 테마가 바뀌면 다음 갱신에 따라간다.
    #[allow(deprecated)]
    fn sync_theme(&self, view: &TextView) {
        let context = view.style_context();
        if let Some(muted) = context.lookup_color("note_muted") {
            if self.syntax.foreground_rgba().as_ref() != Some(&muted) {
                self.syntax.set_foreground_rgba(Some(&muted));
                if let Some((_, done)) = self.tags.iter().find(|(s, _)| *s == Style::TaskDone) {
                    done.set_foreground_rgba(Some(&muted));
                }
                if let Some((_, task)) = self.tags.iter().find(|(s, _)| *s == Style::Task) {
                    task.set_foreground_rgba(Some(&muted));
                }
            }
        }
        if let Some(accent) = context.lookup_color("lsm_accent") {
            let tint = gdk::RGBA::new(accent.red(), accent.green(), accent.blue(), 0.16);
            if self.link_hover.background_rgba().as_ref() != Some(&tint) {
                self.link_hover.set_background_rgba(Some(&tint));
            }
        }
    }

    fn sync_tasks(self: &Rc<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        let document = self.document.borrow();
        self.sync_bullets(&view, &document);
        let mut widgets = self.task_widgets.borrow_mut();
        // shortcut: GTK overlay에는 공개 제거 API가 없어 창 안에서 최대 개수만큼 재사용, GTK가 제거 API를 제공하면 회수 정책 재검토.
        for widget in widgets.iter().skip(document.tasks.len()) {
            widget.button.set_visible(false);
        }
        while widgets.len() < document.tasks.len() {
            let button = gtk::CheckButton::new();
            button.add_css_class("md-checkbox");
            button.set_focusable(false);
            button.set_tooltip_text(Some("완료 상태 변경 · Ctrl+Enter"));
            let offset = Rc::new(Cell::new(0));
            let position = offset.clone();
            let weak = Rc::downgrade(self);
            button.connect_toggled(move |_| {
                if let Some(editor) = weak.upgrade() {
                    if !editor.syncing_tasks.get() {
                        editor.toggle_task(position.get());
                    }
                }
            });
            view.add_overlay(&button, 0, 0);
            widgets.push(TaskWidget { button, offset });
        }
        self.syncing_tasks.set(true);
        for (widget, task) in widgets.iter().zip(&document.tasks) {
            widget.offset.set(task.range.start);
            widget.button.set_active(task.checked);
            widget.button.set_sensitive(view.is_editable());
            let iter = buffer.iter_at_offset(task.range.start);
            let mut end = iter;
            end.forward_to_line_end();
            let label = buffer.text(&buffer.iter_at_offset(task.range.end), &end, true);
            widget
                .button
                .update_property(&[gtk::accessible::Property::Label(label.trim())]);
            // 인용문처럼 원문으로 두는 항목에는 위젯을 올리지 않는다.
            let index = document
                .items
                .partition_point(|item| item.marker.end < task.range.end);
            let item = document.items.get(index).filter(|item| {
                item.marker.start <= task.range.start && task.range.end <= item.marker.end
            });
            let visible = !self.source.get() && item.is_some();
            widget.button.set_visible(visible);
            if let (true, Some(item)) = (visible, item) {
                let rect = view.iter_location(&buffer.iter_at_offset(item.content.start));
                widget.button.set_size_request(CHECK_BOX, rect.height());
                view.move_overlay(&widget.button, marker_x(&view, item.depth), rect.y());
            }
        }
        self.syncing_tasks.set(false);
    }

    /// 글머리 기호는 원문을 바꾸지 않고 표식 칸에 그린다. 입력·클릭은 본문으로 통과한다.
    fn sync_bullets(&self, view: &TextView, document: &Document) {
        let buffer = view.buffer();
        let bullets: Vec<_> = document
            .items
            .iter()
            .filter(|item| item.kind == ListKind::Bullet && !self.source.get())
            .collect();
        let mut widgets = self.bullet_widgets.borrow_mut();
        for widget in widgets.iter().skip(bullets.len()) {
            widget.set_visible(false);
        }
        while widgets.len() < bullets.len() {
            let label = gtk::Label::new(None);
            label.add_css_class("md-bullet");
            label.set_can_target(false);
            view.add_overlay(&label, 0, 0);
            widgets.push(label);
        }
        for (label, item) in widgets.iter().zip(&bullets) {
            label.set_text(BULLETS[item.depth.min(BULLETS.len() - 1)]);
            let rect = view.iter_location(&buffer.iter_at_offset(item.content.start));
            label.set_size_request(BULLET_BOX, rect.height());
            label.set_visible(true);
            view.move_overlay(label, marker_x(view, item.depth), rect.y());
        }
    }

    fn link_at(&self, offset: i32) -> Option<markdown::Link> {
        self.document
            .borrow()
            .links
            .iter()
            .find(|link| link.owner.contains(&offset))
            .cloned()
    }

    /// 포인터 또는 커서를 따라 링크 카드 예약을 갱신한다.
    fn sync_caret_preview(self: &Rc<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let buffer = view.buffer();
        if self.composing.get() || buffer.has_selection() || !view.is_focus() {
            self.preview.caret(None, gdk::Rectangle::new(0, 0, 0, 0));
            return;
        }
        let offset = buffer.iter_at_mark(&buffer.get_insert()).offset();
        let link = self.link_at(offset);
        self.preview
            .caret(link.as_ref(), LinkPreview::anchor(&view, offset, None));
    }

    fn toggle_task(self: &Rc<Self>, offset: i32) -> bool {
        if self.composing.get() {
            return false;
        }
        let Some(view) = self.view.upgrade() else {
            return false;
        };
        if !view.is_editable() {
            return false;
        }
        self.refresh();
        let document = self.document.borrow();
        let Some(task) = document
            .tasks
            .iter()
            .find(|task| task.range.contains(&offset))
        else {
            return false;
        };
        let buffer = view.buffer();
        let mut start = buffer.iter_at_offset(task.range.start + 1);
        let mut end = buffer.iter_at_offset(task.range.start + 2);
        buffer.begin_user_action();
        buffer.delete(&mut start, &mut end);
        buffer.insert(&mut start, if task.checked { " " } else { "x" });
        buffer.end_user_action();
        true
    }
}

// GTK already moves tag boundaries with edits; change only ranges whose decoration differs.
fn reconcile(buffer: &TextBuffer, tag: &TextTag, mut desired: Vec<Range<i32>>) {
    desired.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<i32>> = Vec::new();
    for range in desired {
        if let Some(last) = merged.last_mut().filter(|last| range.start <= last.end) {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    let desired: BTreeSet<_> = merged
        .into_iter()
        .map(|range| (range.start, range.end))
        .collect();
    let mut current = BTreeSet::new();
    let mut iter = buffer.start_iter();
    loop {
        if !iter.has_tag(tag) && !iter.forward_to_tag_toggle(Some(tag)) {
            break;
        }
        let start = iter.offset();
        iter.forward_to_tag_toggle(Some(tag));
        if start < iter.offset() {
            current.insert((start, iter.offset()));
        }
        if iter.is_end() {
            break;
        }
    }
    for &(start, end) in current.difference(&desired) {
        buffer.remove_tag(
            tag,
            &buffer.iter_at_offset(start),
            &buffer.iter_at_offset(end),
        );
    }
    for &(start, end) in desired.difference(&current) {
        buffer.apply_tag(
            tag,
            &buffer.iter_at_offset(start),
            &buffer.iter_at_offset(end),
        );
    }
}

fn copy_source(view: &TextView, cut: bool) {
    if cut && !view.is_editable() {
        return;
    }
    let buffer = view.buffer();
    if let Some((start, end)) = buffer.selection_bounds() {
        view.clipboard().set_text(&buffer.text(&start, &end, true));
        if cut {
            buffer.begin_user_action();
            buffer.delete_selection(true, true);
            buffer.end_user_action();
        }
    }
}

/// 선택이 닿은 줄 범위. 선택이 없으면 비어 있다.
fn selected_lines(buffer: &TextBuffer) -> Range<i32> {
    if buffer.has_selection() {
        active_lines(buffer)
    } else {
        0..0
    }
}

/// 선택 중 표식 글자를 뺄 항목. 번호는 그 글자가 곧 표식이고 빈 항목은 줄이 접혀 뺄 수 없다.
fn hides_marker(item: &markdown::ListItem, selected: &Range<i32>) -> bool {
    item.kind != ListKind::Ordered
        && item.content.start < item.content.end
        && selected.start < selected.end
        && item.marker.start < selected.end
        && item.content.end >= selected.start
}

fn active_lines(buffer: &TextBuffer) -> Range<i32> {
    let (mut start, mut end) = buffer.selection_bounds().unwrap_or_else(|| {
        let cursor = buffer.iter_at_mark(&buffer.get_insert());
        (cursor, cursor)
    });
    start.set_line_offset(0);
    let has_next = end.forward_line();
    // Include an empty last line and the caret at EOF, without revealing the next line.
    start.offset()..(end.offset() + i32::from(!has_next))
}

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
}

pub(in crate::app) fn attach(view: &TextView) -> CheckButton {
    attach_editor(view).0
}

fn attach_editor(view: &TextView) -> (CheckButton, Rc<LiveEditor>) {
    let buffer = view.buffer();
    buffer.set_enable_undo(true);
    let (tags, hidden, syntax, marker, link_hover) = create_tags(&buffer);
    let preview = LinkPreview::new(view, link_hover.clone());
    let editor = Rc::new(LiveEditor {
        view: view.downgrade(),
        document: RefCell::new(Document::default()),
        tags,
        hidden,
        syntax,
        marker,
        paragraphs: RefCell::new(HashMap::new()),
        link_hover,
        preview,
        dirty: Cell::new(true),
        restyle: Cell::new(true),
        pending: Cell::new(false),
        composing: Cell::new(false),
        source: Cell::new(false),
        task_widgets: RefCell::new(Vec::new()),
        bullet_widgets: RefCell::new(Vec::new()),
        syncing_tasks: Cell::new(false),
        layout_frames: Cell::new(0),
        edited: Cell::new(false),
        caret_moved: Cell::new(false),
        hovered: RefCell::new(None),
        pointer: Cell::new(None),
        commit_newline: Cell::new(false),
        hang_fix: RefCell::new(HashMap::new()),
    });
    editor.refresh();
    connect_updates(view, &buffer, &editor);
    let source = source_toggle(&editor);
    connect_gestures(view, &editor);
    connect_keys(view, editor.clone(), &source);
    (source, editor)
}

#[cfg(test)]
pub(in crate::app) mod tests {
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
        until(|| view.is_mapped() && view.width() > 0 && view.height() > 0);
        (window, view, source, editor)
    }

    /// 숨긴 표식이 있는 줄의 모든 지점을 눌러도 GTK가 중단되지 않아야 한다.
    /// 줄 아래 간격을 다시 쓰면 이 검사가 프로세스 중단으로 실패한다.
    pub(in crate::app) fn fmt_003_list_layout_and_hit_testing() {
        let text =
            "제목\n- 우유\n  - 저지방\n- [ ] 할 일\n- [x] 끝남\n1. 번호\n- \n**굵게** 끝\n마지막 *줄*";
        let (window, view, source, _editor) = shown_window(text);
        let buffer = view.buffer();
        buffer.place_cursor(&buffer.iter_at_offset(0));
        settle(60);
        for y in 0..view.height() {
            for x in (0..view.width()).step_by(13) {
                let (bx, by) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, x, y);
                let _ = view.iter_at_position(bx, by);
                let _ = view.iter_at_location(bx, by);
            }
        }
        let offset = |needle: &str| text[..text.find(needle).unwrap()].chars().count() as i32;
        // 모든 항목은 표식을 왼쪽 여백으로 내어 쓰고 본문은 표식 칸 뒤에서 시작한다. 커서 줄도 같다.
        let hanging = |at: i32| {
            buffer
                .iter_at_offset(at)
                .tags()
                .into_iter()
                .find(|tag| tag.is_indent_set() && tag.indent() != 0)
                .map(|tag| tag.left_margin() - tag.indent())
        };
        for caret in [0, offset("우유")] {
            buffer.place_cursor(&buffer.iter_at_offset(caret));
            settle(40);
            assert_eq!(hanging(offset("우유")), Some(20 + GUTTER));
            assert_eq!(hanging(offset("저지방")), Some(20 + GUTTER * 2));
            assert_eq!(hanging(offset("할 일")), Some(20 + GUTTER));
            assert_eq!(hanging(offset("번호")), Some(20 + GUTTER));
            assert!(
                tagged(&buffer, offset("- 우유"), "md-marker"),
                "caret line stays rendered"
            );
        }
        assert!(tagged(&buffer, offset("끝남"), "md-task-done"));
        assert!(!tagged(&buffer, offset("할 일"), "md-task-done"));
        // 번호는 글자 그대로 쓰고, 빈 항목도 줄이 접히지 않은 채 표식이 그려진다.
        assert!(!tagged(&buffer, offset("1. "), "md-marker"));
        assert!(tagged(&buffer, offset("1. "), "md-syntax"));
        assert!(tagged(&buffer, offset("- \n"), "md-marker"));
        assert!(!tagged(&buffer, offset("- \n"), "md-hidden"));
        // 선택해도 표식을 유지한다. 복사는 원문이다.
        buffer.select_range(
            &buffer.iter_at_offset(offset("우유")),
            &buffer.iter_at_offset(offset("우유") + 1),
        );
        settle(30);
        assert!(tagged(&buffer, offset("- 우유"), "md-marker"));
        // 표식 안의 커서는 본문 앞으로 옮긴다.
        buffer.place_cursor(&buffer.iter_at_offset(offset("- [ ] 할 일")));
        settle(30);
        assert_eq!(
            buffer.iter_at_mark(&buffer.get_insert()).offset(),
            offset("할 일")
        );
        // 긴 목록을 스크롤해도 화면 안 항목의 표식이 남아 있다.
        let long = "- [ ] 항목\n- 기호\n".repeat(200);
        buffer.set_text(&long);
        buffer.place_cursor(&buffer.start_iter());
        settle(120);
        let adjustment = view.vadjustment().unwrap();
        adjustment.set_value(adjustment.upper() / 2.);
        settle(120);
        assert_eq!(count(view.upcast_ref(), "md-checkbox"), 200);
        assert_eq!(count(view.upcast_ref(), "md-bullet"), 200);
        buffer.set_text(text);
        settle(30);
        // 원문 보기에는 어떤 표시 태그도 남지 않는다.
        source.set_active(true);
        settle(20);
        assert!(!tagged(&buffer, offset("끝남"), "md-task-done"));
        assert!(buffer
            .iter_at_offset(offset("끝남"))
            .tags()
            .iter()
            .all(|t| !t.is_left_margin_set()));
        window.destroy();
        drain();
    }

    /// 쓰는 흐름: 표식 입력 즉시 렌더, Enter 이어 쓰기, 빈 항목 종료, Backspace, Tab, 번호 밀기, undo.
    pub(in crate::app) fn fmt_003_typing_flow() {
        let (window, view, _source, editor) = shown_window("");
        let buffer = view.buffer();
        let controllers = view.observe_controllers();
        let keys = (0..controllers.n_items())
            .find_map(|i| {
                controllers
                    .item(i)
                    .and_downcast::<gtk::EventControllerKey>()
            })
            .unwrap();
        let press = |key: gdk::Key, modifiers: gdk::ModifierType| -> bool {
            let stopped = keys.emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers]);
            drain();
            stopped
        };
        let none = gdk::ModifierType::empty();
        let type_text = |value: &str| {
            buffer.begin_user_action();
            buffer.insert_at_cursor(value);
            buffer.end_user_action();
            drain();
        };
        let caret = || buffer.iter_at_mark(&buffer.get_insert()).offset();
        // `- ` 두 글자로 바로 기호가 되고 본문은 그 뒤에 이어진다.
        type_text("메모\n- ");
        assert_eq!(editor.document.borrow().items.len(), 1);
        assert!(tagged(&buffer, 3, "md-marker"));
        type_text("사과");
        assert!(press(gdk::Key::Return, none));
        assert_eq!(text(&buffer), "메모\n- 사과\n- ");
        type_text("우유");
        assert!(press(gdk::Key::Tab, none));
        assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유");
        assert!(press(gdk::Key::Return, none));
        assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유\n  - ");
        // 빈 항목: 한 단계 내어 쓰고, 다시 누르면 목록이 끝난다.
        assert!(press(gdk::Key::Return, none));
        assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유\n- ");
        assert!(press(gdk::Key::Return, none));
        assert_eq!(text(&buffer), "메모\n- 사과\n  - 우유\n");
        // 체크: 완료 항목 뒤 새 항목은 비어 있다. 한 번의 undo로 되돌린다.
        buffer.set_text("- [x] 끝남");
        buffer.place_cursor(&buffer.end_iter());
        drain();
        assert!(press(gdk::Key::Return, none));
        assert_eq!(text(&buffer), "- [x] 끝남\n- [ ] ");
        assert_eq!(editor.document.borrow().tasks.len(), 2);
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), "- [x] 끝남");
        // 번호: 사이에 끼우면 뒤 번호가 밀리고 undo 한 번으로 복구된다.
        buffer.set_text("1. 가\n2. 나\n  - 하위\n3. 다\n\n1. 별개");
        buffer.place_cursor(&buffer.iter_at_offset(4));
        drain();
        assert!(press(gdk::Key::Return, none));
        assert_eq!(
            text(&buffer),
            "1. 가\n2. \n3. 나\n  - 하위\n4. 다\n\n1. 별개"
        );
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), "1. 가\n2. 나\n  - 하위\n3. 다\n\n1. 별개");
        // Backspace: 본문 맨 앞에서 표식을 지운다. 중간에서는 평소대로 지운다.
        buffer.set_text("앞\n- [ ] 본문");
        buffer.place_cursor(&buffer.iter_at_offset(8));
        drain();
        assert_eq!(caret(), 8);
        assert!(press(gdk::Key::BackSpace, none));
        assert_eq!(text(&buffer), "앞\n본문");
        buffer.undo();
        drain();
        buffer.place_cursor(&buffer.iter_at_offset(9));
        drain();
        assert!(!press(gdk::Key::BackSpace, none));
        // 왼쪽 화살표는 표식을 건너 앞 줄 끝으로 간다.
        buffer.place_cursor(&buffer.iter_at_offset(8));
        drain();
        assert!(press(gdk::Key::Left, none));
        drain();
        assert_eq!(caret(), 1);
        // 앞 줄 끝의 Delete는 표식까지 지워 본문만 잇는다.
        assert!(press(gdk::Key::Delete, none));
        assert_eq!(text(&buffer), "앞본문");
        // 항목이 아닌 줄의 Enter·Tab은 평소대로다.
        buffer.set_text("평문");
        buffer.place_cursor(&buffer.end_iter());
        drain();
        assert!(!press(gdk::Key::Return, none));
        assert!(!press(gdk::Key::Tab, none));
        // 조합 중 Enter는 입력기에 맡기고 확정 뒤 표식을 채운다.
        buffer.set_text("- 항목");
        buffer.place_cursor(&buffer.end_iter());
        drain();
        view.emit_preedit_changed("한");
        assert!(!press(gdk::Key::Return, none));
        buffer.insert_at_cursor("\n");
        view.emit_preedit_changed("");
        drain();
        assert_eq!(text(&buffer), "- 항목\n- ");
        window.destroy();
        drain();
    }

    pub(in crate::app) fn fmt_003_link_preview_lifecycle() {
        let text = "[이름](https://a.example/b) 그리고 https://c.example\n\n둘째 줄";
        let (window, view, _source, editor) = shown_window(text);
        let buffer = view.buffer();
        buffer.place_cursor(&buffer.end_iter());
        settle(60);
        let link = |i: usize| editor.document.borrow().links[i].clone();
        let anchor = gdk::Rectangle::new(10, 10, 1, 18);
        // 지나가는 포인터에는 뜨지 않는다.
        editor.preview.hover(Some(&link(0)), anchor);
        assert!(editor.preview.is_pending());
        editor.preview.hover(None, anchor);
        assert!(!editor.preview.is_pending());
        settle(450);
        assert_eq!(editor.preview.shown_url(), None);
        // 머무르면 뜨고, 이름과 목적지를 함께 보여 준다. 포커스는 편집기에 남는다.
        view.grab_focus();
        until(&|| {
            // 창 관리자 없는 환경에서는 포커스가 흔들려 예약이 취소될 수 있다.
            if !editor.preview.is_shown() && !editor.preview.is_pending() {
                editor.preview.hover(Some(&link(0)), anchor);
            }
            editor.preview.is_shown()
        });
        assert_eq!(
            editor.preview.shown_url().as_deref(),
            Some("https://a.example/b")
        );
        assert!(editor.preview.card_is_visible());
        assert_eq!(editor.preview.title(), "이름");
        assert!(view.is_focus(), "preview must not steal typing focus");
        // 옆 링크로 옮기면 기다리지 않고 내용을 바꾼다.
        editor.preview.hover(Some(&link(1)), anchor);
        until(&|| editor.preview.shown_url().as_deref() == Some("https://c.example"));
        assert_eq!(editor.preview.title(), "c.example");
        // 복사는 카드를 닫지 않고 원본 주소를 넣는다.
        editor.preview.click_copy();
        let copied = glib::MainContext::default()
            .block_on(view.clipboard().read_text_future())
            .unwrap()
            .unwrap();
        assert_eq!(copied.as_str(), "https://c.example");
        assert!(editor.preview.is_shown());
        // 벗어나면 잠시 뒤 닫힌다.
        editor.preview.hover(None, anchor);
        assert!(editor.preview.is_shown(), "grace period to reach the card");
        until(&|| !editor.preview.is_shown());
        until(|| !editor.preview.card_is_visible());
        // Esc는 포인터를 옮기지 않고 닫는다.
        until(&|| {
            // 창 관리자 없는 환경에서는 포커스가 흔들려 예약이 취소될 수 있다.
            if !editor.preview.is_shown() && !editor.preview.is_pending() {
                editor.preview.hover(Some(&link(0)), anchor);
            }
            editor.preview.is_shown()
        });
        let controllers = view.observe_controllers();
        let key = (0..controllers.n_items())
            .find_map(|i| {
                controllers
                    .item(i)
                    .and_downcast::<gtk::EventControllerKey>()
            })
            .unwrap();
        let stopped: bool = key.emit_by_name(
            "key-pressed",
            &[&gdk::Key::Escape, &0u32, &gdk::ModifierType::empty()],
        );
        assert!(stopped);
        assert!(!editor.preview.is_shown());
        settle(150);
        // 주소 수정은 원문의 주소만 선택한다.
        until(&|| {
            // 창 관리자 없는 환경에서는 포커스가 흔들려 예약이 취소될 수 있다.
            if !editor.preview.is_shown() && !editor.preview.is_pending() {
                editor.preview.hover(Some(&link(0)), anchor);
            }
            editor.preview.is_shown()
        });
        editor.preview.click_edit();
        settle(30);
        let (start, end) = buffer.selection_bounds().unwrap();
        assert_eq!(buffer.text(&start, &end, true), "https://a.example/b");
        assert!(!editor.preview.is_shown());
        // 입력하면 예약까지 취소되고 표시 태그는 undo에 들어가지 않는다.
        buffer.place_cursor(&buffer.end_iter());
        until(&|| {
            // 창 관리자 없는 환경에서는 포커스가 흔들려 예약이 취소될 수 있다.
            if !editor.preview.is_shown() && !editor.preview.is_pending() {
                editor.preview.hover(Some(&link(0)), anchor);
            }
            editor.preview.is_shown()
        });
        assert!(!buffer.can_undo());
        buffer.insert_at_cursor("a");
        assert!(!editor.preview.is_shown());
        buffer.undo();
        settle(30);
        // 커서가 링크에 머무르면 키보드로도 뜨고, 벗어나면 닫힌다.
        view.grab_focus();
        buffer.place_cursor(&buffer.iter_at_offset(2));
        until(&|| {
            if !editor.preview.is_shown() && !editor.preview.is_pending() {
                editor.sync_caret_preview();
            }
            editor.preview.is_shown()
        });
        assert_eq!(
            editor.preview.shown_url().as_deref(),
            Some("https://a.example/b")
        );
        buffer.place_cursor(&buffer.end_iter());
        until(&|| !editor.preview.is_shown());
        // Esc로 닫으면 같은 링크 안에서 움직여도 다시 뜨지 않고, 벗어났다 오면 뜬다.
        buffer.place_cursor(&buffer.iter_at_offset(2));
        until(&|| {
            if !editor.preview.is_shown() && !editor.preview.is_pending() {
                editor.sync_caret_preview();
            }
            editor.preview.is_shown()
        });
        editor.preview.dismiss();
        buffer.place_cursor(&buffer.iter_at_offset(3));
        settle(750);
        assert!(!editor.preview.is_shown(), "dismissed stays dismissed");
        buffer.place_cursor(&buffer.end_iter());
        drain();
        // 조합 중에는 뜨지 않는다.
        buffer.place_cursor(&buffer.iter_at_offset(2));
        view.emit_preedit_changed("한");
        settle(750);
        assert!(!editor.preview.is_shown());
        view.emit_preedit_changed("");
        // 대기 중 창을 닫아도 타이머·보조 표면이 남지 않는다.
        editor.preview.hover(Some(&link(0)), anchor);
        window.destroy();
        settle(450);
    }

    pub(in crate::app) fn fmt_001_live_editor_preserves_source_selection_undo_and_preedit() {
        let view = TextView::new();
        let buffer = view.buffer();
        let original =
            "# 제목😀\n**굵게** *기울임* ~~취소~~\n- [ ] 할 일\n[링크](https://example.com)\n\n";
        buffer.set_text(original);
        let source = attach(&view);
        buffer.place_cursor(&buffer.end_iter());
        drain();
        assert!(
            tagged(&buffer, 0, "md-hidden"),
            "inactive heading syntax must hide"
        );
        let bold = original[..original.find("**").unwrap()].chars().count() as i32;
        assert!(tagged(&buffer, bold, "md-hidden"));
        buffer.place_cursor(&buffer.iter_at_offset(bold + 3));
        drain();
        assert!(
            !tagged(&buffer, bold, "md-hidden"),
            "caret line reveals syntax"
        );
        assert!(
            tagged(&buffer, 0, "md-hidden"),
            "do not reveal neighboring line"
        );
        assert_eq!(text(&buffer), original);
        assert!(
            !buffer.can_undo(),
            "display updates must not enter undo history"
        );
        let checkbox = find_checkbox(view.upcast_ref()).unwrap();
        buffer.place_cursor(&buffer.end_iter());
        drain();
        assert!(
            checkbox.is_visible(),
            "inactive tasks have native checkboxes"
        );
        checkbox.set_active(true);
        drain();
        assert_eq!(text(&buffer), original.replace("[ ]", "[x]"));
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), original);

        buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
        // Clipboard actions can arrive before the queued visibility update.
        view.emit_by_name::<()>("copy-clipboard", &[]);
        let copied = glib::MainContext::default()
            .block_on(view.clipboard().read_text_future())
            .unwrap()
            .unwrap();
        assert_eq!(copied.as_str(), original);
        view.emit_by_name::<()>("cut-clipboard", &[]);
        drain();
        assert_eq!(text(&buffer), "");
        assert!(
            !checkbox.is_visible(),
            "removed tasks leave no stale overlays"
        );
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), original, "cut is one undo action");
        buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
        drain();
        assert!(
            !tagged(&buffer, 0, "md-hidden"),
            "selection must include raw syntax"
        );
        let selection = buffer.selection_bounds().unwrap();
        source.set_active(true);
        drain();
        assert_eq!(
            buffer.selection_bounds().unwrap().0.offset(),
            selection.0.offset()
        );
        assert_eq!(
            buffer.selection_bounds().unwrap().1.offset(),
            selection.1.offset()
        );
        assert!(!tagged(&buffer, 0, "md-h1"));
        source.set_active(false);
        buffer.place_cursor(&buffer.end_iter());
        drain();
        view.emit_preedit_changed("한");
        buffer.place_cursor(&buffer.start_iter());
        drain();
        assert!(
            tagged(&buffer, 0, "md-hidden"),
            "composition freezes layout"
        );
        view.emit_preedit_changed("");
        drain();
        assert!(!tagged(&buffer, 0, "md-hidden"));
        buffer.place_cursor(&buffer.end_iter());
        buffer.begin_user_action();
        buffer.insert_at_cursor("입력📝");
        buffer.end_user_action();
        drain();
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), original);
        buffer.redo();
        drain();
        assert_eq!(text(&buffer), format!("{original}입력📝"));
        buffer.undo();
        drain();

        // Run the same task mutation used by the keyboard/click handlers through a captured key signal.
        let task = original[..original.find("[ ]").unwrap()].chars().count() as i32;
        buffer.place_cursor(&buffer.iter_at_offset(task));
        drain();
        let controllers = view.observe_controllers();
        let key = (0..controllers.n_items())
            .find_map(|i| {
                controllers
                    .item(i)
                    .and_downcast::<gtk::EventControllerKey>()
            })
            .unwrap();
        let stopped: bool = key.emit_by_name(
            "key-pressed",
            &[&gdk::Key::Return, &0u32, &gdk::ModifierType::CONTROL_MASK],
        );
        assert!(stopped);
        drain();
        assert_eq!(text(&buffer), original.replace("[ ]", "[x]"));
        buffer.undo();
        drain();
        assert_eq!(
            text(&buffer),
            original,
            "one undo restores a checkbox toggle"
        );
        view.set_editable(false);
        drain();
        assert!(!checkbox.is_sensitive());
        let stopped: bool = key.emit_by_name(
            "key-pressed",
            &[&gdk::Key::Return, &0u32, &gdk::ModifierType::CONTROL_MASK],
        );
        assert!(!stopped);
        assert_eq!(text(&buffer), original);
        view.set_editable(true);

        buffer.set_text("한글😀");
        buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
        assert!(super::super::formatting::handle(
            &view,
            gdk::Key::b,
            gdk::ModifierType::CONTROL_MASK
        ));
        drain();
        assert_eq!(text(&buffer), "**한글😀**");
        assert!(super::super::formatting::handle(
            &view,
            gdk::Key::b,
            gdk::ModifierType::CONTROL_MASK
        ));
        drain();
        assert_eq!(
            text(&buffer),
            "한글😀",
            "repeat shortcut removes formatting"
        );
        buffer.undo();
        drain();
        assert_eq!(text(&buffer), "**한글😀**");

        // PERF-001-sized real widget path; report the bounded synthetic input timings.
        buffer.set_text(&"**한글😀** [링크](https://example.com)\n".repeat(230));
        drain();
        let mut samples = Vec::new();
        for _ in 0..100 {
            let started = Instant::now();
            buffer.insert_at_cursor("a");
            drain();
            samples.push(started.elapsed());
        }
        samples.sort();
        println!(
            "FMT-001 widget input+idle p95={:.3}ms",
            samples[94].as_secs_f64() * 1000.
        );
        assert!(samples[94] < std::time::Duration::from_millis(50));
    }
}

fn create_tags(buffer: &TextBuffer) -> (Vec<(Style, TextTag)>, TextTag, TextTag, TextTag, TextTag) {
    let tags = vec![
        (
            Style::Heading1,
            TextTag::builder()
                .name("md-h1")
                .scale(1.6)
                .weight(700)
                .build(),
        ),
        (
            Style::Heading2,
            TextTag::builder()
                .name("md-h2")
                .scale(1.3)
                .weight(700)
                .build(),
        ),
        (
            Style::Bold,
            TextTag::builder().name("md-bold").weight(700).build(),
        ),
        (
            Style::Italic,
            TextTag::builder()
                .name("md-italic")
                .style(pango::Style::Italic)
                .build(),
        ),
        (
            Style::Strike,
            TextTag::builder()
                .name("md-strike")
                .strikethrough(true)
                .build(),
        ),
        (
            Style::Link,
            TextTag::builder()
                .name("md-link")
                .underline(pango::Underline::Single)
                .build(),
        ),
        (Style::Task, TextTag::builder().name("md-task").build()),
        (
            Style::TaskDone,
            TextTag::builder()
                .name("md-task-done")
                .strikethrough(true)
                .build(),
        ),
    ];
    let hidden = TextTag::builder().name("md-hidden").invisible(true).build();
    let syntax = TextTag::builder().name("md-syntax").build();
    let marker = TextTag::builder()
        .name("md-marker")
        // Pango는 알파 0을 '지정 안 함'으로 읽어 불투명하게 그린다. 보이지 않을 만큼만 남긴다.
        .foreground_rgba(&gdk::RGBA::new(0., 0., 0., 0.002))
        .build();
    let link_hover = TextTag::builder().name("md-link-hover").build();
    for (_, tag) in &tags {
        buffer.tag_table().add(tag);
    }
    buffer.tag_table().add(&hidden);
    buffer.tag_table().add(&syntax);
    buffer.tag_table().add(&marker);
    buffer.tag_table().add(&link_hover);
    (tags, hidden, syntax, marker, link_hover)
}

fn connect_updates(view: &TextView, buffer: &TextBuffer, editor: &Rc<LiveEditor>) {
    let state = editor.clone();
    view.connect_map(move |_| state.schedule());
    let state = editor.clone();
    view.connect_editable_notify(move |_| state.schedule());
    let weak = Rc::downgrade(editor);
    let width = Cell::new(0);
    view.add_tick_callback(move |view, _| {
        if let Some(editor) = weak.upgrade() {
            if view.width() != width.get() {
                width.set(view.width());
                editor.preview.hide();
                editor.schedule();
            }
            // Text layout validates after tag changes; settle overlays on the following frames.
            if !editor.composing.get() && editor.layout_frames.get() > 0 {
                editor.sync_tasks();
                editor.layout_frames.set(editor.layout_frames.get() - 1);
                if editor.layout_frames.get() == 0 {
                    editor.correct_hang();
                }
            }
        }
        glib::ControlFlow::Continue
    });
    let state = editor.clone();
    buffer.connect_changed(move |_| {
        state.dirty.set(true);
        state.edited.set(true);
        // 입력이 시작되면 카드는 바로 물러나고, 포인터가 다시 움직일 때까지 뜨지 않는다.
        state.preview.hide();
        state.hovered.replace(None);
        state.schedule();
    });
    let state = editor.clone();
    buffer.connect_mark_set(move |_, _, mark| {
        if mark
            .name()
            .is_some_and(|name| name == "insert" || name == "selection_bound")
        {
            state.caret_moved.set(true);
            state.schedule();
        }
    });
    let state = editor.clone();
    view.connect_preedit_changed(move |_, preedit| {
        state.composing.set(!preedit.is_empty());
        if preedit.is_empty() {
            state.schedule();
        } else {
            state.preview.hide();
        }
    });
    // 이미 스크롤 뷰 안일 수도, 나중에 들어갈 수도 있어 현재 조절기와 이후 교체를 모두 잇는다.
    let weak = Rc::downgrade(editor);
    let watch = move |view: &TextView| {
        let Some(adjustment) = view.vadjustment() else {
            return;
        };
        let scrolled = weak.clone();
        adjustment.connect_value_changed(move |_| {
            if let Some(editor) = scrolled.upgrade() {
                editor.preview.hide();
            }
        });
        // 긴 문서는 뒤늦게 줄 높이가 확정되므로 표식 위치도 다시 맞춘다.
        let measured = weak.clone();
        adjustment.connect_changed(move |_| {
            if let Some(editor) = measured.upgrade() {
                editor.layout_frames.set(2);
            }
        });
    };
    watch(view);
    view.connect_vadjustment_notify(watch);
    if let Some(settings) = gtk::Settings::default() {
        for property in crate::presentation::theme::WATCHED {
            if settings.find_property(property).is_some() {
                let weak = Rc::downgrade(editor);
                settings.connect_notify_local(Some(property), move |_, _| {
                    if let Some(editor) = weak.upgrade() {
                        editor.schedule();
                    }
                });
            }
        }
    }
    let weak = Rc::downgrade(editor);
    view.connect_has_focus_notify(move |view| {
        if !view.has_focus() {
            if let Some(editor) = weak.upgrade() {
                // 다른 앱으로 넘어가면 떠 있는 표면을 남기지 않는다.
                editor.preview.hide();
            }
        }
    });
}

fn source_toggle(editor: &Rc<LiveEditor>) -> CheckButton {
    let source = CheckButton::with_label("Markdown 원문 보기");
    source.add_css_class("lsm-action");
    source.set_tooltip_text(Some("Markdown 원문 표시 · 기본은 실시간 서식 편집"));
    let state = editor.clone();
    source.connect_toggled(move |button| {
        state.source.set(button.is_active());
        state.restyle.set(true);
        state.preview.hide();
        state.refresh();
        if let Some(view) = state.view.upgrade() {
            view.grab_focus();
        }
    });
    source
}

fn connect_gestures(view: &TextView, editor: &Rc<LiveEditor>) {
    let click = gtk::GestureClick::new();
    click.set_button(1);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let state = editor.clone();
    click.connect_pressed(move |gesture, count, x, y| {
        if count != 1 || state.composing.get() {
            return;
        }
        let Some(view) = state.view.upgrade() else {
            return;
        };
        let Some(iter) = char_at(&view, x, y) else {
            return;
        };
        let offset = iter.offset();
        if gesture
            .current_event_state()
            .contains(gdk::ModifierType::CONTROL_MASK)
        {
            state.refresh();
            if let Some(link) = state.link_at(offset) {
                state.preview.hide();
                link_preview::open_url(&view, &link.url);
                gesture.set_state(gtk::EventSequenceState::Claimed);
            }
        } else if !view.buffer().has_selection() && state.toggle_task(offset) {
            gesture.set_state(gtk::EventSequenceState::Claimed);
        }
    });
    view.add_controller(click);
    connect_hover(view, editor);
    view.connect_copy_clipboard(|view| {
        view.stop_signal_emission_by_name("copy-clipboard");
        copy_source(view, false);
    });
    view.connect_cut_clipboard(|view| {
        view.stop_signal_emission_by_name("cut-clipboard");
        copy_source(view, true);
    });
}

/// 포인터 바로 아래 글자. 가까운 경계로 반올림하지 않고, 줄 끝 너머 빈 곳은 None이다.
fn char_at(view: &TextView, x: f64, y: f64) -> Option<gtk::TextIter> {
    let (bx, by) = view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
    view.iter_at_position(bx, by).map(|(iter, _)| iter)
}

/// 링크 위 포인터를 카드 예약으로 옮긴다. 끄는 중이나 조합 중에는 띄우지 않는다.
fn connect_hover(view: &TextView, editor: &Rc<LiveEditor>) {
    let motion = gtk::EventControllerMotion::new();
    let state = editor.clone();
    motion.connect_motion(move |controller, x, y| {
        let Some(view) = state.view.upgrade() else {
            return;
        };
        let modifiers = controller.current_event_state();
        let link = if modifiers.contains(gdk::ModifierType::BUTTON1_MASK) || state.composing.get() {
            None
        } else {
            char_at(&view, x, y).and_then(|iter| {
                state
                    .link_at(iter.offset())
                    .map(|link| (link, iter.offset()))
            })
        };
        let owner = link.as_ref().map(|(link, _)| link.owner.clone());
        // 표면이 바뀌면 GTK가 제자리 이동을 합성한다. 진짜로 움직였을 때만 카드를 예약한다.
        let moved = state.pointer.replace(Some((x, y))) != Some((x, y));
        // Ctrl을 누르면 눌러서 여는 곳임을 커서로 알린다.
        let opens = owner.is_some() && modifiers.contains(gdk::ModifierType::CONTROL_MASK);
        view.set_cursor_from_name(Some(if opens { "pointer" } else { "text" }));
        if !moved || *state.hovered.borrow() == owner {
            return;
        }
        state.hovered.replace(owner);
        match link {
            Some((link, offset)) => state.preview.hover(
                Some(&link),
                LinkPreview::anchor(&view, offset, Some(x as i32)),
            ),
            None => state.preview.hover(None, gdk::Rectangle::new(0, 0, 0, 0)),
        }
    });
    let state = editor.clone();
    motion.connect_leave(move |_| {
        state.hovered.replace(None);
        state.preview.pointer_left();
    });
    view.add_controller(motion);
}

fn connect_keys(view: &TextView, editor: Rc<LiveEditor>, source: &CheckButton) {
    let source_toggle = source.downgrade();
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if key == gdk::Key::Escape && editor.preview.is_shown() {
            editor.preview.dismiss();
            return glib::Propagation::Stop;
        }
        if !editor.composing.get() {
            if let Some(view) = editor.view.upgrade() {
                if super::formatting::handle(&view, key, modifiers) {
                    return glib::Propagation::Stop;
                }
                if view.is_editable() && !editor.source.get() {
                    // 대기 중인 갱신을 먼저 반영해 커서와 항목 범위가 어긋나지 않게 한다.
                    if editor.pending.get() {
                        editor.refresh();
                    }
                    let document = editor.document.borrow();
                    if list_edit::handle(&view, &document, key, modifiers) {
                        return glib::Propagation::Stop;
                    }
                }
            }
        } else if matches!(key, gdk::Key::Return | gdk::Key::KP_Enter) {
            editor.commit_newline.set(true);
        }
        if (key == gdk::Key::m || key == gdk::Key::M)
            && modifiers.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK)
        {
            if let Some(source) = source_toggle.upgrade() {
                source.set_active(!source.is_active());
            }
            return glib::Propagation::Stop;
        }
        if key == gdk::Key::Return && modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
            if let Some(view) = editor.view.upgrade() {
                let buffer = view.buffer();
                if !buffer.has_selection() {
                    let cursor = buffer.iter_at_mark(&buffer.get_insert());
                    editor.refresh();
                    let task = editor
                        .document
                        .borrow()
                        .tasks
                        .iter()
                        .find(|task| {
                            buffer.iter_at_offset(task.range.start).line() == cursor.line()
                        })
                        .map(|task| task.range.start);
                    if task.is_some_and(|offset| editor.toggle_task(offset)) {
                        return glib::Propagation::Stop;
                    }
                    // 체크 항목이 아니면 커서 아래 링크를 연다. 키보드만으로 링크를 여는 경로다.
                    if let Some(link) = editor.link_at(cursor.offset()) {
                        editor.preview.hide();
                        link_preview::open_url(&view, &link.url);
                        return glib::Propagation::Stop;
                    }
                }
            }
        }
        glib::Propagation::Proceed
    });
    view.add_controller(keys);
}
