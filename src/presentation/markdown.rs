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
    TaskDone,
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Link {
    /// 눈에 보이는 이름. 숨긴 문법을 뺀 범위다.
    pub range: Range<i32>,
    /// 문법까지 포함한 전체. 커서 줄에서 드러난 원문도 같은 링크로 본다.
    pub owner: Range<i32>,
    /// 원문에서 주소만 있는 범위.
    pub url_range: Range<i32>,
    pub url: String,
}

#[derive(Debug)]
pub(crate) struct Task {
    pub range: Range<i32>,
    pub checked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    Bullet,
    Ordered,
    Task(bool),
}

/// 원문 첫 줄 기준의 목록 항목. 편집기는 이 범위로 표식 칸과 내어쓰기를 만든다.
#[derive(Debug)]
pub(crate) struct ListItem {
    /// 줄 시작부터 본문 직전까지. 들여쓰기·기호·체크 문법과 뒤 공백을 포함한다.
    pub marker: Range<i32>,
    /// 본문부터 줄 끝까지. 비어 있으면 표식을 숨기지 않는다.
    pub content: Range<i32>,
    pub depth: usize,
    pub kind: ListKind,
}

#[derive(Default, Debug)]
pub(crate) struct Document {
    pub spans: Vec<Span>,
    pub markers: Vec<Marker>,
    pub links: Vec<Link>,
    pub tasks: Vec<Task>,
    pub items: Vec<ListItem>,
}

pub(crate) fn safe_link(url: &str) -> bool {
    !url.chars().any(char::is_control)
        && ["https://", "http://", "mailto:"]
            .iter()
            .any(|prefix| url.to_ascii_lowercase().starts_with(prefix))
}

/// GTK offsets count characters; the parser reports UTF-8 byte ranges.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Summary {
    pub title: String,
    pub preview: String,
    /// 제목 줄이 목록 항목이면 그 표식. 카드가 제목 앞에 같이 보여 준다.
    pub title_marker: Option<LineMarker>,
    /// 제목 다음 줄들. 표식을 잃지 않는 구조화된 미리보기다.
    pub lines: Vec<PreviewLine>,
    /// `lines`에 담지 않은 나머지 내용 줄 수.
    pub remaining: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LineMarker {
    Bullet,
    Number(String),
    Task(bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviewLine {
    pub marker: Option<LineMarker>,
    pub depth: usize,
    pub text: String,
}

/// 줄 예산에 맞춘 표시 행. 표식 없는 연속 문단은 한 행으로 합쳐 줄바꿈한다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PreviewRow {
    pub line: PreviewLine,
    pub max_lines: i32,
}

const PREVIEW_CHARS: usize = 240;
const PREVIEW_LINES: usize = 12;

/// 카드에 보일 요약. 서식 기호를 뺀 첫 줄을 제목, 나머지를 미리보기로 쓴다.
/// 카드에 보일 만큼만 읽어 긴 문서에서도 비용이 일정하다.
pub(crate) fn summary(text: &str) -> Summary {
    let mut summary = Summary::default();
    let mut has_title = false;
    let mut fence: Option<&str> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let opener = ["```", "~~~"]
            .into_iter()
            .find(|f| trimmed.starts_with(f) && !trimmed.trim_start_matches('`').contains('`'));
        let literal = match (fence, opener) {
            (Some(open), Some(f)) if f == open => {
                fence = None;
                continue;
            }
            (Some(_), _) => true,
            (None, Some(f)) => {
                fence = Some(f);
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
        let (marker, depth, rest) = if literal {
            (None, 0, line)
        } else {
            classify(line)
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
            });
        } else {
            summary.remaining += 1;
        }
    }
    summary
}

/// 목록 표식과 깊이만 읽는다. 카드 요약은 줄 단위라 문서 전체를 파싱하지 않는다.
fn classify(line: &str) -> (Option<LineMarker>, usize, &str) {
    let Some(mark) = line_mark(line) else {
        return (None, 0, line);
    };
    let marker = match mark.kind {
        ListKind::Bullet => LineMarker::Bullet,
        ListKind::Task(checked) => LineMarker::Task(checked),
        ListKind::Ordered => {
            let end = line[mark.start..]
                .find(['.', ')'])
                .map_or(mark.start, |at| mark.start + at + 1);
            LineMarker::Number(line[mark.start..end].to_owned())
        }
    };
    (Some(marker), mark.columns / 2, &line[mark.body..])
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

pub(crate) fn task_progress(text: &str) -> (usize, usize) {
    let tasks = parse(text).tasks;
    (
        tasks.iter().filter(|task| task.checked).count(),
        tasks.len(),
    )
}

/// 서식 문법을 뺀 글자. 링크 이름처럼 원문 조각을 사람이 읽는 글자로 바꿀 때 쓴다.
pub(crate) fn plain_line(line: &str) -> String {
    let source = line.trim();
    let mut out = String::new();
    for event in Parser::new_ext(
        source,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    ) {
        match event {
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
    let mut parsed_tasks: Vec<(Range<i32>, bool)> = Vec::new();
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
            Event::TaskListMarker(checked) => {
                // 탭 뒤의 체크는 범위가 탭부터 시작한다. 토글이 `[` 다음 글자를 바꾸므로 잘라 낸다.
                let box_start = range.start + (raw.len() - raw.trim_start().len());
                parsed_tasks.push((chars(box_start..range.end), checked));
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
    bare_links(text, &literal, &chars, &mut doc);
    scan_items(text, &literal, &chars, &mut doc);
    // 인용문 안이나 번호와 함께 쓴 체크는 원문으로 남기고 토글만 지원한다.
    for (range, checked) in parsed_tasks {
        if !doc
            .items
            .iter()
            .any(|item| item.marker.contains(&range.start))
        {
            doc.spans.push(Span {
                range: range.clone(),
                style: Style::Task,
            });
            doc.tasks.push(Task { range, checked });
        }
    }
    doc.tasks.sort_by_key(|task| task.range.start);
    doc
}

struct LineMark {
    /// 들여쓰기 뒤 표식 시작(바이트).
    start: usize,
    /// 본문 시작(바이트).
    body: usize,
    /// 들여쓰기 칸 수.
    columns: usize,
    kind: ListKind,
    /// 체크 괄호 범위(바이트).
    check: Option<Range<usize>>,
}

/// 한 줄의 목록 표식. 쓰는 동안 바로 보여야 하므로 문단 뒤나 빈 항목도 줄 단위로 인식한다.
/// 공백까지 쳐야 표식이다. `-5도`나 입력 중인 `-`가 잠깐 기호로 바뀌지 않는다.
fn line_mark(line: &str) -> Option<LineMark> {
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

fn scan_items(
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

/// http(s) 주소 문자열을 링크로 본다. 한글 조사가 바로 붙는 일이 많아 비ASCII에서 끝낸다.
fn bare_links(
    text: &str,
    literal: &[Range<usize>],
    chars: &impl Fn(Range<usize>) -> Range<i32>,
    doc: &mut Document,
) {
    let mut from = 0;
    while let Some(found) = text[from..].find("http").map(|at| from + at) {
        from = found + 4;
        let rest = &text[found..];
        let lower = rest.get(..8).unwrap_or(rest).to_ascii_lowercase();
        let scheme = if lower.starts_with("https://") {
            8
        } else if lower.starts_with("http://") {
            7
        } else {
            continue;
        };
        if text[..found]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
            || literal.iter().any(|skip| skip.contains(&found))
        {
            continue;
        }
        let mut length = rest
            .find(|c: char| {
                !c.is_ascii()
                    || c.is_ascii_whitespace()
                    || c.is_ascii_control()
                    || "<>\"`".contains(c)
            })
            .unwrap_or(rest.len());
        loop {
            let url = &rest[..length];
            let Some(last) = url.chars().next_back() else {
                break;
            };
            let unbalanced = last == ')' && url.matches(')').count() > url.matches('(').count();
            if unbalanced || ".,;:!?*_~'".contains(last) {
                length -= last.len_utf8();
            } else {
                break;
            }
        }
        let url = &rest[..length];
        if !rest[scheme..length]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
            || !safe_link(url)
        {
            continue;
        }
        let range = found..found + length;
        // 주소 글자는 서식 문법이 아니다.
        let inside = chars(range.clone());
        doc.markers.retain(|m| !inside.contains(&m.range.start));
        doc.spans
            .retain(|s| !(inside.contains(&s.range.start) || inside.contains(&(s.range.end - 1))));
        doc.spans.push(Span {
            range: chars(range.clone()),
            style: Style::Link,
        });
        doc.links.push(Link {
            range: chars(range.clone()),
            owner: chars(range.clone()),
            url_range: chars(range),
            url: url.to_owned(),
        });
        from = found + length;
    }
    doc.links.sort_by_key(|link| link.owner.start);
}

/// 링크 미리보기에 보일 정보. 이름이 아니라 실제 목적지를 드러내는 것이 목적이다.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct LinkInfo {
    /// 강조할 목적지. 웹은 호스트, 메일은 주소다.
    pub host: String,
    /// 호스트 뒤에 이어지는 경로·쿼리.
    pub rest: String,
    pub mail: bool,
    pub warning: Option<&'static str>,
}

/// 보이지 않거나 표시 방향을 바꾸는 문자는 숨기지 않고 대체 문자로 드러낸다.
fn visible_url(url: &str) -> String {
    url.chars()
        .map(|c| match c {
            '\u{00AD}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{FEFF}' => '\u{FFFD}',
            c => c,
        })
        .collect()
}

fn bare_host(host: &str) -> &str {
    host.strip_prefix("www.").unwrap_or(host)
}

const WEB_TLDS: [&str; 24] = [
    "com", "net", "org", "kr", "io", "co", "ai", "app", "dev", "me", "info", "biz", "edu", "gov",
    "xyz", "shop", "site", "jp", "cn", "uk", "de", "fr", "us", "tv",
];

/// 이름이 도메인처럼 보이면 그 호스트. 다른 곳으로 가는 위장 링크를 가려낸다.
fn label_host(label: &str) -> Option<String> {
    let label = label.trim().to_ascii_lowercase();
    let body = label
        .strip_prefix("https://")
        .or_else(|| label.strip_prefix("http://"))
        .unwrap_or(&label);
    // Node.js·report.pdf 같은 이름은 도메인이 아니다. 주소 표기가 분명하거나 흔한 웹 도메인일 때만 본다.
    let explicit = body.len() < label.len() || body.starts_with("www.");
    let host = body.split(['/', '?', '#', ':']).next()?;
    let domain = host.contains('.')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && host
            .rsplit('.')
            .next()
            .is_some_and(|tld| explicit || WEB_TLDS.contains(&tld));
    domain.then(|| host.to_owned())
}

pub(crate) fn link_info(url: &str, label: &str) -> LinkInfo {
    let url = visible_url(url);
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("mailto:") {
        return LinkInfo {
            host: url["mailto:".len()..]
                .split('?')
                .next()
                .unwrap_or("")
                .to_owned(),
            rest: String::new(),
            mail: true,
            warning: None,
        };
    }
    let secure = lower.starts_with("https://");
    let body = url.split_once("://").map_or(url.as_str(), |(_, body)| body);
    // 브라우저는 역슬래시도 경로 구분자로 읽는다. 호스트 위장을 막으려고 같이 끊는다.
    let cut = body.find(['/', '?', '#', '\\']).unwrap_or(body.len());
    let (authority, rest) = body.split_at(cut);
    let (userinfo, host) = authority
        .rsplit_once('@')
        .map_or((false, authority), |(_, host)| (true, host));
    let host = host.to_ascii_lowercase();
    let spoofed = label_host(label).is_some_and(|shown| {
        let (shown, real) = (bare_host(&shown), bare_host(&host));
        let real = real.split(':').next().unwrap_or(real);
        shown != real && !real.ends_with(&format!(".{shown}"))
    });
    let warning = if host.is_empty() {
        Some("주소에서 목적지를 확인할 수 없어요")
    } else if userinfo {
        Some("주소에 사용자 정보가 포함돼 실제 목적지가 다를 수 있어요")
    } else if spoofed {
        Some("표시된 이름과 실제 주소가 달라요")
    } else if !host.is_ascii()
        || host.split('.').any(|part| part.starts_with("xn--"))
        || url.contains('\u{FFFD}')
    {
        Some("주소에 비슷해 보이는 문자가 있을 수 있어요")
    } else if !secure {
        Some("암호화되지 않은 연결이에요")
    } else {
        None
    };
    LinkInfo {
        host,
        rest: rest.trim_end_matches('/').to_owned(),
        mail: false,
        warning,
    }
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

    #[test]
    fn canvas_001_summary_hides_markup_and_counts_tasks() {
        let summary =
            summary("\n# 출시 **계획**\n- [x] 범위\n- [ ] [노트](https://e.com)\n---\n`코드`");
        assert_eq!(summary.title, "출시 계획");
        assert_eq!(summary.preview, "범위 노트 코드");
        assert_eq!(task_progress("- [x] 범위\n- [ ] 노트\n일반"), (1, 2));
        assert_eq!(super::summary("").title, "");
        assert_eq!(super::summary("<b>raw</b>").title, "raw");
    }

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
    fn fmt_003_list_items_keep_marker_and_content_ranges() {
        let text = "- 항목\n  - 안쪽\n\n1. 번호\n12) 둘\n\n- [x] 끝남\n- [ ] 할 일\n- \n> - 인용";
        let doc = parse(text);
        let view = |item: &ListItem| {
            (
                slice(text, item.marker.clone()),
                slice(text, item.content.clone()),
                item.depth,
                item.kind,
            )
        };
        let items: Vec<_> = doc.items.iter().map(view).collect();
        assert_eq!(
            items,
            [
                ("- ".to_owned(), "항목".to_owned(), 0, ListKind::Bullet),
                ("  - ".into(), "안쪽".into(), 1, ListKind::Bullet),
                ("1. ".into(), "번호".into(), 0, ListKind::Ordered),
                ("12) ".into(), "둘".into(), 0, ListKind::Ordered),
                ("- [x] ".into(), "끝남".into(), 0, ListKind::Task(true)),
                ("- [ ] ".into(), "할 일".into(), 0, ListKind::Task(false)),
                ("- ".into(), "".into(), 0, ListKind::Bullet),
            ]
        );
        assert!(doc.markers.is_empty(), "list markers are drawn, not hidden");
        let done: Vec<_> = doc
            .spans
            .iter()
            .filter(|s| s.style == Style::TaskDone)
            .map(|s| slice(text, s.range.clone()))
            .collect();
        assert_eq!(done, ["끝남"]);
        assert!(parse("```\n- 코드\n```").items.is_empty());
        assert!(
            parse("- - -\n* * *").items.is_empty(),
            "rules are not lists"
        );
        // 쓰는 동안: 문단 바로 뒤, 빈 항목, 빈 체크도 그 자리에서 항목이다. 공백 전에는 아니다.
        let kinds = |text: &str| parse(text).items.iter().map(|i| i.kind).collect::<Vec<_>>();
        assert_eq!(kinds("문단\n- "), [ListKind::Bullet]);
        assert_eq!(kinds("문단\n- [ ] "), [ListKind::Task(false)]);
        assert_eq!(kinds("문단\n2. "), [ListKind::Ordered]);
        assert!(kinds("문단\n-").is_empty());
        assert!(kinds("-5도").is_empty());
        assert_eq!(kinds("- [ ]"), [ListKind::Bullet]);
        assert_eq!(parse("문단\n- [ ] ").tasks.len(), 1);
        // 네 칸 들여쓰기: 기호가 본문으로 새지 않는다.
        let text = "- 바깥\n    - 안쪽\n1. 하나\n    1. 둘";
        let items: Vec<_> = parse(text)
            .items
            .iter()
            .map(|i| (slice(text, i.content.clone()), i.kind))
            .collect();
        assert_eq!(
            items,
            [
                ("바깥".to_owned(), ListKind::Bullet),
                ("안쪽".into(), ListKind::Bullet),
                ("하나".into(), ListKind::Ordered),
                ("둘".into(), ListKind::Ordered),
            ]
        );
        // 탭 뒤 체크: 토글 범위는 괄호부터다. 안 그러면 Ctrl+Enter가 원문을 망가뜨린다.
        let text = "  - \t[ ] 탭";
        let doc = parse(text);
        assert_eq!(slice(text, doc.tasks[0].range.clone()), "[ ]");
        assert_eq!(slice(text, doc.items[0].content.clone()), "탭");
        // 중첩 체크는 각자 항목에 붙는다.
        let text = "- [ ] 위\n\t- [x] 아래";
        let doc = parse(text);
        assert_eq!(doc.items[0].kind, ListKind::Task(false));
        assert_eq!(doc.items[1].kind, ListKind::Task(true));
        assert_eq!(slice(text, doc.items[0].content.clone()), "위");
        // 번호 + 체크는 번호를 숨기지 않는다.
        let text = "1. [ ] 일";
        let doc = parse(text);
        assert_eq!(doc.items[0].kind, ListKind::Ordered);
        assert_eq!(slice(text, doc.items[0].marker.clone()), "1. ");
        assert_eq!(doc.tasks.len(), 1);
        // 인용문 안의 체크가 앞 항목을 바꾸면 표식 범위가 여러 줄로 번진다.
        let text = "- 앞 항목\n\n> - [ ] 인용 할 일";
        let doc = parse(text);
        assert_eq!(doc.items.len(), 1);
        assert_eq!(doc.items[0].kind, ListKind::Bullet);
        assert_eq!(slice(text, doc.items[0].marker.clone()), "- ");
        assert_eq!(doc.tasks.len(), 1);
    }

    #[test]
    fn canvas_004_summary_keeps_list_markers_for_cards() {
        let s = summary(
            "- [ ] 머리 할 일\n- [x] 끝남\n  - 안쪽\n3. 셋째\n문단 하나\n문단 둘\n```\n- 코드\n```\n---",
        );
        assert_eq!(s.title, "머리 할 일");
        assert_eq!(s.title_marker, Some(LineMarker::Task(false)));
        let got: Vec<_> = s
            .lines
            .iter()
            .map(|l| (l.marker.clone(), l.depth, l.text.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (Some(LineMarker::Task(true)), 0, "끝남"),
                (Some(LineMarker::Bullet), 1, "안쪽"),
                (Some(LineMarker::Number("3.".into())), 0, "셋째"),
                (None, 0, "문단 하나"),
                (None, 0, "문단 둘"),
                (None, 0, "- 코드"),
            ]
        );
        assert_eq!(s.remaining, 0);
        assert_eq!(
            summary("```한 줄```\n- 항목").lines[0].marker,
            Some(LineMarker::Bullet)
        );
        let many = summary(&format!("제목\n{}", "- 항목\n".repeat(50)));
        assert_eq!(many.lines.len(), 12);
        assert_eq!(many.remaining, 38);
    }

    #[test]
    fn canvas_004_fit_preview_spends_a_line_budget() {
        let line = |marker: Option<LineMarker>, text: &str| PreviewLine {
            marker,
            depth: 0,
            text: text.into(),
        };
        // 문단만 있으면 지금처럼 한 행으로 이어 세 줄까지 쓴다.
        let (rows, more) = fit_preview(&[line(None, "가"), line(None, "나")], 3);
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (rows[0].line.text.as_str(), rows[0].max_lines),
            ("가 나", 3)
        );
        assert_eq!(more, 0);
        // 항목은 한 줄씩, 앞선 문단은 뒤 항목 몫을 남긴다.
        let items = [
            line(None, "설명"),
            line(Some(LineMarker::Bullet), "하나"),
            line(Some(LineMarker::Bullet), "둘"),
            line(Some(LineMarker::Bullet), "셋"),
        ];
        let (rows, more) = fit_preview(&items, 3);
        assert_eq!(
            rows.iter().map(|r| r.max_lines).collect::<Vec<_>>(),
            [1, 1, 1]
        );
        assert_eq!(more, 1);
        assert_eq!(fit_preview(&items[..2], 3).0[0].max_lines, 2);
        assert_eq!(fit_preview(&[], 3), (Vec::new(), 0));
    }

    #[test]
    fn fmt_003_bare_and_angle_urls_become_links() {
        let urls =
            |text: &str| -> Vec<String> { parse(text).links.into_iter().map(|l| l.url).collect() };
        assert_eq!(
            urls("보기 https://a.example/x_y_z."),
            ["https://a.example/x_y_z"]
        );
        assert_eq!(urls("https://a.example입니다"), ["https://a.example"]);
        assert_eq!(
            urls("(https://a.example/b_(c))"),
            ["https://a.example/b_(c)"]
        );
        assert_eq!(urls("**https://a.example**"), ["https://a.example"]);
        let doc = parse("https://a.example/_b_/c");
        assert_eq!(doc.links[0].url, "https://a.example/_b_/c");
        assert!(doc.markers.is_empty(), "URL characters are not syntax");
        assert_eq!(urls("<https://a.example/b>"), ["https://a.example/b"]);
        assert_eq!(
            urls("[이름](https://a.example) https://b.example"),
            ["https://a.example", "https://b.example"]
        );
        for text in [
            "`https://a.example`",
            "```\nhttps://a.example\n```",
            "xhttps://a.example",
            "https://",
            "http 없음",
            "![그림](https://a.example/i.png)",
            "<a href=\"https://a.example\">x</a>",
        ] {
            assert!(urls(text).is_empty(), "{text}");
        }
        let text = "한글 https://a.example 끝";
        let link = &parse(text).links[0];
        assert_eq!(slice(text, link.range.clone()), "https://a.example");
        let text = "[이름](https://a.example \"제목\")";
        let link = &parse(text).links[0];
        assert_eq!(slice(text, link.url_range.clone()), "https://a.example");
        assert_eq!(slice(text, link.range.clone()), "이름");
        assert_eq!(slice(text, link.owner.clone()), text);
    }

    #[test]
    fn fmt_003_link_info_shows_the_real_destination() {
        let info = link_info("https://www.Example.com/a/b/?q=1", "이름");
        assert_eq!(info.host, "www.example.com");
        assert_eq!(info.rest, "/a/b/?q=1");
        assert!(info.warning.is_none());
        assert_eq!(link_info("https://a.example/", "x").rest, "");
        let mail = link_info("mailto:a@example.test?subject=x", "메일");
        assert!(mail.mail);
        assert_eq!(mail.host, "a@example.test");
        assert!(link_info("http://a.example", "x").warning.is_some());
        // 호스트 위장: 사용자 정보, 역슬래시, 이름과 다른 목적지, 방향 제어 문자.
        let info = link_info("https://google.com@evil.example/x", "x");
        assert_eq!(info.host, "evil.example");
        assert!(info.warning.is_some());
        assert_eq!(
            link_info("https://evil.example\\@good.example", "x").host,
            "evil.example"
        );
        assert!(link_info("https://evil.example", "bank.com")
            .warning
            .is_some());
        assert!(link_info("https://evil.example", "https://bank.com/login")
            .warning
            .is_some());
        assert!(link_info("https://docs.bank.com", "bank.com")
            .warning
            .is_none());
        assert!(link_info("https://www.bank.com", "bank.com")
            .warning
            .is_none());
        assert!(link_info("https://bank.com:8443", "bank.com")
            .warning
            .is_none());
        // 파일·제품 이름은 도메인으로 보지 않는다. 경보가 잦으면 아무도 읽지 않는다.
        for label in [
            "README.md",
            "Node.js",
            "report.pdf",
            "v1.2 문서",
            "Socket.IO 문서",
        ] {
            assert!(
                link_info("https://a.example/x", label).warning.is_none(),
                "{label}"
            );
        }
        // 명시적 주소 표기·포트·경로 안의 위장은 놓치지 않는다.
        assert!(link_info("https://evil.example", "bank.com:443")
            .warning
            .is_some());
        assert!(link_info("https://evil.example/bank.com", "bank.com")
            .warning
            .is_some());
        assert!(link_info("https://evil.example", "www.bank.test")
            .warning
            .is_some());
        // 비ASCII·빈 호스트.
        assert!(link_info("https://\u{0430}pple.com", "x").warning.is_some());
        assert!(link_info("https:///evil.example", "x").warning.is_some());
        assert!(link_info("https://", "x").warning.is_some());
        // 서식 문법이 든 이름도 평문으로 판별한다.
        assert_eq!(plain_line("**bank.com**"), "bank.com");
        assert!(link_info("https://a.example", "v1.2 문서")
            .warning
            .is_none());
        assert!(link_info("https://xn--80ak6aa92e.com", "x")
            .warning
            .is_some());
        assert!(link_info("https://a.example/\u{202E}gpj.exe", "x")
            .rest
            .contains('\u{FFFD}'));
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
