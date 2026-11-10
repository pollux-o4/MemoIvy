use super::*;

#[derive(Default)]
pub(super) struct LayoutState {
    pub paragraphs: RefCell<ParagraphTags>,
    pub allocations: RefCell<Vec<Box<dyn Fn()>>>,
}

pub(super) struct ListIndent {
    pub text_x: i32,
    pub first_line: (i32, i32),
}

pub(super) fn list_indent(
    view: &TextView,
    buffer: &TextBuffer,
    item: &markdown::ListItem,
) -> ListIndent {
    let slot = GUTTER * (item.depth as i32 + 1);
    let prefix_width = match item.kind {
        ListKind::Task | ListKind::Bullet => 0,
        ListKind::Ordered => {
            let prefix = super::super::source_slice(buffer, item.marker.clone());
            (f64::from(view.create_pango_layout(Some(&prefix)).size().0) / f64::from(pango::SCALE))
                .floor() as i32
        }
    };
    let text_x = view.left_margin() + slot;
    ListIndent {
        text_x,
        // GTK adds a negative indent to subsequent lines; reserve the prefix only on the first.
        first_line: ((text_x - prefix_width).max(0), -prefix_width),
    }
}

impl LiveEditor {
    /// 표식은 한 글자처럼 다룬다. 커서가 그 안에 놓이면 본문 앞으로 옮겨 문법 앞에 글자가 들어가지 않게 한다.
    pub(super) fn leave_marker(&self, buffer: &TextBuffer, document: &Document) {
        if buffer.has_selection() || self.source.get() {
            return;
        }
        let at = buffer.iter_at_mark(&buffer.get_insert()).offset();
        if let Some(item) = document
            .item_at(at)
            .map(|(_, item)| item)
            .filter(|item| item.marker.contains(&at))
        {
            buffer.place_cursor(&buffer.iter_at_offset(item.content.start));
        }
    }

    /// 표식 글자는 왼쪽 여백으로 내어 써 본문 시작 위치를 항상 같게 한다.
    pub(super) fn sync_paragraphs(&self, view: &TextView, document: &Document) {
        let buffer = view.buffer();
        let mut wanted: HashMap<(i32, i32), Vec<Range<i32>>> = HashMap::new();
        if !self.source.get() {
            for item in &document.items {
                let mut first_line_end = buffer.iter_at_offset(item.content.start);
                // At an empty body GTK would advance to the next paragraph's end.
                if !first_line_end.ends_line() {
                    first_line_end.forward_to_line_end();
                }
                let mut line_start = buffer.iter_at_offset(item.marker.start);
                line_start.set_line_offset(0);
                let line = line_start.offset()..first_line_end.offset();
                let indent = list_indent(view, &buffer, item);
                wanted.entry(indent.first_line).or_default().push(line);
                for (line, _) in continuation_lines(&buffer, item) {
                    wanted.entry((indent.text_x, 0)).or_default().push(line);
                }
            }
        }
        let mut tags = self.layout.paragraphs.borrow_mut();
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
}

/// 목록 항목에 속한 들여쓰기 이어쓰기 줄과 숨길 원문 접두부.
pub(super) fn continuation_lines(
    buffer: &TextBuffer,
    item: &markdown::ListItem,
) -> Vec<(Range<i32>, Range<i32>)> {
    let mut line = buffer.iter_at_offset(item.content.start);
    if !line.ends_line() {
        line.forward_to_line_end();
    }
    if line.offset() >= item.content.end || !line.forward_char() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    while line.offset() <= item.content.end {
        let start = line.offset();
        let mut end = line;
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        let mut content = line;
        while content.offset() < end.offset() && matches!(content.char(), ' ' | '\t') {
            content.forward_char();
        }
        lines.push((start..end.offset(), start..content.offset()));
        if end.offset() >= item.content.end || !end.forward_char() {
            break;
        }
        line = end;
    }
    lines
}

pub(super) fn continuation_indents(
    buffer: &TextBuffer,
    item: &markdown::ListItem,
) -> Vec<Range<i32>> {
    continuation_lines(buffer, item)
        .into_iter()
        .map(|(_, indent)| indent)
        .filter(|indent| !indent.is_empty())
        .collect()
}

pub(super) fn active_lines(buffer: &TextBuffer) -> Range<i32> {
    let (mut start, mut end) = buffer.selection_bounds().unwrap_or_else(|| {
        let cursor = buffer.iter_at_mark(&buffer.get_insert());
        (cursor, cursor)
    });
    start.set_line_offset(0);
    let has_next = end.forward_line();
    // Include an empty last line and the caret at EOF, without revealing the next line.
    start.offset()..(end.offset() + i32::from(!has_next))
}
