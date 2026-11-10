pub(crate) const EDITOR_LINE_HEIGHT_CSS: &str = "1.5em";

pub(crate) const EDITOR_H1: f64 = 1.6;
pub(crate) const EDITOR_H2: f64 = 1.3;
pub(crate) const EDITOR_MEDIA_QUIET: f64 = 0.7;
pub(crate) const EDITOR_MEDIA_SOURCE: f64 = 0.85;

/// Pango's first-line anchor, measured in pixels relative to its layout origin.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LineAnchor {
    pub baseline: f64,
    pub x_center: f64,
    pub ascent: f64,
    pub line_top: f64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LineBox {
    pub top: i32,
    pub height: i32,
    pub above: i32,
    pub below: i32,
}

/// Measure the first line and its x-height center from Pango's actual ink metrics.
pub(crate) fn line_anchor(
    layout: &gtk::pango::Layout,
    x_layout: &gtk::pango::Layout,
    line_box: Option<LineBox>,
) -> LineAnchor {
    let scale = f64::from(gtk::pango::SCALE);
    let logical = layout.extents().1;
    let (x_ink, _) = x_layout.extents();
    let context = layout.context();
    let description = layout
        .font_description()
        .or_else(|| context.font_description());
    let language = context.language();
    let metrics = context.metrics(description.as_ref(), language.as_ref());
    let ascent = f64::from(metrics.ascent()) / scale;
    let baseline = if let Some(line_box) = line_box {
        let height = line_box.height * gtk::pango::SCALE;
        let spacing = (line_box.above + line_box.below) * gtk::pango::SCALE;
        let font_height = metrics.height().max(metrics.ascent() + metrics.descent());
        let leading = (height - spacing - font_height).max(0);
        (f64::from(line_box.above * gtk::pango::SCALE + leading / 2 + metrics.ascent()) / scale)
            .round()
    } else {
        f64::from(x_layout.baseline()) / scale
    };
    // Keep x-height relative to the x layout's own baseline; subtracting ink from the
    // measured line baseline would cancel the line-font metric entirely.
    let x_center = f64::from(x_layout.baseline() - x_ink.y() - x_ink.height() / 2) / scale;
    LineAnchor {
        baseline,
        x_center,
        ascent,
        line_top: line_box.map_or(f64::from(logical.y()) / scale, |line| f64::from(line.top)),
    }
}
