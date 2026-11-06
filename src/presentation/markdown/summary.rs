use super::{
    list::{indent_columns, line_mark},
    plain_line, LineMarker, ListKind, PreviewLine, PreviewRow, Summary,
};

const PREVIEW_CHARS: usize = 240;
const PREVIEW_LINES: usize = 12;

/// 카드에 보일 요약. 서식 기호를 뺀 첫 줄을 제목, 나머지를 미리보기로 쓴다.
/// 카드에 보일 만큼만 읽어 긴 문서에서도 비용이 일정하다.
pub(crate) fn summary(text: &str) -> Summary {
    let mut summary = Summary::default();
    let mut has_title = false;
    let mut fence: Option<&str> = None;
    let mut owner: Option<(LineMarker, usize, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let opener = ["```", "~~~"]
            .into_iter()
            .find(|f| trimmed.starts_with(f) && !trimmed.trim_start_matches('`').contains('`'));
        let literal = match (fence, opener) {
            (Some(open), Some(f)) if f == open => {
                fence = None;
                owner = None;
                continue;
            }
            (Some(_), _) => true,
            (None, Some(f)) => {
                fence = Some(f);
                owner = None;
                continue;
            }
            (None, None) => false,
        };
        // 다 담은 뒤에는 파싱 없이 남은 줄 수만 센다.
        if summary.lines.len() >= PREVIEW_LINES && summary.preview.chars().count() >= PREVIEW_CHARS
        {
            summary.remaining += usize::from(trimmed.chars().any(char::is_alphanumeric));
            continue;
        }
        let (marker, depth, rest, continuation) = if literal {
            owner = None;
            (None, 0, line, false)
        } else if let Some((marker, depth, rest, indent)) = classify(line) {
            owner = Some((marker.clone(), depth, indent));
            (Some(marker), depth, rest, false)
        } else {
            let indent_bytes = line.len() - line.trim_start_matches([' ', '\t']).len();
            let indent = indent_columns(&line[..indent_bytes]);
            if let Some((marker, depth, _required)) = owner
                .as_ref()
                .filter(|(_, _, required)| indent >= *required)
            {
                (Some(marker.clone()), *depth, line.trim_start(), true)
            } else {
                owner = None;
                (None, 0, line, false)
            }
        };
        let plain = if literal {
            line.trim().to_owned()
        } else {
            plain_line(rest)
        };
        if plain.is_empty() {
            continue;
        }
        if !has_title {
            has_title = true;
            summary.title = plain;
            summary.title_marker = marker;
            continue;
        }
        if summary.preview.chars().count() < PREVIEW_CHARS {
            if !summary.preview.is_empty() {
                summary.preview.push(' ');
            }
            summary.preview.push_str(&plain);
        }
        if summary.lines.len() < PREVIEW_LINES {
            summary.lines.push(PreviewLine {
                marker,
                depth,
                text: plain,
                continuation,
            });
        } else {
            summary.remaining += 1;
        }
    }
    summary
}

/// 목록 표식과 깊이만 읽는다. 카드 요약은 줄 단위라 문서 전체를 파싱하지 않는다.
fn classify(line: &str) -> Option<(LineMarker, usize, &str, usize)> {
    let mark = line_mark(line)?;
    let marker = match mark.kind {
        ListKind::Bullet => LineMarker::Bullet,
        ListKind::Task => LineMarker::Task(mark.task_state?),
        ListKind::Ordered => {
            let end = line[mark.start..]
                .find(['.', ')'])
                .map_or(mark.start, |at| mark.start + at + 1);
            LineMarker::Number(line[mark.start..end].to_owned())
        }
    };
    Some((
        marker,
        mark.columns / 2,
        &line[mark.body..],
        mark.continuation_indent,
    ))
}

/// 표시 줄 예산 안에서 행을 고른다. 돌려주는 수는 담지 못한 원본 줄 수다.
pub(crate) fn fit_preview(lines: &[PreviewLine], budget: i32) -> (Vec<PreviewRow>, usize) {
    // 연속 문단은 합쳐 지금처럼 이어 읽게 한다.
    let mut blocks: Vec<(PreviewLine, usize)> = Vec::new();
    for line in lines {
        match blocks.last_mut() {
            Some((block, count)) if block.marker.is_none() && line.marker.is_none() => {
                block.text.push(' ');
                block.text.push_str(&line.text);
                *count += 1;
            }
            _ => blocks.push((line.clone(), 1)),
        }
    }
    let shown = blocks.len().min(budget.max(0) as usize);
    let mut left = budget;
    let mut rows = Vec::with_capacity(shown);
    let mut used = 0;
    for (index, (line, count)) in blocks.into_iter().take(shown).enumerate() {
        let following = (shown - index - 1) as i32;
        let max_lines = if line.marker.is_none() {
            (left - following).max(1)
        } else {
            1
        };
        left -= max_lines;
        used += count;
        rows.push(PreviewRow { line, max_lines });
    }
    (rows, lines.len() - used)
}
