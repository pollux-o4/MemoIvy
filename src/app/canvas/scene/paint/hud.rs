use super::super::super::{
    input::Interaction,
    model::layout::{Guide, Rect},
    motion,
    port::{CARD_HEIGHT, CARD_WIDTH},
    CanvasView,
};
use super::edges::curves;
use super::token;
use gtk::{cairo, prelude::*};
use uuid::Uuid;

pub(super) fn draw_hud(v: &CanvasView, cr: &cairo::Context) {
    let accent = token(v, "memoivy_accent");
    let guide_color = token(v, "memoivy_danger");
    let camera = v.session.camera.get();
    let animated = v.widgets.window.settings().is_gtk_enable_animations();
    cr.save().expect("cairo save");
    cr.scale(camera.zoom, camera.zoom);
    cr.translate(-camera.x, -camera.y);
    let hair = 1. / camera.zoom;
    if let Some((id, start)) = v.motion.borrow().pulse {
        let elapsed = start.elapsed().as_secs_f64();
        let (_, alpha) = motion::pulse_sample(elapsed, animated);
        if (!animated || elapsed >= motion::TRAVEL_SECONDS) && elapsed < motion::SIGNAL_SECONDS {
            arrivals(v, cr, id, alpha, elapsed);
        }
    }
    for guide in v.session.guides.borrow().iter() {
        cr.set_source_rgba(guide_color.0, guide_color.1, guide_color.2, 0.9);
        cr.set_line_width(hair);
        match *guide {
            Guide::Vertical { x, y0, y1 } => {
                cr.move_to(x, y0 - 12. * hair);
                cr.line_to(x, y1 + 12. * hair);
            }
            Guide::Horizontal { y, x0, x1 } => {
                cr.move_to(x0 - 12. * hair, y);
                cr.line_to(x1 + 12. * hair, y);
            }
        }
        let _ = cr.stroke();
    }
    match &*v.session.interaction.borrow() {
        Interaction::Marquee {
            rect: Some(rect), ..
        } => {
            rounded(cr, *rect, 3. * hair);
            cr.set_source_rgba(accent.0, accent.1, accent.2, 0.1);
            let _ = cr.fill_preserve();
            cr.set_source_rgba(accent.0, accent.1, accent.2, 0.75);
            cr.set_line_width(hair);
            let _ = cr.stroke();
        }
        Interaction::Link { from, end, target } => {
            if let Some(&start) = v.session.shown.borrow().get(from) {
                let curve = match target.and_then(|id| v.session.shown.borrow().get(&id).copied()) {
                    Some(goal) => curves::path(start, goal),
                    None => curves::free_curve(
                        (start[0] + CARD_WIDTH, start[1] + CARD_HEIGHT / 2.),
                        *end,
                    ),
                };
                cr.set_source_rgba(accent.0, accent.1, accent.2, 0.95);
                cr.set_line_width(2. * hair.max(0.75));
                cr.set_line_cap(cairo::LineCap::Round);
                cr.set_line_join(cairo::LineJoin::Round);
                cr.move_to(curve[0].0, curve[0].1);
                cr.curve_to(
                    curve[1].0, curve[1].1, curve[2].0, curve[2].1, curve[3].0, curve[3].1,
                );
                let _ = cr.stroke();
                curves::arrow(cr, curve[2], curve[3], 9. * hair.max(0.75));
            }
            if let Some(rect) = target.and_then(|id| v.shown_rect(id)) {
                rounded(cr, grow(rect, 4.), 18.);
                cr.set_source_rgba(accent.0, accent.1, accent.2, 0.9);
                cr.set_line_width(2.);
                let _ = cr.stroke();
            }
        }
        _ => {}
    }
    cr.restore().expect("cairo restore");
}

fn arrivals(v: &CanvasView, cr: &cairo::Context, id: Uuid, alpha: f64, elapsed: f64) {
    let accent = token(v, "memoivy_accent");
    let done = token(v, "memoivy_success");
    let nodes = v.session.nodes.borrow();
    let Some(node) = nodes.iter().find(|n| n.id == id) else {
        return;
    };
    let grown = 4. + ((elapsed - motion::TRAVEL_SECONDS).max(0.) * 28.);
    for target in &node.next {
        if let Some(rect) = v.shown_rect(*target) {
            rounded(cr, grow(rect, grown), 16. + grown);
            let (r, g, b) = if nodes.iter().any(|n| n.id == *target && n.done) {
                done
            } else {
                accent
            };
            cr.set_source_rgba(r, g, b, alpha * 0.85);
            cr.set_line_width(2.5);
            let _ = cr.stroke();
        }
    }
}

fn grow(r: Rect, by: f64) -> Rect {
    Rect::new(r.x - by, r.y - by, r.w + by * 2., r.h + by * 2.)
}

fn rounded(cr: &cairo::Context, r: Rect, radius: f64) {
    let pi = std::f64::consts::PI;
    cr.new_sub_path();
    cr.arc(r.right() - radius, r.y + radius, radius, -pi / 2., 0.);
    cr.arc(r.right() - radius, r.bottom() - radius, radius, 0., pi / 2.);
    cr.arc(r.x + radius, r.bottom() - radius, radius, pi / 2., pi);
    cr.arc(r.x + radius, r.y + radius, radius, pi, pi * 1.5);
    cr.close_path();
}
