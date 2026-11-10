use super::*;

/// Positions that GTK keeps in its buffer but that are absent from the Markdown source.
/// Child anchors and restored U+FFFC placeholders share this single offset map.
pub(in crate::app) struct AnchorMap {
    entries: Vec<Entry>,
}

struct Entry {
    source: i32,
    buffer: i32,
    anchor: Option<gtk::TextChildAnchor>,
}

impl AnchorMap {
    pub(in crate::app) fn read(buffer: &TextBuffer) -> Self {
        let mut entries = Vec::new();
        let mut iter = buffer.start_iter();
        let mut source = 0;
        loop {
            if let Some(anchor) = iter.child_anchor() {
                entries.push(Entry {
                    source,
                    buffer: iter.offset(),
                    anchor: Some(anchor),
                });
            } else if iter.char() == '\u{fffc}' {
                entries.push(Entry {
                    source,
                    buffer: iter.offset(),
                    anchor: None,
                });
            } else {
                source += 1;
            }
            if !iter.forward_char() {
                break;
            }
        }
        Self { entries }
    }

    /// Buffer position of a source boundary, after all hidden entries at that boundary.
    pub(in crate::app) fn to_buffer(&self, source: i32) -> i32 {
        source
            + self
                .entries
                .iter()
                .filter(|entry| entry.source <= source)
                .count() as i32
    }

    /// Buffer position before child anchors at a source boundary, for edits that remove that row.
    pub(in crate::app) fn to_buffer_before_anchor(&self, source: i32) -> i32 {
        source
            + self
                .entries
                .iter()
                .filter(|entry| {
                    entry.source < source || (entry.source == source && entry.anchor.is_none())
                })
                .count() as i32
    }

    pub(in crate::app) fn to_source(&self, buffer: i32) -> i32 {
        buffer
            - self
                .entries
                .iter()
                .filter(|entry| entry.buffer < buffer)
                .count() as i32
    }

    pub(super) fn children(&self) -> impl Iterator<Item = (i32, i32, &gtk::TextChildAnchor)> {
        self.entries.iter().filter_map(|entry| {
            entry
                .anchor
                .as_ref()
                .map(|anchor| (entry.source, entry.buffer, anchor))
        })
    }

    pub(super) fn placeholders(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.entries
            .iter()
            .filter(|entry| entry.anchor.is_none())
            .map(|entry| (entry.source, entry.buffer))
    }

    /// Shift source ranges to the buffer once, with start positions after anchors at that point.
    pub(super) fn map_document(&self, document: &mut Document) {
        fn map(range: &mut Range<i32>, entries: &[Entry], after_anchors: bool) {
            range.start += entries
                .iter()
                .filter(|entry| {
                    entry.source < range.start
                        || (entry.source == range.start
                            && (after_anchors || entry.anchor.is_none()))
                })
                .count() as i32;
            range.end += entries
                .iter()
                .filter(|entry| entry.source < range.end)
                .count() as i32;
        }

        for span in &mut document.spans {
            map(&mut span.range, &self.entries, true);
        }
        for marker in &mut document.markers {
            map(&mut marker.range, &self.entries, true);
            map(&mut marker.owner, &self.entries, true);
        }
        for link in &mut document.links {
            map(&mut link.range, &self.entries, true);
            map(&mut link.owner, &self.entries, true);
            map(&mut link.url_range, &self.entries, true);
        }
        for task in &mut document.tasks {
            map(&mut task.range, &self.entries, true);
        }
        for item in &mut document.items {
            map(&mut item.marker, &self.entries, true);
            map(&mut item.content, &self.entries, true);
        }
        for image in &mut document.images {
            map(&mut image.line, &self.entries, false);
        }
    }
}

pub(super) fn plain_text(
    buffer: &TextBuffer,
    start: &gtk::TextIter,
    end: &gtk::TextIter,
) -> String {
    let mut text = String::new();
    let mut iter = buffer.iter_at_offset(start.offset());
    while iter.offset() < end.offset() {
        if iter.child_anchor().is_none() && iter.char() != '\u{fffc}' {
            text.push(iter.char());
        }
        if !iter.forward_char() {
            break;
        }
    }
    text
}
