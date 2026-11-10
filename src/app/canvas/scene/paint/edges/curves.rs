use super::super::super::super::model::layout::Rect;
use super::super::super::super::port::{CARD_HEIGHT, CARD_WIDTH};
use gtk::cairo;

pub(in crate::app::canvas::scene::paint) type Curve = [(f64, f64); 4];
// shortcut: 수동 배치용 방향 곡선이라 다른 카드를 자동 회피하지 않음, 복잡한 보드 사용에서 겹침이 반복되면 라우터 검토.
pub(in crate::app::canvas::scene::paint) fn path(a: [f64; 2], b: [f64; 2]) -> Curve {
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
pub(in crate::app::canvas::scene::paint) fn free_curve(
    start: (f64, f64),
    end: (f64, f64),
) -> Curve {
    let bend = ((end.0 - start.0).abs() / 2.).min(96.);
    let side = if end.0 >= start.0 { 1. } else { -1. };
    [
        start,
        (start.0 + bend, start.1),
        (end.0 - side * bend, end.1),
        end,
    ]
}

pub(in crate::app::canvas::scene::paint) fn curve_bounds(p: Curve, margin: f64) -> Rect {
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

pub(in crate::app::canvas::scene::paint) fn point(p: Curve, t: f64) -> (f64, f64) {
    let u = 1. - t;
    (
        u * u * u * p[0].0 + 3. * u * u * t * p[1].0 + 3. * u * t * t * p[2].0 + t * t * t * p[3].0,
        u * u * u * p[0].1 + 3. * u * u * t * p[1].1 + 3. * u * t * t * p[2].1 + t * t * t * p[3].1,
    )
}

/// 곡선의 앞부분 t까지를 같은 모양으로 잘라 새 연결이 그려지는 효과를 만든다.
pub(in crate::app::canvas::scene::paint) fn head(p: Curve, t: f64) -> Curve {
    let mix = |a: (f64, f64), b: (f64, f64)| (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
    let ab = mix(p[0], p[1]);
    let bc = mix(p[1], p[2]);
    let cd = mix(p[2], p[3]);
    let abc = mix(ab, bc);
    let bcd = mix(bc, cd);
    [p[0], ab, abc, mix(abc, bcd)]
}

pub(in crate::app::canvas::scene::paint) fn arrow(
    cr: &cairo::Context,
    from: (f64, f64),
    to: (f64, f64),
    size: f64,
) {
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
