use super::super::super::super::{model::layout::Rect, motion, CanvasView};
use super::super::{background::screen_rect, rgba, token, Rgba};
use super::{
    curves::{self, Curve},
    edge_tiles,
};
use gtk::{cairo, prelude::*};
use std::f64::consts::TAU;
use uuid::Uuid;

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
pub(in crate::app::canvas::scene::paint) fn draw_edges(
    v: &CanvasView,
    snapshot: &gtk::Snapshot,
    width: f64,
    height: f64,
    edge: Rgba,
) {
    let accent = token(v, "memoivy_accent");
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
                    .filter(|e| curves::curve_bounds(e.path, ink).intersects(area))
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
        if !curves::curve_bounds(e.path, 40.).intersects(visible) && e.pulse.is_none() {
            continue;
        }
        let world = curves::curve_bounds(e.path, ink);
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
            let path = curves::path(from, to);
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
                curve: if intro < 1. {
                    curves::head(path, intro)
                } else {
                    path
                },
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
            let path = curves::path(from, to);
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
    curves::arrow(cr, curve[2], curve[3], line * 4.6);
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
        let (x, y) = curves::point(p, back);
        let fall = 1. - i as f64 / 10.;
        cr.set_source_rgba(accent.0, accent.1, accent.2, alpha * 0.55 * fall * fall);
        cr.arc(x, y, 1.5 + 2.5 * fall, 0., TAU);
        let _ = cr.fill();
    }
    let (x, y) = curves::point(p, t);
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
