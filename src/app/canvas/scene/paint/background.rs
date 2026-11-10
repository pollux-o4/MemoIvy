use super::super::super::{model::camera::Camera, CanvasView};
use super::Rgba;
use gtk::{gdk, graphene, prelude::*};

const GRID: f64 = 24.;
const MIN_DOT_GAP: f64 = 16.;

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

pub(super) fn screen_rect(x: f64, y: f64, w: f64, h: f64) -> graphene::Rect {
    graphene::Rect::new(x as f32, y as f32, w as f32, h as f32)
}

/// 배경·격자는 GPU 노드로, 연결선은 곡선이 덮는 화면 영역만 Cairo로 그린다.
/// 창 전체 CPU 표면을 만들지 않아 이동 비용이 창 크기·배율에 거의 묶이지 않는다.
pub(in crate::app::canvas) fn draw_backdrop(
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
    super::edges::render::draw_edges(v, snapshot, width, height, colors.edge);
}

/// 배율과 무관하게 점 수를 제한한다. 축소하면 간격을 두 배로 키우고, 사라질 점은 서서히 흐려진다.
/// 주 점 하나와 보조 점 셋을 담은 타일을 반복 노드로 깔아 점 수와 무관하게 노드 몇 개로 끝낸다.
pub(super) fn draw_grid(
    camera: Camera,
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
