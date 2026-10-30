use super::*;

impl LiveEditor {
    /// 표시 전용 태그 색은 CSS 토큰에서 읽는다. 테마가 바뀌면 다음 갱신에 따라간다.
    #[allow(deprecated)]
    pub(super) fn sync_theme(&self, view: &TextView) {
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
}

pub(super) fn create_tags(
    buffer: &TextBuffer,
) -> (Vec<(Style, TextTag)>, TextTag, TextTag, TextTag, TextTag) {
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

// GTK already moves tag boundaries with edits; change only ranges whose decoration differs.
pub(super) fn reconcile(buffer: &TextBuffer, tag: &TextTag, mut desired: Vec<Range<i32>>) {
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
