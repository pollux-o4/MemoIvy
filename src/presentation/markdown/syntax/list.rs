use super::{Document, ListItem, ListKind, Span, Style, Task, TaskState};
use std::ops::Range;

pub(in crate::presentation::markdown) struct LineMark {
    /// 들여쓰기 뒤 표식 시작(바이트).
    pub(in crate::presentation::markdown) start: usize,
    /// 본문 시작(바이트).
    pub(in crate::presentation::markdown) body: usize,
    /// 들여쓰기 칸 수.
    pub(in crate::presentation::markdown) columns: usize,
    /// 목록 표식 바로 뒤 본문 시작 칸. 체크 표식은 본문에 포함한다.
    pub(in crate::presentation::markdown) continuation_indent: usize,
    pub(in crate::presentation::markdown) kind: ListKind,
    pub(in crate::presentation::markdown) task_state: Option<TaskState>,
    /// 체크 괄호 범위(바이트).
    pub(in crate::presentation::markdown) check: Option<Range<usize>>,
}

pub(in crate::presentation::markdown) fn indent_columns(text: &str) -> usize {
    text.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum()
}

/// 한 줄의 목록 표식. 쓰는 동안 바로 보여야 하므로 문단 뒤나 빈 항목도 줄 단위로 인식한다.
/// 공백까지 쳐야 표식이다. `-5도`나 입력 중인 `-`가 잠깐 기호로 바뀌지 않는다.
pub(in crate::presentation::markdown) fn line_mark(line: &str) -> Option<LineMark> {
    let body = line.trim_start_matches([' ', '\t']);
    let start = line.len() - body.len();
    let columns = indent_columns(&line[..start]);
    let bytes = body.as_bytes();
    let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    let width = if matches!(bytes.first(), Some(b'-' | b'*' | b'+')) {
        1
    } else if (1..=9).contains(&digits) && matches!(bytes.get(digits), Some(b'.' | b')')) {
        digits + 1
    } else {
        return None;
    };
    if !matches!(bytes.get(width), Some(b' ' | b'\t')) {
        return None;
    }
    let after = start + width;
    let pad = |from: usize| {
        from + line[from..]
            .bytes()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count()
    };
    let mut body_at = pad(after);
    let mut continuation_indent = columns + width + indent_columns(&line[after..body_at]);
    let mut kind = if digits > 0 {
        ListKind::Ordered
    } else {
        ListKind::Bullet
    };
    let mut check = None;
    let mut task_state = None;
    if digits == 0 {
        if let Some(marker) = line.get(body_at..body_at + 3) {
            if let Some(state) = TaskState::from_marker(marker) {
                if matches!(line.as_bytes().get(body_at + 3), Some(b' ' | b'\t')) {
                    check = Some(body_at..body_at + 3);
                    task_state = Some(state);
                    kind = ListKind::Task;
                    let check_end = body_at + 3;
                    body_at = pad(check_end);
                    continuation_indent += 3 + indent_columns(&line[check_end..body_at]);
                }
            }
        }
    }
    Some(LineMark {
        start,
        body: body_at,
        columns,
        continuation_indent,
        kind,
        task_state,
        check,
    })
}

pub(in crate::presentation::markdown) fn scan_items(
    text: &str,
    literal: &[Range<usize>],
    chars: &impl Fn(Range<usize>) -> Range<i32>,
    doc: &mut Document,
) {
    let mut parents: Vec<usize> = Vec::new();
    let mut continuation_indents = Vec::new();
    let mut previous_item: Option<usize> = None;
    let mut at = 0;
    for raw in text.split_inclusive('\n') {
        let line_start = at;
        at += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let Some(mark) = line_mark(line) else {
            let prefix_len = line.len() - line.trim_start_matches([' ', '\t']).len();
            let indent = indent_columns(&line[..prefix_len]) as i32;
            if let Some(index) =
                previous_item.filter(|&index| indent >= continuation_indents[index] as i32)
            {
                doc.items[index].content.end = chars(line_start..line_start + line.len()).end;
                previous_item = Some(index);
            } else {
                // 들여쓰기가 없는 빈 줄은 다음 목록 표식의 깊이만 유지하고 이어쓰기는 끝낸다.
                previous_item = None;
            }
            if !line.trim().is_empty() && !line.starts_with([' ', '\t']) {
                parents.clear();
            }
            continue;
        };
        if literal
            .iter()
            .any(|skip| skip.contains(&(line_start + mark.start)))
        {
            continue;
        }
        while parents.last().is_some_and(|indent| *indent >= mark.columns) {
            parents.pop();
        }
        let depth = parents.len();
        parents.push(mark.columns);
        let content = chars(line_start + mark.body..line_start + line.len());
        let item_index = doc.items.len();
        doc.items.push(ListItem {
            marker: chars(line_start..line_start + mark.body),
            content,
            depth,
            kind: mark.kind,
        });
        if let (Some(check), Some(state)) = (&mark.check, mark.task_state) {
            doc.tasks.push(Task {
                range: chars(line_start + check.start..line_start + check.end),
                state,
                item: Some(item_index),
            });
        }
        continuation_indents.push(mark.continuation_indent);
        previous_item = Some(doc.items.len() - 1);
    }
    doc.spans.extend(
        doc.tasks
            .iter()
            .filter(|task| task.state == TaskState::Done)
            .filter_map(|task| task.item.and_then(|index| doc.items.get(index)))
            .filter(|item| item.content.start < item.content.end)
            .map(|item| Span {
                range: item.content.clone(),
                style: Style::TaskDone,
            }),
    );
}
