use super::{Document, ListItem, ListKind, Span, Style, Task};
use std::ops::Range;

pub(super) struct LineMark {
    /// 들여쓰기 뒤 표식 시작(바이트).
    pub(super) start: usize,
    /// 본문 시작(바이트).
    pub(super) body: usize,
    /// 들여쓰기 칸 수.
    pub(super) columns: usize,
    pub(super) kind: ListKind,
    /// 체크 괄호 범위(바이트).
    check: Option<Range<usize>>,
}

/// 한 줄의 목록 표식. 쓰는 동안 바로 보여야 하므로 문단 뒤나 빈 항목도 줄 단위로 인식한다.
/// 공백까지 쳐야 표식이다. `-5도`나 입력 중인 `-`가 잠깐 기호로 바뀌지 않는다.
pub(super) fn line_mark(line: &str) -> Option<LineMark> {
    let body = line.trim_start_matches([' ', '\t']);
    let start = line.len() - body.len();
    let columns = line[..start]
        .chars()
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum();
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
    let mut kind = if digits > 0 {
        ListKind::Ordered
    } else {
        ListKind::Bullet
    };
    let mut check = None;
    if digits == 0 {
        for (mark, checked) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
            if line[body_at..].starts_with(mark)
                && matches!(line.as_bytes().get(body_at + 3), Some(b' ' | b'\t'))
            {
                check = Some(body_at..body_at + 3);
                kind = ListKind::Task(checked);
                body_at = pad(body_at + 3);
                break;
            }
        }
    }
    Some(LineMark {
        start,
        body: body_at,
        columns,
        kind,
        check,
    })
}

pub(super) fn scan_items(
    text: &str,
    literal: &[Range<usize>],
    chars: &impl Fn(Range<usize>) -> Range<i32>,
    doc: &mut Document,
) {
    let mut parents: Vec<usize> = Vec::new();
    let mut at = 0;
    for raw in text.split_inclusive('\n') {
        let line_start = at;
        at += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let Some(mark) = line_mark(line) else {
            // 들여 쓴 이어짐 줄은 목록을 끊지 않는다.
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
        if let (Some(check), ListKind::Task(checked)) = (&mark.check, mark.kind) {
            doc.tasks.push(Task {
                range: chars(line_start + check.start..line_start + check.end),
                checked,
            });
            if checked && content.start < content.end {
                doc.spans.push(Span {
                    range: content.clone(),
                    style: Style::TaskDone,
                });
            }
        }
        doc.items.push(ListItem {
            marker: chars(line_start..line_start + mark.body),
            content,
            depth,
            kind: mark.kind,
        });
    }
}
