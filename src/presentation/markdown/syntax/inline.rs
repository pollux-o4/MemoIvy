use super::{
    links::{bare_links, label_end},
    list::{line_mark, scan_items, LineMark},
    safe_link, Document, Link, Marker, Span, Style, Task, TaskState,
};
use pulldown_cmark::{Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// 서식 문법을 뺀 글자. 링크 이름처럼 원문 조각을 사람이 읽는 글자로 바꿀 때 쓴다.
pub(crate) fn plain_line(line: &str) -> String {
    let source = line.trim();
    let mut out = String::new();
    let mut image_depth = 0;
    for event in Parser::new_ext(
        source,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    ) {
        match event {
            Event::Start(Tag::Image { .. }) => {
                image_depth += 1;
                if image_depth == 1 {
                    out.push_str("[이미지] ");
                }
            }
            Event::End(TagEnd::Image) => image_depth -= 1,
            // 설명 뒤의 `|너비`는 표시 문법이라 뺀다.
            Event::Text(text) if image_depth > 0 => {
                out.push_str(text.rsplit_once('|').map_or(&*text, |(alt, _)| alt))
            }
            Event::Text(text) | Event::Code(text) => out.push_str(&text),
            Event::SoftBreak | Event::HardBreak => out.push(' '),
            _ => {}
        }
    }
    let out = out.trim();
    let only_markers = source.chars().all(|c| !c.is_alphanumeric());
    if out.is_empty() && !only_markers {
        source.to_owned()
    } else {
        out.to_owned()
    }
}

pub(crate) fn parse(text: &str) -> Document {
    let mut offsets = vec![0; text.len() + 1];
    let mut count = 0;
    for (byte, _) in text.char_indices() {
        offsets[byte] = count;
        count += 1;
    }
    offsets[text.len()] = count;
    let chars = |range: Range<usize>| offsets[range.start]..offsets[range.end];
    let mut doc = Document::default();
    let mut image_depth = 0;
    // 일반 주소를 찾을 때 건너뛸 구간: 코드와 이미 링크인 곳.
    let mut literal: Vec<Range<usize>> = Vec::new();
    // shortcut: 전체 파싱은 입력 idle마다 한 번 병합, PERF-001 크기를 넘는 문서에서 지연이 측정되면 증분 파싱 검토.
    for (event, range) in Parser::new_ext(
        text,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    )
    .into_offset_iter()
    {
        match &event {
            Event::Start(Tag::Image { .. }) => {
                image_depth += 1;
                literal.push(range.clone());
                if image_depth == 1 {
                    doc.images
                        .extend(super::media::block_image(text, range.clone(), &chars));
                }
                continue;
            }
            Event::End(TagEnd::Image) => {
                image_depth -= 1;
                continue;
            }
            _ if image_depth > 0 => continue,
            _ => {}
        }
        let raw = &text[range.clone()];
        let mut style = None;
        let mut markers = Vec::new();
        match event {
            Event::Code(_) | Event::Html(_) | Event::InlineHtml(_) => literal.push(range.clone()),
            Event::Start(Tag::CodeBlock(_)) | Event::Rule => literal.push(range.clone()),
            Event::Start(Tag::Heading { level, .. }) => {
                let prefix = match level {
                    HeadingLevel::H1 => "# ",
                    HeadingLevel::H2 => "## ",
                    _ => continue,
                };
                if raw.starts_with(prefix) {
                    style = Some(if level == HeadingLevel::H1 {
                        Style::Heading1
                    } else {
                        Style::Heading2
                    });
                    markers.push(range.start..range.start + prefix.len());
                }
            }
            Event::Start(tag @ (Tag::Strong | Tag::Emphasis | Tag::Strikethrough)) => {
                let width = if tag == Tag::Emphasis { 1 } else { 2 };
                style = Some(match tag {
                    Tag::Strong => Style::Bold,
                    Tag::Emphasis => Style::Italic,
                    _ => Style::Strike,
                });
                markers.push(range.start..range.start + width);
                markers.push(range.end - width..range.end);
            }
            Event::Start(Tag::Link {
                link_type: LinkType::Inline,
                dest_url,
                ..
            }) => {
                literal.push(range.clone());
                if !safe_link(&dest_url) {
                    continue;
                }
                if let Some(end) = label_end(raw) {
                    style = Some(Style::Link);
                    markers.push(range.start..range.start + 1);
                    markers.push(range.start + end..range.end);
                    let url_range = raw[end..].find(dest_url.as_ref()).map_or(
                        range.start + end + 2..range.end - 1,
                        |at| {
                            let at = range.start + end + at;
                            at..at + dest_url.len()
                        },
                    );
                    doc.links.push(Link {
                        range: chars(range.start + 1..range.start + end),
                        owner: chars(range.clone()),
                        url_range: chars(url_range),
                        url: dest_url.to_string(),
                    });
                }
            }
            Event::Start(Tag::Link {
                link_type: LinkType::Autolink,
                dest_url,
                ..
            }) => {
                literal.push(range.clone());
                if safe_link(&dest_url) && raw.len() > 2 {
                    style = Some(Style::Link);
                    markers.push(range.start..range.start + 1);
                    markers.push(range.end - 1..range.end);
                    let inner = range.start + 1..range.end - 1;
                    doc.links.push(Link {
                        range: chars(inner.clone()),
                        owner: chars(range.clone()),
                        url_range: chars(inner),
                        url: dest_url.to_string(),
                    });
                }
            }
            Event::Start(Tag::Link { .. }) => literal.push(range.clone()),
            _ => {}
        }
        if let Some(style) = style {
            doc.spans.push(Span {
                range: chars(range.clone()),
                style,
            });
        }
        for marker in markers {
            doc.markers.push(Marker {
                range: chars(marker),
                owner: chars(range.clone()),
            });
        }
    }
    bare_links(text, &literal, &chars, &mut doc);
    scan_items(text, &literal, &chars, &mut doc);
    scan_external_tasks(text, &literal, &chars, &mut doc);
    doc.tasks.sort_by_key(|task| task.range.start);
    doc
}

/// 목록 파서가 소유하지 않는 인용문·번호 목록의 체크도 같은 표식 규칙으로 읽는다.
fn scan_external_tasks(
    text: &str,
    literal: &[Range<usize>],
    chars: &impl Fn(Range<usize>) -> Range<i32>,
    doc: &mut Document,
) {
    let mut at = 0;
    for raw in text.split_inclusive('\n') {
        let line_start = at;
        at += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let prefix = quote_prefix(line);
        let source = &line[prefix..];
        let Some(mark) = line_mark(source) else {
            continue;
        };
        let Some((check, state)) = task_marker(source, &mark) else {
            continue;
        };
        let source_range = line_start + prefix + check.start..line_start + prefix + check.end;
        if literal
            .iter()
            .any(|range| range.contains(&source_range.start))
        {
            continue;
        }
        let range = chars(source_range);
        if doc
            .items
            .iter()
            .any(|item| item.marker.contains(&range.start))
        {
            continue;
        }
        doc.spans.push(Span {
            range: range.clone(),
            style: Style::Task,
        });
        doc.tasks.push(Task {
            range,
            state,
            item: None,
        });
    }
}

fn task_marker(line: &str, mark: &LineMark) -> Option<(Range<usize>, TaskState)> {
    if let (Some(range), Some(state)) = (&mark.check, mark.task_state) {
        return Some((range.clone(), state));
    }
    if mark.kind != super::ListKind::Ordered {
        return None;
    }
    let tail = line.get(mark.body..)?;
    let marker = tail.get(..3)?;
    if !matches!(tail.as_bytes().get(3), Some(b' ' | b'\t')) {
        return None;
    }
    Some((mark.body..mark.body + 3, TaskState::from_marker(marker)?))
}

fn quote_prefix(line: &str) -> usize {
    let mut at = 0;
    let mut found = false;
    loop {
        let mut marker = at;
        let mut spaces = 0;
        while matches!(line.as_bytes().get(marker), Some(b' ' | b'\t')) && spaces < 3 {
            marker += 1;
            spaces += 1;
        }
        if line.as_bytes().get(marker) != Some(&b'>') {
            break;
        }
        found = true;
        at = marker + 1;
        if matches!(line.as_bytes().get(at), Some(b' ' | b'\t')) {
            at += 1;
        }
    }
    if found {
        at
    } else {
        0
    }
}
