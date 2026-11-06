//! 한 줄 전체가 첨부 이미지 참조인 줄(DATA-003·FMT-004). 문장 중간의 이미지는 원문으로 둔다.
use super::{Image, IMAGE_SCHEME, IMAGE_WIDTH};
use std::ops::Range;

/// `![설명|너비](asset:<해시>)`의 설명과 너비. 너비가 범위 밖이면 없는 것으로 본다.
fn split_alt(label: &str) -> (String, Option<i32>) {
    match label.rsplit_once('|') {
        Some((alt, width)) => match width.trim().parse::<i32>() {
            Ok(width) => (
                alt.trim().to_owned(),
                IMAGE_WIDTH.contains(&width).then_some(width),
            ),
            Err(_) => (label.trim().to_owned(), None),
        },
        None => (label.trim().to_owned(), None),
    }
}

/// pulldown이 이미지로 읽은 범위가 그 줄 전체일 때만 이미지 줄로 본다.
pub(in crate::presentation::markdown) fn block_image(
    text: &str,
    range: Range<usize>,
    chars: &impl Fn(Range<usize>) -> Range<i32>,
) -> Option<Image> {
    let line_start = text[..range.start].rfind('\n').map_or(0, |at| at + 1);
    let line_end = text[range.end..]
        .find('\n')
        .map_or(text.len(), |at| range.end + at);
    let line = &text[line_start..line_end];
    let raw = &text[range.clone()];
    if line.trim() != raw {
        return None;
    }
    let close = raw.rfind("](")?;
    let target = raw[close + 2..raw.len().checked_sub(1)?].trim();
    if !target.starts_with(IMAGE_SCHEME) || !raw.starts_with("![") || !raw.ends_with(')') {
        return None;
    }
    let (alt, width) = split_alt(&raw[2..close]);
    Some(Image {
        line: chars(line_start..line_end),
        alt,
        width,
        target: target.to_owned(),
    })
}
