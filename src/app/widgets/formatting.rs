use gtk::{gdk, prelude::*, TextView};

pub(super) fn handle(view: &TextView, key: gdk::Key, modifiers: gdk::ModifierType) -> bool {
    let mods = modifiers
        & (gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SHIFT_MASK
            | gdk::ModifierType::ALT_MASK);
    let ctrl = gdk::ModifierType::CONTROL_MASK;
    let shift = ctrl | gdk::ModifierType::SHIFT_MASK;
    let key = key.to_lower();
    let (prefix, suffix, line) = match (mods, key) {
        (m, gdk::Key::b) if m == ctrl => ("**", "**", false),
        (m, gdk::Key::i) if m == ctrl => ("*", "*", false),
        (m, gdk::Key::x) if m == shift => ("~~", "~~", false),
        (m, gdk::Key::k) if m == ctrl => ("[", "](https://)", false),
        (m, gdk::Key::_1) if m == ctrl => ("# ", "", true),
        (m, gdk::Key::_2) if m == ctrl => ("## ", "", true),
        (m, gdk::Key::_7 | gdk::Key::ampersand) if m == shift => ("1. ", "", true),
        (m, gdk::Key::_8 | gdk::Key::asterisk) if m == shift => ("- ", "", true),
        (m, gdk::Key::_9 | gdk::Key::parenleft) if m == shift => ("- [ ] ", "", true),
        _ => return false,
    };
    if !view.is_editable() {
        return true;
    }
    let buffer = view.buffer();
    let had_selection = buffer.has_selection();
    let (mut start, mut end) = buffer.selection_bounds().unwrap_or_else(|| {
        let cursor = buffer.iter_at_mark(&buffer.get_insert());
        (cursor, cursor)
    });
    if !line && prefix == suffix {
        let style = match prefix {
            "**" => crate::presentation::markdown::Style::Bold,
            "*" => crate::presentation::markdown::Style::Italic,
            _ => crate::presentation::markdown::Style::Strike,
        };
        let width = prefix.chars().count() as i32;
        let full = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        if crate::presentation::markdown::parse(&full)
            .spans
            .iter()
            .any(|span| {
                span.style == style && span.range == (start.offset() - width..end.offset() + width)
            })
        {
            start = buffer.iter_at_offset(start.offset() - width);
            end = buffer.iter_at_offset(end.offset() + width);
        }
    }
    if line {
        start.set_line_offset(0);
        if end.line_offset() == 0 && end.line() > start.line() {
            end.backward_char();
        }
        end.forward_to_line_end();
    }
    let text = buffer.text(&start, &end, true);
    let offset = start.offset();
    let (replacement, mut selection) = transform(&text, prefix, suffix, line);
    if line && !had_selection {
        let end = replacement.chars().count() as i32;
        selection = (end, end);
    }
    buffer.begin_user_action();
    buffer.delete(&mut start, &mut end);
    buffer.insert(&mut start, &replacement);
    buffer.select_range(
        &buffer.iter_at_offset(offset + selection.1),
        &buffer.iter_at_offset(offset + selection.0),
    );
    buffer.end_user_action();
    true
}

fn transform(text: &str, prefix: &str, suffix: &str, line: bool) -> (String, (i32, i32)) {
    if line {
        let remove = !text.is_empty() && text.lines().all(|line| line.starts_with(prefix));
        let result = text
            .split('\n')
            .map(|line| {
                if remove {
                    line.strip_prefix(prefix).unwrap_or(line).to_owned()
                } else {
                    let plain = ["- [ ] ", "- [x] ", "## ", "# ", "- ", "1. "]
                        .iter()
                        .find_map(|old| line.strip_prefix(old))
                        .unwrap_or(line);
                    format!("{prefix}{plain}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let length = result.chars().count() as i32;
        return (result, (0, length));
    }
    if !text.is_empty()
        && text.len() >= prefix.len() + suffix.len()
        && text.starts_with(prefix)
        && text.ends_with(suffix)
    {
        let plain = &text[prefix.len()..text.len() - suffix.len()];
        return (plain.to_owned(), (0, plain.chars().count() as i32));
    }
    let start = prefix.chars().count() as i32;
    let end = start + text.chars().count() as i32;
    let selection = if prefix == "[" {
        (end + 2, end + 10)
    } else {
        (start, end)
    };
    (format!("{prefix}{text}{suffix}"), selection)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fmt_002_formats_unicode_and_multiline_selections() {
        assert_eq!(
            transform("한글😀", "**", "**", false),
            ("**한글😀**".into(), (2, 5))
        );
        assert_eq!(transform("**한글**", "**", "**", false).0, "한글");
        assert_eq!(transform("", "**", "**", false), ("****".into(), (2, 2)));
        assert_eq!(
            transform("첫 줄\n둘째", "- [ ] ", "", true).0,
            "- [ ] 첫 줄\n- [ ] 둘째"
        );
        assert_eq!(transform("## 제목", "# ", "", true).0, "# 제목");
        assert_eq!(transform("", "- ", "", true).0, "- ");
        assert_eq!(
            transform("- 첫 줄\n- 둘째", "- ", "", true).0,
            "첫 줄\n둘째"
        );
        assert_eq!(
            transform("링크", "[", "](https://)", false),
            ("[링크](https://)".into(), (5, 13))
        );
    }
}
