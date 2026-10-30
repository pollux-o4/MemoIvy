use super::{Document, Link, LinkInfo, Span, Style};
use std::ops::Range;

pub(crate) fn safe_link(url: &str) -> bool {
    !url.chars().any(char::is_control)
        && ["https://", "http://", "mailto:"]
            .iter()
            .any(|prefix| url.to_ascii_lowercase().starts_with(prefix))
}

/// http(s) 주소 문자열을 링크로 본다. 한글 조사가 바로 붙는 일이 많아 비ASCII에서 끝낸다.
pub(super) fn bare_links(
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
pub(super) fn label_end(raw: &str) -> Option<usize> {
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
