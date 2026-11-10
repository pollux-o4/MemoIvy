//! 카드·패널이 함께 쓰는 구조화된 미리보기. 체크·기호·번호를 잃지 않고 읽기 전용으로 그린다.
//! 일반 문단 메모는 행 하나뿐이라 카드 수가 많아도 위젯이 크게 늘지 않는다.
use crate::presentation::markdown::{LineMarker, PreviewLine, PreviewRow, TaskState, BULLETS};
use gtk::{pango, prelude::*, Box as GtkBox, Label, Orientation};
use std::cell::RefCell;

const DEPTH_INDENT: i32 = 12;
const MAX_DEPTH: usize = 2;

/// 표식 칸. 체크는 CSS 상자, 기호·번호는 글자 하나로 그린다.
pub(in crate::app::canvas) fn marker_label() -> Label {
    let label = Label::new(None);
    label.add_css_class("line-marker");
    label.set_valign(gtk::Align::Start);
    label.set_visible(false);
    label
}

pub(in crate::app::canvas) fn set_marker(label: &Label, marker: Option<&LineMarker>, depth: usize) {
    for class in ["task", "checked", "doing", "bullet", "number"] {
        label.remove_css_class(class);
    }
    label.set_valign(if matches!(marker, Some(LineMarker::Number(_))) {
        gtk::Align::Baseline
    } else {
        gtk::Align::Start
    });
    label.set_visible(marker.is_some());
    match marker {
        Some(LineMarker::Task(state)) => {
            label.add_css_class("task");
            match state {
                TaskState::Done => label.add_css_class("checked"),
                TaskState::Doing => label.add_css_class("doing"),
                TaskState::Open => {}
            }
            label.set_text(if *state == TaskState::Done { "✓" } else { "" });
        }
        Some(LineMarker::Bullet) => {
            label.add_css_class("bullet");
            label.set_text(BULLETS[depth.min(BULLETS.len() - 1)]);
        }
        Some(LineMarker::Number(number)) => {
            label.add_css_class("number");
            label.set_text(number);
        }
        None => label.set_text(""),
    }
}

struct Row {
    root: GtkBox,
    marker: Label,
    text: Label,
}

impl Row {
    fn new() -> Self {
        let root = GtkBox::new(Orientation::Horizontal, 6);
        let marker = marker_label();
        let text = Label::new(None);
        text.set_xalign(0.);
        text.set_yalign(0.);
        text.set_valign(gtk::Align::Baseline);
        text.set_wrap_mode(pango::WrapMode::WordChar);
        text.set_ellipsize(pango::EllipsizeMode::End);
        text.set_max_width_chars(1);
        text.set_hexpand(true);
        root.append(&marker);
        root.append(&text);
        Self { root, marker, text }
    }

    fn set(&self, row: &PreviewRow) {
        let PreviewLine {
            marker,
            depth,
            continuation,
            text,
        } = &row.line;
        self.root
            .set_margin_start(DEPTH_INDENT * (*depth).min(MAX_DEPTH) as i32);
        set_marker(&self.marker, marker.as_ref(), *depth);
        self.marker.set_opacity(if *continuation { 0. } else { 1. });
        self.text.set_text(text);
        // 한 줄이면 줄바꿈을 꺼야 Label의 높이 측정이 어긋나지 않는다.
        self.text.set_wrap(row.max_lines > 1);
        self.text.set_lines(row.max_lines);
        if matches!(marker, Some(LineMarker::Task(_))) {
            self.marker.set_valign(gtk::Align::Baseline);
            let anchor = crate::presentation::typography::line_anchor(
                &self.text.layout(),
                &self.text.create_pango_layout(Some("x")),
                None,
            );
            let (ink, _) = self.marker.layout().extents();
            let scale = f64::from(pango::SCALE);
            let marker_ink_center = f64::from(ink.y() + ink.height() / 2) / scale;
            let marker_baseline = f64::from(self.marker.layout().baseline()) / scale;
            let margin = (marker_baseline - marker_ink_center - anchor.x_center)
                .round()
                .max(0.) as i32;
            self.marker.set_margin_top(margin);
        } else if marker.is_some() && !continuation {
            self.marker.set_valign(gtk::Align::Start);
            let anchor = crate::presentation::typography::line_anchor(
                &self.text.layout(),
                &self.text.create_pango_layout(Some("x")),
                None,
            );
            let ink = self.marker.layout().extents().0;
            let marker_center = f64::from(ink.y() + ink.height() / 2) / f64::from(pango::SCALE);
            let x_center = anchor.line_top + anchor.baseline - anchor.x_center;
            self.marker
                .set_margin_top((x_center - marker_center).round() as i32);
        } else {
            self.marker.set_margin_top(0);
        }
        if matches!(marker, Some(LineMarker::Task(TaskState::Done))) {
            self.text.add_css_class("line-done");
        } else {
            self.text.remove_css_class("line-done");
        }
        self.root.set_visible(true);
    }
}

pub(in crate::app::canvas) struct Preview {
    pub root: GtkBox,
    rows: RefCell<Vec<Row>>,
}

impl Preview {
    pub fn new() -> Self {
        let root = GtkBox::new(Orientation::Vertical, 3);
        root.add_css_class("card-preview");
        Self {
            root,
            rows: RefCell::new(Vec::new()),
        }
    }

    pub fn set(&self, wanted: &[PreviewRow]) {
        let mut rows = self.rows.borrow_mut();
        while rows.len() < wanted.len() {
            let row = Row::new();
            self.root.append(&row.root);
            rows.push(row);
        }
        for (row, line) in rows.iter().zip(wanted) {
            row.set(line);
        }
        for row in rows.iter().skip(wanted.len()) {
            row.root.set_visible(false);
        }
    }
}

#[cfg(test)]
impl Preview {
    /// 보이는 행의 (표식 글자, 본문).
    pub fn shown_rows(&self) -> Vec<(String, String)> {
        self.rows
            .borrow()
            .iter()
            .filter(|row| row.root.get_visible())
            .map(|row| (row.marker.text().to_string(), row.text.text().to_string()))
            .collect()
    }

    pub(in crate::app) fn ink_test_rows(&self) -> Vec<(Label, Label)> {
        self.rows
            .borrow()
            .iter()
            .filter(|row| row.root.get_visible())
            .map(|row| (row.marker.clone(), row.text.clone()))
            .collect()
    }
}
