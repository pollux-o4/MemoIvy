use super::*;

impl LiveEditor {
    /// 표식은 한 글자처럼 다룬다. 커서가 그 안에 놓이면 본문 앞으로 옮겨 문법 앞에 글자가 들어가지 않게 한다.
    pub(super) fn leave_marker(&self, buffer: &TextBuffer, document: &Document) {
        if buffer.has_selection() || self.source.get() {
            return;
        }
        let at = buffer.iter_at_mark(&buffer.get_insert()).offset();
        if let Some(item) = document.items.iter().find(|i| i.marker.contains(&at)) {
            buffer.place_cursor(&buffer.iter_at_offset(item.content.start));
        }
    }

    /// 표식 글자는 왼쪽 여백으로 내어 써 본문 시작 위치를 항상 같게 한다.
    pub(super) fn sync_paragraphs(
        &self,
        view: &TextView,
        document: &Document,
        selected: &Range<i32>,
    ) {
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
    pub(super) fn correct_hang(self: &Rc<Self>) {
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
}

/// 선택이 닿은 줄 범위. 선택이 없으면 비어 있다.
pub(super) fn selected_lines(buffer: &TextBuffer) -> Range<i32> {
    if buffer.has_selection() {
        active_lines(buffer)
    } else {
        0..0
    }
}

/// 선택 중 표식 글자를 뺄 항목. 번호는 그 글자가 곧 표식이고 빈 항목은 줄이 접혀 뺄 수 없다.
pub(super) fn hides_marker(item: &markdown::ListItem, selected: &Range<i32>) -> bool {
    item.kind != ListKind::Ordered
        && item.content.start < item.content.end
        && selected.start < selected.end
        && item.marker.start < selected.end
        && item.content.end >= selected.start
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
