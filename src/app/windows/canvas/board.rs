use super::{interaction, motion, CanvasView};
use crate::persistence::canvas::{position, HEIGHT, WIDTH};
use gtk::{cairo, prelude::*};
use std::rc::Rc;

pub(super) fn attach(view: &Rc<CanvasView>) {
    let weak = Rc::downgrade(view);
    view.drawing.set_draw_func(move |_, cr, width, height| {
        let Some(v) = weak.upgrade() else {
            return;
        };
        let dark = crate::presentation::theme::is_dark(&v.window.settings());
        let bg = if dark { 0.115 } else { 0.969 };
        cr.set_source_rgb(bg, bg + 0.005, bg + 0.015);
        let _ = cr.paint();
        let z = v.zoom.get();
        cr.translate(
            -v.scroll.hadjustment().value(),
            -v.scroll.vadjustment().value(),
        );
        cr.scale(z, z);
        cr.set_source_rgba(0.5, 0.55, 0.65, if dark { 0.25 } else { 0.22 });
        // Only paint visible grid dots, even on a large board.
        let x0 = (v.scroll.hadjustment().value() / z / 24.).floor() as i32;
        let y0 = (v.scroll.vadjustment().value() / z / 24.).floor() as i32;
        let x1 =
            ((v.scroll.hadjustment().value() + v.scroll.width() as f64) / z / 24.).ceil() as i32;
        let y1 =
            ((v.scroll.vadjustment().value() + v.scroll.height() as f64) / z / 24.).ceil() as i32;
        for x in x0..=x1 {
            for y in y0..=y1 {
                cr.arc(
                    x as f64 * 24.,
                    y as f64 * 24.,
                    0.85 / z,
                    0.,
                    std::f64::consts::TAU,
                );
                let _ = cr.fill();
            }
        }
        let Some(st) = v.state.upgrade() else {
            return;
        };
        let s = st.borrow();
        let animations = v.window.settings().is_gtk_enable_animations();
        for (i, note) in s.store.notes.iter().enumerate() {
            for id in &note.canvas.next {
                let Some((j, target)) = s.store.notes.iter().enumerate().find(|(_, n)| n.id == *id)
                else {
                    continue;
                };
                let p = path(position(note, i), position(target, j));
                let active = v.pulse.get().filter(|(from, t)| {
                    *from == note.id && t.elapsed().as_secs_f64() < motion::SIGNAL_SECONDS
                });
                let selected =
                    v.selected.get() == Some(note.id) || v.selected.get() == Some(target.id);
                if active.is_some() {
                    cr.set_source_rgb(0.22, 0.48, 0.95);
                } else if selected {
                    cr.set_source_rgba(0.32, 0.46, 0.72, 0.9);
                } else {
                    cr.set_source_rgba(0.48, 0.53, 0.62, 0.65);
                }
                cr.set_line_width(if selected || active.is_some() {
                    2.5
                } else {
                    1.8
                });
                cr.move_to(p[0].0, p[0].1);
                cr.curve_to(p[1].0, p[1].1, p[2].0, p[2].1, p[3].0, p[3].1);
                let _ = cr.stroke();
                arrow(cr, p[2], p[3]);
                if let Some((_, start)) = active {
                    let elapsed = start.elapsed().as_secs_f64();
                    let (t, alpha) = motion::pulse_sample(elapsed, animations);
                    if !animations || elapsed >= motion::TRAVEL_SECONDS {
                        arrival(cr, position(target, j), alpha);
                    }
                    let (x, y) = point(p, t);
                    cr.set_source_rgba(0.22, 0.48, 0.95, 0.14 * alpha);
                    cr.arc(x, y, 12., 0., std::f64::consts::TAU);
                    let _ = cr.fill();
                    cr.set_source_rgba(0.22, 0.48, 0.95, alpha);
                    cr.arc(x, y, 5., 0., std::f64::consts::TAU);
                    let _ = cr.fill();
                }
            }
        }
        let _ = (width, height);
    });
    for adjustment in [view.scroll.hadjustment(), view.scroll.vadjustment()] {
        let weak = view.drawing.downgrade();
        adjustment.connect_value_changed(move |_| {
            if let Some(d) = weak.upgrade() {
                d.queue_draw();
            }
        });
    }
}

pub(super) fn rescale(v: &CanvasView) {
    let Some(st) = v.state.upgrade() else {
        return;
    };
    let s = st.borrow();
    let z = v.zoom.get();
    let (mut width, mut height) = (1400_f64, 900_f64);
    let mut child = v.fixed.first_child();
    for (i, note) in s.store.notes.iter().enumerate() {
        let [x, y] = position(note, i);
        width = width.max(x + WIDTH + 80.);
        height = height.max(y + HEIGHT + 80.);
        if let Some(card) = child {
            child = card.next_sibling();
            place(&v.fixed, &card, x, y, z);
        }
    }
    v.fixed
        .set_size_request((width * z).ceil() as i32, (height * z).ceil() as i32);
    v.scroll
        .hadjustment()
        .set_upper((width * z).max(v.scroll.hadjustment().page_size()));
    v.scroll
        .vadjustment()
        .set_upper((height * z).max(v.scroll.vadjustment().page_size()));
    v.zoom_label.set_text(&format!("{:.0}%", z * 100.));
}

pub(super) fn render(v: &Rc<CanvasView>) {
    let Some(st) = v.state.upgrade() else {
        return;
    };
    let notes = st.borrow().store.notes.clone();
    let focus = gtk::prelude::GtkWindowExt::focus(&v.window);
    let mut focused_id = None;
    if let Some(focus) = focus {
        let mut parent = Some(focus);
        while let Some(widget) = parent {
            if widget.parent().as_ref() == Some(v.fixed.upcast_ref()) {
                focused_id = Some(widget.widget_name().to_string());
                break;
            }
            parent = widget.parent();
        }
    }
    let scroll_position = (
        v.scroll.hadjustment().value(),
        v.scroll.vadjustment().value(),
    );
    if focused_id.is_some() {
        gtk::prelude::GtkWindowExt::set_focus(&v.window, None::<&gtk::Widget>);
    }

    if !notes.iter().any(|n| Some(n.id) == v.selected.get()) {
        v.selected.set(None);
    }
    let mut child = v.fixed.first_child();
    while let Some(card) = child {
        child = card.next_sibling();
        v.fixed.remove(&card);
    }
    let z = v.zoom.get();
    v.zoom_label.set_text(&format!("{:.0}%", z * 100.));
    let edges: usize = notes.iter().map(|n| n.canvas.next.len()).sum();
    v.summary
        .set_text(&format!("메모 {} · 연결 {}", notes.len(), edges));
    let (mut width, mut height) = (1400_f64, 900_f64);
    for (i, note) in notes.iter().enumerate() {
        let [x, y] = position(note, i);
        width = width.max(x + WIDTH + 80.);
        height = height.max(y + HEIGHT + 80.);
        let card = interaction::card(v, note, i);
        v.fixed.put(&card, 0., 0.);
        place(&v.fixed, &card, x, y, z);
        if focused_id.as_deref() == Some(note.id.to_string().as_str()) {
            card.child_focus(gtk::DirectionType::TabForward);
        }
    }
    v.fixed
        .set_size_request((width * z).ceil() as i32, (height * z).ceil() as i32);
    if focused_id.is_some() {
        v.scroll.hadjustment().set_value(scroll_position.0);
        v.scroll.vadjustment().set_value(scroll_position.1);
    }
    v.drawing.queue_draw();
}

pub(super) fn place(fixed: &gtk::Fixed, card: &impl IsA<gtk::Widget>, x: f64, y: f64, z: f64) {
    let transform = gtk::gsk::Transform::new()
        .translate(&gtk::graphene::Point::new((x * z) as f32, (y * z) as f32))
        .scale(z as f32, z as f32);
    fixed.set_child_transform(card, Some(&transform));
}

fn arrival(cr: &cairo::Context, p: [f64; 2], alpha: f64) {
    cr.set_source_rgba(0.32, 0.55, 1., alpha * 0.8);
    cr.set_line_width(3.);
    let (x, y, w, h, r) = (p[0] - 5., p[1] - 5., WIDTH + 10., HEIGHT + 10., 18.);
    let pi = std::f64::consts::PI;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -pi / 2., 0.);
    cr.arc(x + w - r, y + h - r, r, 0., pi / 2.);
    cr.arc(x + r, y + h - r, r, pi / 2., pi);
    cr.arc(x + r, y + r, r, pi, pi * 1.5);
    cr.close_path();
    let _ = cr.stroke();
}

type Curve = [(f64, f64); 4];
// shortcut: 수동 배치용 방향 곡선이라 다른 카드를 자동 회피하지 않음, 복잡한 보드 사용에서 겹침이 반복되면 라우터 검토.
fn path(a: [f64; 2], b: [f64; 2]) -> Curve {
    if (b[0] - a[0]).abs() >= WIDTH + 24. {
        let direction = if b[0] > a[0] { 1. } else { -1. };
        let start = (
            a[0] + if direction > 0. { WIDTH } else { 0. },
            a[1] + HEIGHT / 2.,
        );
        let end = (
            b[0] + if direction > 0. { 0. } else { WIDTH },
            b[1] + HEIGHT / 2.,
        );
        let bend = ((end.0 - start.0).abs() / 2.).max(40.);
        [
            start,
            (start.0 + direction * bend, start.1),
            (end.0 - direction * bend, end.1),
            end,
        ]
    } else {
        let direction = if b[1] >= a[1] { 1. } else { -1. };
        let start = (
            a[0] + WIDTH / 2.,
            a[1] + if direction > 0. { HEIGHT } else { 0. },
        );
        let end = (
            b[0] + WIDTH / 2.,
            b[1] + if direction > 0. { 0. } else { HEIGHT },
        );
        let bend = ((end.1 - start.1).abs() / 2.).max(40.);
        [
            start,
            (start.0, start.1 + direction * bend),
            (end.0, end.1 - direction * bend),
            end,
        ]
    }
}
fn point(p: Curve, t: f64) -> (f64, f64) {
    let u = 1. - t;
    (
        u * u * u * p[0].0 + 3. * u * u * t * p[1].0 + 3. * u * t * t * p[2].0 + t * t * t * p[3].0,
        u * u * u * p[0].1 + 3. * u * u * t * p[1].1 + 3. * u * t * t * p[2].1 + t * t * t * p[3].1,
    )
}
fn arrow(cr: &cairo::Context, from: (f64, f64), to: (f64, f64)) {
    let angle = (to.1 - from.1).atan2(to.0 - from.0);
    cr.move_to(to.0, to.1);
    cr.line_to(
        to.0 - 10. * (angle - 0.45).cos(),
        to.1 - 10. * (angle - 0.45).sin(),
    );
    cr.move_to(to.0, to.1);
    cr.line_to(
        to.0 - 10. * (angle + 0.45).cos(),
        to.1 - 10. * (angle + 0.45).sin(),
    );
    let _ = cr.stroke();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_001_curve_endpoints_and_finite_paths() {
        for target in [[400., 0.], [0., 400.], [0., 0.], [-400., 0.], [0., -400.]] {
            let p = path([0., 0.], target);
            assert_eq!(point(p, 0.), p[0]);
            assert_eq!(point(p, 1.), p[3]);
            for i in 0..100 {
                let (x, y) = point(p, i as f64 / 100.);
                assert!(x.is_finite() && y.is_finite());
            }
        }
    }
}
