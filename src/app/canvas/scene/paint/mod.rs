//! 보드 배경·격자·연결선·HUD 그리기. 렌더 노드 위젯(backdrop)과 연결선 타일 캐시(edge_tiles)를 함께 둔다.
pub(in crate::app::canvas) mod backdrop;
pub(in crate::app::canvas) mod edge_tiles;

use super::super::{
    input::Interaction,
    model::layout::{Guide, Rect},
    motion,
    port::{CARD_HEIGHT, CARD_WIDTH},
    CanvasView,
};
use gtk::{cairo, gdk, graphene, prelude::*};
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
        .set_draw_func(move |snapshot, width, height| {
            if let Some(v) = weak.upgrade() {
                draw_backdrop(&v, snapshot, width, height);
            }
        });
    let weak = Rc::downgrade(view);
    view.widgets.hud.set_draw_func(move |_, cr, _, _| {
        if let Some(v) = weak.upgrade() {
            draw_hud(&v, cr);
        }
    });
    let weak = Rc::downgrade(view);
    view.widgets.backdrop.connect_resized(move || {
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

/// 그릴 것이 없으면 HUD를 숨겨 빈 전체 화면 텍스처를 매 프레임 올리지 않는다. 화면 이동은 HUD에 그릴 것이 없다.
pub(in super::super) fn sync_hud(v: &CanvasView) {
    let active = !matches!(
        *v.session.interaction.borrow(),
        Interaction::Idle | Interaction::Pan { .. }
    ) || !v.session.guides.borrow().is_empty()
        || v.motion.borrow().pulse.is_some();
    v.widgets.hud.set_visible(active);
    if active {
        v.widgets.hud.queue_draw();
    }
}

fn screen_rect(x: f64, y: f64, w: f64, h: f64) -> graphene::Rect {
    graphene::Rect::new(x as f32, y as f32, w as f32, h as f32)
}

/// 배경·격자는 GPU 노드로, 연결선은 곡선이 덮는 화면 영역만 Cairo로 그린다.
/// 창 전체 CPU 표면을 만들지 않아 이동 비용이 창 크기·배율에 거의 묶이지 않는다.
pub(in super::super) fn draw_backdrop(
    v: &CanvasView,
    snapshot: &gtk::Snapshot,
    width: f64,
    height: f64,
) {
    let colors = palette(crate::presentation::theme::is_dark(
        &v.widgets.window.settings(),
    ));
    let (r, g, b) = colors.background;
    snapshot.append_color(
        &gdk::RGBA::new(r as f32, g as f32, b as f32, 1.),
        &screen_rect(0., 0., width, height),
    );
    draw_grid(v.session.camera.get(), snapshot, width, height, colors.dot);
    draw_edges(v, snapshot, width, height, colors.edge);
}

/// 배율과 무관하게 점 수를 제한한다. 축소하면 간격을 두 배로 키우고, 사라질 점은 서서히 흐려진다.
/// 주 점 하나와 보조 점 셋을 담은 타일을 반복 노드로 깔아 점 수와 무관하게 노드 몇 개로 끝낸다.
fn draw_grid(
    camera: super::super::model::camera::Camera,
    snapshot: &gtk::Snapshot,
    width: f64,
    height: f64,
    dot: Rgba,
) {
    let mut step = GRID;
    while step * camera.zoom < MIN_DOT_GAP {
        step *= 2.;
    }
    let fade = ((step * camera.zoom - MIN_DOT_GAP) / MIN_DOT_GAP).clamp(0., 1.);
    let major = step * 2.;
    let gap = step * camera.zoom;
    let half = gap / 2.;
    // 주 점이 타일 안쪽 (half, half)에 오도록 반 칸 당겨 점이 타일 경계에서 잘리지 않게 한다.
    let (mx, my) = camera.to_screen((
        (camera.x / major).floor() * major,
        (camera.y / major).floor() * major,
    ));
    let origin = (mx - half, my - half);
    snapshot.push_repeat(
        &screen_rect(0., 0., width, height),
        Some(&screen_rect(origin.0, origin.1, gap * 2., gap * 2.)),
    );
    let minor = dot.3 * fade;
    for (x, y, alpha) in [
        (half, half, dot.3),
        (half + gap, half, minor),
        (half, half + gap, minor),
        (half + gap, half + gap, minor),
    ] {
        if alpha > 0.002 {
            snapshot.append_color(
                &gdk::RGBA::new(dot.0 as f32, dot.1 as f32, dot.2 as f32, alpha as f32),
                &screen_rect(origin.0 + x - 0.75, origin.1 + y - 0.75, 1.5, 1.5),
            );
        }
    }
    snapshot.pop();
}

struct Edge {
    id: Uuid,
    path: Curve,
    curve: Curve,
    color: Rgba,
    width: f64,
    /// 프로젝트 안쪽 메모로 이어지는 투영 연결. 점선으로 그리고 편집 대상이 아니다.
    dashed: bool,
    pulse: Option<std::time::Instant>,
}

/// 정적인 연결선은 타일 텍스처로 재사용하고, 신호·새 연결 애니메이션 중에는 곡선이 덮는 영역만 매 프레임 그린다.
fn draw_edges(v: &CanvasView, snapshot: &gtk::Snapshot, width: f64, height: f64, edge: Rgba) {
    let accent = token(v, "lsm_accent");
    let camera = v.session.camera.get();
    let visible = Rect::new(
        camera.x,
        camera.y,
        width / camera.zoom,
        height / camera.zoom,
    );
    let animated = v.widgets.window.settings().is_gtk_enable_animations();
    let line = 1.5 / camera.zoom.max(0.45);
    // 화살촉·신호 머리(반지름 13)까지 덮는 월드 여백.
    let ink = line * 6. + 14.;
    let edges = edge_list(v, line, edge, accent);
    let busy = edges.iter().any(|e| e.pulse.is_some() || e.curve != e.path);
    let scale = v.widgets.backdrop.scale_factor();
    let key = edge_key(&edges, camera.zoom, scale);
    let mut tiles = v.session.edge_tiles.borrow_mut();
    if !busy && tiles.stable(key) {
        let tile = edge_tiles::TILE / camera.zoom;
        tiles.draw(
            snapshot,
            (camera.x * camera.zoom, camera.y * camera.zoom),
            (width, height),
            scale,
            |cr, (ix, iy)| {
                let area = Rect::new(ix as f64 * tile, iy as f64 * tile, tile, tile);
                cr.scale(camera.zoom, camera.zoom);
                let mut drew = false;
                for e in edges
                    .iter()
                    .filter(|e| curve_bounds(e.path, ink).intersects(area))
                {
                    stroke_edge(cr, e, line);
                    drew = true;
                }
                drew
            },
        );
        return;
    }
    for e in &edges {
        if !curve_bounds(e.path, 40.).intersects(visible) && e.pulse.is_none() {
            continue;
        }
        let world = curve_bounds(e.path, ink);
        let (x0, y0) = camera.to_screen((world.x, world.y));
        let (x1, y1) = camera.to_screen((world.right(), world.bottom()));
        let (x0, y0) = (x0.max(0.).floor(), y0.max(0.).floor());
        let (x1, y1) = (x1.min(width).ceil(), y1.min(height).ceil());
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let cr = snapshot.append_cairo(&screen_rect(x0, y0, x1 - x0, y1 - y0));
        cr.scale(camera.zoom, camera.zoom);
        cr.translate(-camera.x, -camera.y);
        stroke_edge(&cr, e, line);
        if let Some(start) = e.pulse {
            let elapsed = start.elapsed().as_secs_f64();
            let (t, alpha) = motion::pulse_sample(elapsed, animated);
            comet(&cr, e.path, t, alpha, animated, accent);
        }
    }
}

fn edge_list(v: &CanvasView, line: f64, edge: Rgba, accent: (f64, f64, f64)) -> Vec<Edge> {
    let selection = v.session.selection.borrow();
    let motion_state = v.motion.borrow();
    let shown = v.session.shown.borrow();
    let nodes = v.session.nodes.borrow();
    let mut edges = Vec::new();
    for node in nodes.iter() {
        let Some(&from) = shown.get(&node.id) else {
            continue;
        };
        for target in &node.next {
            let Some(&to) = shown.get(target) else {
                continue;
            };
            let path = path(from, to);
            let pulse = motion_state
                .pulse
                .filter(|(id, t)| {
                    *id == node.id && t.elapsed().as_secs_f64() < motion::SIGNAL_SECONDS
                })
                .map(|(_, t)| t);
            let highlighted = selection.contains(node.id) || selection.contains(*target);
            let intro = motion_state
                .link
                .filter(|(a, b, _)| *a == node.id && b == target)
                .map_or(1., |(_, _, t)| {
                    motion::link_progress(t.elapsed().as_secs_f64())
                });
            edges.push(Edge {
                id: node.id,
                path,
                curve: if intro < 1. { head(path, intro) } else { path },
                color: if pulse.is_some() || highlighted {
                    (accent.0, accent.1, accent.2, 0.95)
                } else {
                    edge
                },
                width: if highlighted { line * 1.35 } else { line },
                dashed: false,
                pulse,
            });
        }
        for target in &node.through {
            let Some(&to) = shown.get(target) else {
                continue;
            };
            let path = path(from, to);
            edges.push(Edge {
                id: node.id,
                path,
                curve: path,
                color: (edge.0, edge.1, edge.2, edge.3 * 0.7),
                width: line,
                dashed: true,
                pulse: None,
            });
        }
    }
    edges
}

/// 화면 이동을 뺀, 연결선 모양을 정하는 모든 값.
fn edge_key(edges: &[Edge], zoom: f64, scale: i32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    zoom.to_bits().hash(&mut hash);
    scale.hash(&mut hash);
    for e in edges {
        e.id.hash(&mut hash);
        for value in e.curve.iter().flat_map(|p| [p.0, p.1]) {
            value.to_bits().hash(&mut hash);
        }
        for value in [e.color.0, e.color.1, e.color.2, e.color.3, e.width] {
            value.to_bits().hash(&mut hash);
        }
        e.dashed.hash(&mut hash);
    }
    hash.finish()
}

fn stroke_edge(cr: &cairo::Context, e: &Edge, line: f64) {
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    rgba(cr, e.color);
    cr.set_line_width(e.width);
    let curve = e.curve;
    if e.dashed {
        cr.set_dash(&[line * 3., line * 3.], 0.);
    }
    cr.move_to(curve[0].0, curve[0].1);
    cr.curve_to(
        curve[1].0, curve[1].1, curve[2].0, curve[2].1, curve[3].0, curve[3].1,
    );
    let _ = cr.stroke();
    cr.set_dash(&[], 0.);
    arrow(cr, curve[2], curve[3], line * 4.6);
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
pub(in crate::app::canvas) mod tests {
    use super::*;
    fn pixels(
        renderer: &gtk::gsk::CairoRenderer,
        snapshot: gtk::Snapshot,
        size: (i32, i32),
    ) -> Vec<u8> {
        let mut pixels = vec![0; (size.0 * size.1 * 4) as usize];
        if let Some(node) = snapshot.to_node() {
            let area = screen_rect(0., 0., f64::from(size.0), f64::from(size.1));
            renderer
                .render_texture(&node, Some(&area))
                .download(&mut pixels, size.0 as usize * 4);
        }
        pixels
    }

    /// 화면 이동 중 재사용하는 연결선 타일이 직접 그린 곡선과 같은 화면을 만든다.
    pub(in crate::app::canvas) fn canvas_002_edge_tiles_match_direct(v: &CanvasView) {
        let renderer = gtk::gsk::CairoRenderer::new();
        renderer.realize(None).unwrap();
        v.motion.borrow_mut().link = None;
        *v.session.edge_tiles.borrow_mut() = Default::default();
        let size = (640, 400);
        let frame = || {
            let snapshot = gtk::Snapshot::new();
            draw_edges(v, &snapshot, 640., 400., (0., 0., 0., 1.));
            pixels(&renderer, snapshot, size)
        };
        let direct = frame();
        assert!(
            v.session.edge_tiles.borrow().is_empty(),
            "first frame draws directly"
        );
        let tiled = frame();
        assert!(
            !v.session.edge_tiles.borrow().is_empty(),
            "stable frame reuses tiles"
        );
        let changed = frame();
        renderer.unrealize();
        assert!(direct.iter().any(|&p| p != 0), "fixture must show an edge");
        let worst = direct.iter().zip(&tiled).map(|(a, b)| a.abs_diff(*b)).max();
        assert!(
            worst <= Some(2),
            "tiles must match direct drawing: {worst:?}"
        );
        assert_eq!(tiled, changed);
    }

    /// GTK 초기화가 필요해 캔버스 GTK 시나리오에서 호출한다.
    pub(in crate::app::canvas) fn canvas_002_grid_preserves_subpixel_motion() {
        let renderer = gtk::gsk::CairoRenderer::new();
        renderer.realize(None).unwrap();
        let sample = |x| {
            let snapshot = gtk::Snapshot::new();
            draw_grid(
                super::super::super::model::camera::Camera { x, y: 0., zoom: 1. },
                &snapshot,
                64.,
                64.,
                (1., 1., 1., 1.),
            );
            pixels(&renderer, snapshot, (64, 64))
        };
        let (still, moved) = (sample(0.), sample(0.25));
        renderer.unrealize();
        assert!(still.iter().any(|&p| p != 0), "grid tile must paint dots");
        assert_ne!(
            still, moved,
            "slow pan must not snap the grid to whole pixels"
        );
    }

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
