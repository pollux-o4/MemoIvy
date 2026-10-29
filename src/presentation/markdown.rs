use pulldown_cmark::{Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Heading1,
    Heading2,
    Bold,
    Italic,
    Strike,
    Link,
    Task,
}

#[derive(Debug)]
pub(crate) struct Span {
    pub range: Range<i32>,
    pub style: Style,
}

#[derive(Debug)]
pub(crate) struct Marker {
    pub range: Range<i32>,
    pub owner: Range<i32>,
}

#[derive(Debug)]
pub(crate) struct Link {
    pub range: Range<i32>,
    pub url: String,
}

#[derive(Debug)]
pub(crate) struct Task {
    pub display_start: i32,
    pub range: Range<i32>,
    pub checked: bool,
}

#[derive(Default, Debug)]
pub(crate) struct Document {
    pub spans: Vec<Span>,
    pub markers: Vec<Marker>,
    pub links: Vec<Link>,
    pub tasks: Vec<Task>,
}

pub(crate) fn safe_link(url: &str) -> bool {
    !url.chars().any(char::is_control)
        && ["https://", "http://", "mailto:"]
            .iter()
            .any(|prefix| url.to_ascii_lowercase().starts_with(prefix))
}

/// GTK offsets count characters; the parser reports UTF-8 byte ranges.
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
                if !safe_link(&dest_url) {
                    continue;
                }
                if let Some(end) = label_end(raw) {
                    style = Some(Style::Link);
                    markers.push(range.start..range.start + 1);
                    markers.push(range.start + end..range.end);
                    doc.links.push(Link {
                        range: chars(range.start + 1..range.start + end),
                        url: dest_url.to_string(),
                    });
                }
            }
            Event::TaskListMarker(checked) => {
                let has_bullet = ["- ", "* ", "+ "]
                    .iter()
                    .any(|prefix| text[..range.start].ends_with(prefix));
                doc.tasks.push(Task {
                    display_start: offsets[range.start - if has_bullet { 2 } else { 0 }],
                    range: chars(range.clone()),
                    checked,
                });
                style = Some(Style::Task);
            }
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
    doc
}

// Only hide a link's syntax when its label boundary is unambiguous.
fn label_end(raw: &str) -> Option<usize> {
    let mut depth = 0;
    let mut escaped = false;
    let mut code_ticks = 0;
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
        } else if byte == b'\\' && code_ticks == 0 {
            escaped = true;
        } else if byte == b'`' {
            let width = bytes[index..].iter().take_while(|&&b| b == b'`').count();
            if code_ticks == 0 {
                code_ticks = width;
            } else if code_ticks == width {
                code_ticks = 0;
            }
            index += width;
            continue;
        } else if code_ticks == 0 {
            if byte == b'[' {
                depth += 1;
            }
            if byte == b']' {
                depth -= 1;
                if depth == 0 {
                    return (bytes.get(index + 1) == Some(&b'(')).then_some(index);
                }
            }
        }
        index += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slice(text: &str, range: Range<i32>) -> String {
        text.chars()
            .skip(range.start as usize)
            .take((range.end - range.start) as usize)
            .collect()
    }

    #[test]
    fn fmt_001_source_ranges_and_unicode() {
        let text = "# 제목 📝\n## 소제목\n**굵게** *기울임* ~~취소~~\n- 항목\n3. 순서\n- [ ] 할 일\n- [x] 완료\n[한글😀](https://example.com?a=1&b=2)";
        let doc = parse(text);
        for style in [
            Style::Heading1,
            Style::Heading2,
            Style::Bold,
            Style::Italic,
            Style::Strike,
            Style::Task,
            Style::Link,
        ] {
            assert!(
                doc.spans.iter().any(|s| s.style == style),
                "missing {style:?}"
            );
        }
        assert_eq!(slice(text, doc.links[0].range.clone()), "한글😀");
        assert_eq!(slice(text, doc.tasks[0].range.clone()), "[ ]");
        assert_eq!(slice(text, doc.tasks[1].range.clone()), "[x]");
        assert!(!doc.tasks[0].checked);
        assert!(doc.tasks[1].checked);
        for m in doc.markers {
            assert!(!slice(text, m.range).is_empty());
        }
    }

    #[test]
    fn fmt_001_incomplete_escaped_and_unsupported_stay_literal() {
        for text in [
            "**unfinished",
            "[unfinished](",
            "\\*escaped*",
            "`**code**`",
            "```\n**code**\n```",
            "<b>raw</b>",
            "### outside scope",
            "plain & < >",
            "![**image alt**](https://example.com/image.png)",
        ] {
            let doc = parse(text);
            assert!(doc.markers.is_empty(), "unexpected hiding: {text}");
        }
        let doc = parse("**한글 *nested😀* bold**");
        assert_eq!(doc.markers.len(), 4);
        let doc = parse("[a `](` b](https://example.com/a_(b))");
        assert_eq!(doc.links.len(), 1);
        assert_eq!(doc.links[0].range, 1..9);
    }

    #[test]
    fn fmt_001_link_protocols() {
        for url in [
            "https://example.com",
            "HTTP://example.com",
            "mailto:a@example.test",
        ] {
            assert!(safe_link(url));
        }
        for url in [
            "javascript:alert(1)",
            "file:///tmp/test",
            "data:text/html,test",
            "https://example.com\n",
        ] {
            assert!(!safe_link(url));
        }
        assert!(parse("[bad](javascript:alert) [file](file:///tmp/test)")
            .links
            .is_empty());
    }
}
