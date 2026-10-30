use super::super::{
    input::Interaction,
    model::layout::{Guide, Rect},
    motion,
    port::{CARD_HEIGHT, CARD_WIDTH},
    CanvasView,
};
use gtk::{cairo, prelude::*};
use std::{f64::consts::TAU, rc::Rc};
use uuid::Uuid;

type Rgba = (f64, f64, f64, f64);
const GRID: f64 = 24.;
const MIN_DOT_GAP: f64 = 16.;

#[allow(deprecated)]
fn token(v: &CanvasView, name: &str) -> (f64, f64, f64) {
    let color = v
        .widgets
        .surface
        .style_context()
        .lookup_color(name)
        .expect("installed application color token");
    (
        f64::from(color.red()),
        f64::from(color.green()),
        f64::from(color.blue()),
    )
}

struct Palette {
    background: (f64, f64, f64),
    dot: Rgba,
    edge: Rgba,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            background: (0.102, 0.106, 0.118),
            dot: (1., 1., 1., 0.13),
            edge: (0.72, 0.76, 0.84, 0.46),
        }
    } else {
        Palette {
            background: (0.961, 0.961, 0.969),
            dot: (0.1, 0.13, 0.2, 0.17),
            edge: (0.32, 0.37, 0.47, 0.5),
        }
    }
}

fn rgba(cr: &cairo::Context, c: Rgba) {
    cr.set_source_rgba(c.0, c.1, c.2, c.3);
}

pub(in super::super) fn attach(view: &Rc<CanvasView>) {
    let weak = Rc::downgrade(view);
    view.widgets
        .backdrop
        .set_draw_func(move |_, cr, width, height| {
            if let Some(v) = weak.upgrade() {
                draw_backdrop(&v, cr, f64::from(width), f64::from(height));
            }
        });
    let weak = Rc::downgrade(view);
    view.widgets.hud.set_draw_func(move |_, cr, _, _| {
        if let Some(v) = weak.upgrade() {
            draw_hud(&v, cr);
        }
    });
    let weak = Rc::downgrade(view);
    view.widgets.backdrop.connect_resize(move |_, _, _| {
        if let Some(v) = weak.upgrade() {
            let weak = Rc::downgrade(&v);
            // 첫 그리기 전에 맞춰 원점에서 튀어 오르는 프레임을 없앤다.
            gtk::glib::idle_add_local_full(gtk::glib::Priority::HIGH_IDLE, move || {
                if let Some(v) = weak.upgrade() {
                    v.first_layout();
                    v.resized();
                }
                gtk::glib::ControlFlow::Break
            });
        }
    });
    if let Some(settings) = gtk::Settings::default() {
        for property in crate::presentation::theme::WATCHED {
            if settings.find_property(property).is_none() {
                continue;
            }
            let weak = view.widgets.backdrop.downgrade();
            settings.connect_notify_local(Some(property), move |_, _| {
                if let Some(area) = weak.upgrade() {
                    area.queue_draw();
                }
            });
        }
    }
}

/// 그릴 것이 없으면 HUD를 숨겨 빈 전체 화면 텍스처를 매 프레임 올리지 않는다.
pub(in super::super) fn sync_hud(v: &CanvasView) {
    let active = !v.session.interaction.borrow().is_idle()
        || !v.session.guides.borrow().is_empty()
        || v.motion.borrow().pulse.is_some();
    v.widgets.hud.set_visible(active);
    if active {
        v.widgets.hud.queue_draw();
    }
}

pub(in super::super) fn draw_backdrop(
    v: &CanvasView,
    cr: &cairo::Context,
    width: f64,
    height: f64,
) {
    let colors = palette(crate::presentation::theme::is_dark(
        &v.widgets.window.settings(),
    ));
    let (r, g, b) = colors.background;
    cr.set_source_rgb(r, g, b);
    let _ = cr.paint();
    draw_grid(v, cr, width, height, colors.dot);
    let camera = v.session.camera.get();
    cr.save().expect("cairo save");
    cr.scale(camera.zoom, camera.zoom);
    cr.translate(-camera.x, -camera.y);
    draw_edges(v, cr, width, height, colors.edge);
    cr.restore().expect("cairo restore");
}

/// 배율과 무관하게 점 수를 제한한다. 축소하면 간격을 두 배로 키우고, 사라질 점은 서서히 흐려진다.
fn draw_grid(v: &CanvasView, cr: &cairo::Context, width: f64, height: f64, dot: Rgba) {
    let camera = v.session.camera.get();
    let mut step = GRID;
    while step * camera.zoom < MIN_DOT_GAP {
        step *= 2.;
    }
    let fade = ((step * camera.zoom - MIN_DOT_GAP) / MIN_DOT_GAP).clamp(0., 1.);
    let major = step * 2.;
    let x0 = (camera.x / step).floor() as i64;
    let y0 = (camera.y / step).floor() as i64;
    let x1 = ((camera.x + width / camera.zoom) / step).ceil() as i64;
    let y1 = ((camera.y + height / camera.zoom) / step).ceil() as i64;
    for pass in 0..2 {
        let is_major = pass == 1;
        let alpha = if is_major { dot.3 } else { dot.3 * fade };
        if alpha <= 0.002 {
            continue;
        }
        for ix in x0..=x1 {
            for iy in y0..=y1 {
                let world = (ix as f64 * step, iy as f64 * step);
                let on_major = (world.0 / major).fract() == 0. && (world.1 / major).fract() == 0.;
                if on_major != is_major {
                    continue;
                }
                let (sx, sy) = camera.to_screen(world);
                cr.rectangle(sx.round() - 0.75, sy.round() - 0.75, 1.5, 1.5);
            }
        }
        cr.set_source_rgba(dot.0, dot.1, dot.2, alpha);
        let _ = cr.fill();
    }
}

fn draw_edges(v: &CanvasView, cr: &cairo::Context, width: f64, height: f64, edge: Rgba) {
    let accent = token(v, "lsm_accent");
    let camera = v.session.camera.get();
    let visible = Rect::new(
        camera.x,
        camera.y,
        width / camera.zoom,
        height / camera.zoom,
    );
    let animated = v.widgets.window.settings().is_gtk_enable_animations();
    let selection = v.session.selection.borrow();
    let motion_state = v.motion.borrow();
    let shown = v.session.shown.borrow();
    let nodes = v.session.nodes.borrow();
    let line = 1.5 / camera.zoom.max(0.45);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    for node in nodes.iter() {
        let Some(&from) = shown.get(&node.id) else {
            continue;
        };
        for target in &node.next {
            let Some(&to) = shown.get(target) else {
                continue;
            };
            let p = path(from, to);
            let active = motion_state.pulse.filter(|(id, t)| {
                *id == node.id && t.elapsed().as_secs_f64() < motion::SIGNAL_SECONDS
            });
            if !curve_bounds(p, 40.).intersects(visible) && active.is_none() {
                continue;
            }
            let highlighted = selection.contains(node.id) || selection.contains(*target);
            let intro = motion_state
                .link
                .filter(|(a, b, _)| *a == node.id && b == target)
                .map_or(1., |(_, _, t)| {
                    motion::link_progress(t.elapsed().as_secs_f64())
                });
            let curve = if intro < 1. { head(p, intro) } else { p };
            let color: Rgba = if active.is_some() || highlighted {
                (accent.0, accent.1, accent.2, 0.95)
            } else {
                edge
            };
            rgba(cr, color);
            cr.set_line_width(if highlighted { line * 1.35 } else { line });
            cr.move_to(curve[0].0, curve[0].1);
            cr.curve_to(
                curve[1].0, curve[1].1, curve[2].0, curve[2].1, curve[3].0, curve[3].1,
            );
            let _ = cr.stroke();
            arrow(cr, curve[2], curve[3], line * 4.6);
            if let Some((_, start)) = active {
                let elapsed = start.elapsed().as_secs_f64();
                let (t, alpha) = motion::pulse_sample(elapsed, animated);
                comet(cr, p, t, alpha, animated, accent);
            }
        }
    }
}

/// 신호 머리와 잦아드는 꼬리. 모션 줄이기에서는 머리만 도착점에 표시한다.
fn comet(
    cr: &cairo::Context,
    p: Curve,
    t: f64,
    alpha: f64,
    animated: bool,
    accent: (f64, f64, f64),
) {
    let tail = if animated { 9 } else { 0 };
    for i in (0..=tail).rev() {
        let back = (t - i as f64 * 0.018).max(0.);
        let (x, y) = point(p, back);
        let fall = 1. - i as f64 / 10.;
        cr.set_source_rgba(accent.0, accent.1, accent.2, alpha * 0.55 * fall * fall);
        cr.arc(x, y, 1.5 + 2.5 * fall, 0., TAU);
        let _ = cr.fill();
    }
    let (x, y) = point(p, t);
    cr.set_source_rgba(accent.0, accent.1, accent.2, alpha * 0.16);
    cr.arc(x, y, 13., 0., TAU);
    let _ = cr.fill();
    cr.set_source_rgba(1., 1., 1., alpha);
    cr.arc(x, y, 5.5, 0., TAU);
    let _ = cr.fill();
    cr.set_source_rgba(accent.0, accent.1, accent.2, alpha);
    cr.arc(x, y, 4., 0., TAU);
    let _ = cr.fill();
}

fn draw_hud(v: &CanvasView, cr: &cairo::Context) {
    let accent = token(v, "lsm_accent");
    let guide_color = token(v, "lsm_danger");
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
                    Some(goal) => path(start, goal),
                    None => free_curve((start[0] + CARD_WIDTH, start[1] + CARD_HEIGHT / 2.), *end),
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
                arrow(cr, curve[2], curve[3], 9. * hair.max(0.75));
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
    let accent = token(v, "lsm_accent");
    let done = token(v, "lsm_success");
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

type Curve = [(f64, f64); 4];
// shortcut: 수동 배치용 방향 곡선이라 다른 카드를 자동 회피하지 않음, 복잡한 보드 사용에서 겹침이 반복되면 라우터 검토.
fn path(a: [f64; 2], b: [f64; 2]) -> Curve {
    if (b[0] - a[0]).abs() >= CARD_WIDTH + 24. {
        let direction = if b[0] > a[0] { 1. } else { -1. };
        let start = (
            a[0] + if direction > 0. { CARD_WIDTH } else { 0. },
            a[1] + CARD_HEIGHT / 2.,
        );
        let end = (
            b[0] + if direction > 0. { 0. } else { CARD_WIDTH },
            b[1] + CARD_HEIGHT / 2.,
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
            a[0] + CARD_WIDTH / 2.,
            a[1] + if direction > 0. { CARD_HEIGHT } else { 0. },
        );
        let end = (
            b[0] + CARD_WIDTH / 2.,
            b[1] + if direction > 0. { 0. } else { CARD_HEIGHT },
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

/// 끌고 있는 연결 미리보기. 곡률을 거리 안으로 제한해 왼쪽으로 끌어도 고리가 생기지 않는다.
fn free_curve(start: (f64, f64), end: (f64, f64)) -> Curve {
    let bend = ((end.0 - start.0).abs() / 2.).min(96.);
    let side = if end.0 >= start.0 { 1. } else { -1. };
    [
        start,
        (start.0 + bend, start.1),
        (end.0 - side * bend, end.1),
        end,
    ]
}

fn curve_bounds(p: Curve, margin: f64) -> Rect {
    let xs = p.map(|q| q.0);
    let ys = p.map(|q| q.1);
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Rect::new(
        min_x - margin,
        min_y - margin,
        max_x - min_x + margin * 2.,
        max_y - min_y + margin * 2.,
    )
}

fn point(p: Curve, t: f64) -> (f64, f64) {
    let u = 1. - t;
    (
        u * u * u * p[0].0 + 3. * u * u * t * p[1].0 + 3. * u * t * t * p[2].0 + t * t * t * p[3].0,
        u * u * u * p[0].1 + 3. * u * u * t * p[1].1 + 3. * u * t * t * p[2].1 + t * t * t * p[3].1,
    )
}

/// 곡선의 앞부분 t까지를 같은 모양으로 잘라 새 연결이 그려지는 효과를 만든다.
fn head(p: Curve, t: f64) -> Curve {
    let mix = |a: (f64, f64), b: (f64, f64)| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
    let ab = mix(p[0], p[1]);
    let bc = mix(p[1], p[2]);
    let cd = mix(p[2], p[3]);
    let abc = mix(ab, bc);
    let bcd = mix(bc, cd);
    [p[0], ab, abc, mix(abc, bcd)]
}

fn arrow(cr: &cairo::Context, from: (f64, f64), to: (f64, f64), size: f64) {
    let angle = (to.1 - from.1).atan2(to.0 - from.0);
    cr.move_to(to.0, to.1);
    cr.line_to(
        to.0 - size * (angle - 0.42).cos(),
        to.1 - size * (angle - 0.42).sin(),
    );
    cr.line_to(
        to.0 - size * (angle + 0.42).cos(),
        to.1 - size * (angle + 0.42).sin(),
    );
    cr.close_path();
    let _ = cr.fill();
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
            let half = head(p, 0.5);
            let end = point(p, 0.5);
            assert!((half[3].0 - end.0).abs() < 1e-9 && (half[3].1 - end.1).abs() < 1e-9);
        }
    }
}
