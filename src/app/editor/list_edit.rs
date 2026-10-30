//! 목록을 쓰는 동안의 키 동작. 원문을 바꾸는 작업은 모두 한 번의 undo로 되돌린다.
use crate::presentation::markdown::{Document, ListItem, ListKind};
use gtk::{gdk, prelude::*, TextBuffer, TextView};

const INDENT: &str = "  ";

fn slice(buffer: &TextBuffer, start: i32, end: i32) -> String {
    buffer
        .text(
            &buffer.iter_at_offset(start),
            &buffer.iter_at_offset(end),
            true,
        )
        .to_string()
}

fn delete(buffer: &TextBuffer, start: i32, end: i32) {
    buffer.delete(
        &mut buffer.iter_at_offset(start),
        &mut buffer.iter_at_offset(end),
    );
}

fn cursor(buffer: &TextBuffer) -> i32 {
    buffer.iter_at_mark(&buffer.get_insert()).offset()
}

fn item_at(document: &Document, offset: i32) -> Option<(usize, &ListItem)> {
    document
        .items
        .iter()
        .enumerate()
        .find(|(_, item)| item.marker.start <= offset && offset <= item.content.end)
}

/// 다음 항목의 표식. 체크는 비운 상태로 시작하고 번호는 하나 늘린다.
fn next_marker(marker: &str, kind: ListKind) -> String {
    let body = marker.trim_start();
    let indent = &marker[..marker.len() - body.len()];
    match kind {
        ListKind::Bullet => format!("{indent}{} ", &body[..1]),
        ListKind::Task(_) => format!("{indent}{} [ ] ", &body[..1]),
        ListKind::Ordered => {
            let digits = body.bytes().take_while(u8::is_ascii_digit).count();
            let number: u64 = body[..digits].parse().unwrap_or(0);
            format!("{indent}{}{} ", number + 1, &body[digits..=digits])
        }
    }
}

/// 사이에 새 번호가 끼면 이어지는 같은 단계 번호를 민다. 뒤에서부터 바꿔 앞 위치가 흔들리지 않는다.
fn renumber_after(buffer: &TextBuffer, document: &Document, index: usize, first: u64) {
    let depth = document.items[index].depth;
    let mut changes = Vec::new();
    let mut number = first;
    let mut line = buffer
        .iter_at_offset(document.items[index].marker.start)
        .line();
    for item in &document.items[index + 1..] {
        let at = buffer.iter_at_offset(item.marker.start).line();
        if at != line + 1 || item.depth < depth {
            break;
        }
        line = at;
        if item.depth > depth {
            continue;
        }
        if item.kind != ListKind::Ordered {
            break;
        }
        let marker = slice(buffer, item.marker.start, item.marker.end);
        let lead = marker.len() - marker.trim_start().len();
        let digits = marker[lead..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if marker[lead..lead + digits].parse::<u64>().ok() != Some(number) {
            let start = item.marker.start + lead as i32;
            changes.push((start, start + digits as i32, number));
        }
        number += 1;
    }
    for (start, end, value) in changes.into_iter().rev() {
        delete(buffer, start, end);
        buffer.insert(&mut buffer.iter_at_offset(start), &value.to_string());
    }
}

fn outdent(buffer: &TextBuffer, item: &ListItem) -> bool {
    let marker = slice(buffer, item.marker.start, item.marker.end);
    let width = if marker.starts_with('\t') {
        1
    } else {
        marker.chars().take(2).take_while(|c| *c == ' ').count()
    };
    if width > 0 {
        delete(buffer, item.marker.start, item.marker.start + width as i32);
    }
    width > 0
}

/// 항목을 풀어 평문으로 만든다. 들여쓴 항목은 먼저 한 단계 내어 쓴다.
fn unlist(buffer: &TextBuffer, item: &ListItem) {
    buffer.begin_user_action();
    if item.depth == 0 || !outdent(buffer, item) {
        delete(buffer, item.marker.start, item.marker.end);
    }
    buffer.end_user_action();
}

fn split(view: &TextView, document: &Document, index: usize, item: &ListItem) {
    let buffer = view.buffer();
    let next = next_marker(
        &slice(&buffer, item.marker.start, item.marker.end),
        item.kind,
    );
    buffer.begin_user_action();
    if item.kind == ListKind::Ordered {
        let number = next
            .trim_start()
            .bytes()
            .take_while(u8::is_ascii_digit)
            .fold(0u64, |value, digit| value * 10 + u64::from(digit - b'0'));
        renumber_after(&buffer, document, index, number + 1);
    }
    buffer.insert_at_cursor(&format!("\n{next}"));
    buffer.end_user_action();
    view.scroll_mark_onscreen(&buffer.get_insert());
}

fn indent_lines(buffer: &TextBuffer, document: &Document, backwards: bool) -> bool {
    let (start, end) = buffer.selection_bounds().map_or_else(
        || {
            let at = cursor(buffer);
            (at, at)
        },
        |(a, b)| (a.offset(), b.offset()),
    );
    let items: Vec<_> = document
        .items
        .iter()
        .filter(|item| item.content.end >= start && item.marker.start <= end)
        .collect();
    if items.is_empty() {
        return false;
    }
    buffer.begin_user_action();
    for item in items.iter().rev() {
        if backwards {
            outdent(buffer, item);
        } else {
            buffer.insert(&mut buffer.iter_at_offset(item.marker.start), INDENT);
        }
    }
    buffer.end_user_action();
    true
}

/// 처리했으면 true. 조합 중·잠금·원문 보기에서는 호출하지 않는다.
pub(super) fn handle(
    view: &TextView,
    document: &Document,
    key: gdk::Key,
    modifiers: gdk::ModifierType,
) -> bool {
    let buffer = view.buffer();
    let mods = modifiers
        & (gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SHIFT_MASK
            | gdk::ModifierType::ALT_MASK);
    if mods == gdk::ModifierType::empty() && key == gdk::Key::Tab {
        return indent_lines(&buffer, document, false);
    }
    if key == gdk::Key::ISO_Left_Tab
        || (mods == gdk::ModifierType::SHIFT_MASK && key == gdk::Key::Tab)
    {
        return indent_lines(&buffer, document, true);
    }
    if buffer.has_selection() {
        return false;
    }
    let at = cursor(&buffer);
    let found = item_at(document, at);
    // 표식은 한 글자처럼 다룬다. 왼쪽으로 가면 표식을 건너 앞 줄 끝으로 간다.
    if key == gdk::Key::Left
        && (mods == gdk::ModifierType::empty() || mods == gdk::ModifierType::CONTROL_MASK)
    {
        let Some((_, item)) = found.filter(|(_, item)| item.content.start == at) else {
            return false;
        };
        if item.marker.start > 0 {
            buffer.place_cursor(&buffer.iter_at_offset(item.marker.start - 1));
        }
        return true;
    }
    if mods != gdk::ModifierType::empty() {
        return false;
    }
    match key {
        gdk::Key::Return | gdk::Key::KP_Enter => {
            let Some((index, item)) = found else {
                return false;
            };
            if item.content.start == item.content.end {
                // 빈 항목에서 한 번 더 누르면 목록을 끝낸다.
                unlist(&buffer, item);
            } else {
                split(view, document, index, item);
            }
            true
        }
        gdk::Key::BackSpace => {
            let Some((_, item)) = found.filter(|(_, item)| item.content.start == at) else {
                return false;
            };
            unlist(&buffer, item);
            true
        }
        gdk::Key::Delete => {
            // 다음 줄이 항목이면 표식까지 지워 본문만 이어 붙인다.
            let Some(next) = document
                .items
                .iter()
                .find(|item| item.marker.start == at + 1)
            else {
                return false;
            };
            if !buffer.iter_at_offset(at).ends_line() {
                return false;
            }
            buffer.begin_user_action();
            delete(&buffer, at, next.marker.end);
            buffer.end_user_action();
            true
        }
        _ => false,
    }
}

/// 조합 중에 누른 Enter는 입력기가 처리해 줄바꿈만 남는다. 확정된 뒤 다음 표식을 채운다.
pub(super) fn continue_after_commit(view: &TextView, document: &Document) {
    let buffer = view.buffer();
    let at = buffer.iter_at_mark(&buffer.get_insert());
    if !at.ends_line() || at.line_offset() != 0 || at.offset() == 0 {
        return;
    }
    let Some((_, item)) = item_at(document, at.offset() - 1) else {
        return;
    };
    if item.content.start < item.content.end {
        let next = next_marker(
            &slice(&buffer, item.marker.start, item.marker.end),
            item.kind,
        );
        buffer.begin_user_action();
        buffer.insert_at_cursor(&next);
        buffer.end_user_action();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fmt_003_next_marker_keeps_style_and_indent() {
        assert_eq!(next_marker("- ", ListKind::Bullet), "- ");
        assert_eq!(next_marker("  * ", ListKind::Bullet), "  * ");
        assert_eq!(next_marker("- [x] ", ListKind::Task(true)), "- [ ] ");
        assert_eq!(next_marker("9. ", ListKind::Ordered), "10. ");
        assert_eq!(next_marker("  3) ", ListKind::Ordered), "  4) ");
    }
}
