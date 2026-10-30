use std::collections::HashMap;
use uuid::Uuid;

const TIDY_GAP: f64 = 32.;
const FLOW_COLUMN_GAP: f64 = 96.;
const FLOW_ROW_GAP: f64 = 32.;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::app::windows::canvas) struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
    pub fn from_corners(a: (f64, f64), b: (f64, f64)) -> Self {
        Self::new(
            a.0.min(b.0),
            a.1.min(b.1),
            (a.0 - b.0).abs(),
            (a.1 - b.1).abs(),
        )
    }
    pub fn right(self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(self) -> f64 {
        self.y + self.h
    }
    pub fn contains(self, p: (f64, f64)) -> bool {
        p.0 >= self.x && p.0 <= self.right() && p.1 >= self.y && p.1 <= self.bottom()
    }
    pub fn intersects(self, other: Rect) -> bool {
        self.x <= other.right()
            && other.x <= self.right()
            && self.y <= other.bottom()
            && other.y <= self.bottom()
    }
    pub fn union(self, other: Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }
}

pub(in crate::app::windows::canvas) fn bounds(
    rects: impl IntoIterator<Item = Rect>,
) -> Option<Rect> {
    rects.into_iter().reduce(Rect::union)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::app::windows::canvas) enum Align {
    Left,
    CenterX,
    Right,
    Top,
    CenterY,
    Bottom,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::app::windows::canvas) enum Axis {
    Horizontal,
    Vertical,
}

pub(in crate::app::windows::canvas) type Node = (Uuid, Rect);
pub(in crate::app::windows::canvas) type Moves = Vec<(Uuid, [f64; 2])>;

/// 선택 영역의 경계를 기준으로 맞춘다. 두 개 미만이면 바꾸지 않는다.
pub(in crate::app::windows::canvas) fn align(nodes: &[Node], how: Align) -> Moves {
    if nodes.len() < 2 {
        return Vec::new();
    }
    let area = bounds(nodes.iter().map(|n| n.1)).expect("at least two nodes");
    nodes
        .iter()
        .map(|&(id, r)| {
            let (mut x, mut y) = (r.x, r.y);
            match how {
                Align::Left => x = area.x,
                Align::CenterX => x = area.x + (area.w - r.w) / 2.,
                Align::Right => x = area.right() - r.w,
                Align::Top => y = area.y,
                Align::CenterY => y = area.y + (area.h - r.h) / 2.,
                Align::Bottom => y = area.bottom() - r.h,
            }
            (id, [x, y])
        })
        .collect()
}

/// 양 끝을 고정하고 사이 간격을 같게 만든다. 세 개 미만이면 바꾸지 않는다.
pub(in crate::app::windows::canvas) fn distribute(nodes: &[Node], axis: Axis) -> Moves {
    if nodes.len() < 3 {
        return Vec::new();
    }
    let start = |r: &Rect| match axis {
        Axis::Horizontal => r.x,
        Axis::Vertical => r.y,
    };
    let size = |r: &Rect| match axis {
        Axis::Horizontal => r.w,
        Axis::Vertical => r.h,
    };
    let mut sorted = nodes.to_vec();
    sorted.sort_by(|a, b| start(&a.1).total_cmp(&start(&b.1)));
    let first = start(&sorted[0].1);
    let last = sorted[sorted.len() - 1].1;
    let occupied: f64 = sorted.iter().map(|n| size(&n.1)).sum();
    let gap = (start(&last) + size(&last) - first - occupied) / (sorted.len() - 1) as f64;
    let mut cursor = first;
    sorted
        .iter()
        .map(|&(id, r)| {
            let at = cursor;
            cursor += size(&r) + gap;
            match axis {
                Axis::Horizontal => (id, [at, r.y]),
                Axis::Vertical => (id, [r.x, at]),
            }
        })
        .collect()
}

/// 현재 줄·칸 구성을 유지한 격자로 정리한다.
pub(in crate::app::windows::canvas) fn tidy(nodes: &[Node]) -> Moves {
    if nodes.len() < 2 {
        return Vec::new();
    }
    let area = bounds(nodes.iter().map(|n| n.1)).expect("at least two nodes");
    let mut sorted = nodes.to_vec();
    sorted.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
    let mut rows: Vec<Vec<Node>> = Vec::new();
    for node in sorted {
        let joins = rows.last().is_some_and(|row| {
            let center = row.iter().map(|n| n.1.y + n.1.h / 2.).sum::<f64>() / row.len() as f64;
            (node.1.y + node.1.h / 2. - center).abs() < node.1.h / 2.
        });
        if joins {
            rows.last_mut().expect("joined row").push(node);
        } else {
            rows.push(vec![node]);
        }
    }
    let cell_w = nodes.iter().map(|n| n.1.w).fold(0., f64::max) + TIDY_GAP;
    let mut moves = Vec::with_capacity(nodes.len());
    let mut y = area.y;
    for row in &mut rows {
        row.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
        for (column, &(id, _)) in row.iter().enumerate() {
            moves.push((id, [area.x + column as f64 * cell_w, y]));
        }
        y += row.iter().map(|n| n.1.h).fold(0., f64::max) + TIDY_GAP;
    }
    moves
}

/// 연결 방향을 왼쪽에서 오른쪽으로 읽히도록 층으로 배치한다. 순환은 되돌아가는 연결을 무시해 끊는다.
pub(in crate::app::windows::canvas) fn flow(nodes: &[Node], edges: &[(Uuid, Uuid)]) -> Moves {
    if nodes.len() < 2 {
        return Vec::new();
    }
    let area = bounds(nodes.iter().map(|n| n.1)).expect("at least two nodes");
    let mut order = nodes.to_vec();
    order.sort_by(|a, b| a.1.y.total_cmp(&b.1.y).then(a.1.x.total_cmp(&b.1.x)));
    let index: HashMap<Uuid, usize> = order.iter().enumerate().map(|(i, n)| (n.0, i)).collect();
    let mut out = vec![Vec::new(); order.len()];
    for (from, to) in edges {
        if let (Some(&a), Some(&b)) = (index.get(from), index.get(to)) {
            if a != b {
                out[a].push(b);
            }
        }
    }
    let forward = forward_edges(&out);
    let mut rank = vec![0_usize; order.len()];
    for _ in 0..order.len() {
        let mut changed = false;
        for &(a, b) in &forward {
            if rank[b] < rank[a] + 1 {
                rank[b] = rank[a] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let columns = rank.iter().max().map_or(1, |r| r + 1);
    let mut column_nodes = vec![Vec::new(); columns];
    for (i, &r) in rank.iter().enumerate() {
        column_nodes[r].push(i);
    }
    // 앞 층의 평균 순서를 따라 줄을 맞춰 선 교차를 줄인다.
    let mut row_of = vec![0_f64; order.len()];
    for column in &mut column_nodes {
        let key = |i: usize, rows: &[f64]| {
            let parents: Vec<f64> = forward
                .iter()
                .filter(|edge| edge.1 == i)
                .map(|edge| rows[edge.0])
                .collect();
            if parents.is_empty() {
                i as f64
            } else {
                parents.iter().sum::<f64>() / parents.len() as f64
            }
        };
        let keys: Vec<(usize, f64)> = column.iter().map(|&i| (i, key(i, &row_of))).collect();
        let mut sorted = keys;
        sorted.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        *column = sorted.iter().map(|k| k.0).collect();
        for (row, &i) in column.iter().enumerate() {
            row_of[i] = row as f64;
        }
    }
    let cell_w = order.iter().map(|n| n.1.w).fold(0., f64::max) + FLOW_COLUMN_GAP;
    let cell_h = order.iter().map(|n| n.1.h).fold(0., f64::max) + FLOW_ROW_GAP;
    let tallest = column_nodes.iter().map(Vec::len).max().unwrap_or(1) as f64;
    let mut moves = Vec::with_capacity(order.len());
    for (c, column) in column_nodes.iter().enumerate() {
        let offset = (tallest - column.len() as f64) * cell_h / 2.;
        for (row, &i) in column.iter().enumerate() {
            moves.push((
                order[i].0,
                [
                    area.x + c as f64 * cell_w,
                    area.y + offset + row as f64 * cell_h,
                ],
            ));
        }
    }
    moves
}

fn forward_edges(out: &[Vec<usize>]) -> Vec<(usize, usize)> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        New,
        Active,
        Done,
    }
    let mut marks = vec![Mark::New; out.len()];
    let mut edges = Vec::new();
    for root in 0..out.len() {
        if marks[root] != Mark::New {
            continue;
        }
        let mut stack = vec![(root, 0)];
        marks[root] = Mark::Active;
        while let Some(&mut (node, ref mut next)) = stack.last_mut() {
            if let Some(&target) = out[node].get(*next) {
                *next += 1;
                match marks[target] {
                    Mark::Active => {}
                    Mark::New => {
                        edges.push((node, target));
                        marks[target] = Mark::Active;
                        stack.push((target, 0));
                    }
                    Mark::Done => edges.push((node, target)),
                }
            } else {
                marks[node] = Mark::Done;
                stack.pop();
            }
        }
    }
    edges
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::app::windows::canvas) enum Guide {
    Vertical { x: f64, y0: f64, y1: f64 },
    Horizontal { y: f64, x0: f64, x1: f64 },
}

/// 다른 카드의 가장자리·중심에 가장 가까운 축별 보정값과 표시할 안내선(축마다 하나)을 돌려준다.
pub(in crate::app::windows::canvas) fn snap(
    moving: Rect,
    others: &[Rect],
    threshold: f64,
) -> (f64, f64, Vec<Guide>) {
    let lines = |r: Rect| [r.x, r.x + r.w / 2., r.right()];
    let rows = |r: Rect| [r.y, r.y + r.h / 2., r.bottom()];
    let best = |own: [f64; 3], candidates: &mut dyn Iterator<Item = [f64; 3]>| {
        let mut found: Option<f64> = None;
        for other in candidates {
            for a in own {
                for b in other {
                    let delta = b - a;
                    if delta.abs() <= threshold && found.is_none_or(|f| delta.abs() < f.abs()) {
                        found = Some(delta);
                    }
                }
            }
        }
        found.unwrap_or(0.)
    };
    let dx = best(lines(moving), &mut others.iter().map(|r| lines(*r)));
    let dy = best(rows(moving), &mut others.iter().map(|r| rows(*r)));
    let snapped = Rect::new(moving.x + dx, moving.y + dy, moving.w, moving.h);
    let mut guides = Vec::new();
    for x in lines(snapped) {
        let matching: Vec<Rect> = others
            .iter()
            .copied()
            .filter(|r| lines(*r).iter().any(|l| (l - x).abs() < 0.01))
            .collect();
        if let Some(area) = bounds(matching.into_iter().chain([snapped])) {
            if area != snapped {
                guides.push(Guide::Vertical {
                    x,
                    y0: area.y,
                    y1: area.bottom(),
                });
                break;
            }
        }
    }
    for y in rows(snapped) {
        let matching: Vec<Rect> = others
            .iter()
            .copied()
            .filter(|r| rows(*r).iter().any(|l| (l - y).abs() < 0.01))
            .collect();
        if let Some(area) = bounds(matching.into_iter().chain([snapped])) {
            if area != snapped {
                guides.push(Guide::Horizontal {
                    y,
                    x0: area.x,
                    x1: area.right(),
                });
                break;
            }
        }
    }
    (dx, dy, guides)
}

/// 묶음 전체의 상대 위치를 유지한 채 허용 범위 안으로 이동량을 제한한다.
pub(in crate::app::windows::canvas) fn clamp_delta(
    origins: &[[f64; 2]],
    delta: (f64, f64),
    limit: f64,
) -> (f64, f64) {
    let min_x = origins.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let max_x = origins
        .iter()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = origins.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let max_y = origins
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    (
        delta.0.clamp(-min_x, (limit - max_x).max(-min_x)),
        delta.1.clamp(-min_y, (limit - max_y).max(-min_y)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(points: &[(f64, f64)]) -> Vec<Node> {
        points
            .iter()
            .map(|p| (Uuid::new_v4(), Rect::new(p.0, p.1, 100., 50.)))
            .collect()
    }
    fn at(moves: &Moves, id: Uuid) -> [f64; 2] {
        moves.iter().find(|m| m.0 == id).unwrap().1
    }

    #[test]
    fn canvas_003_align_uses_selection_bounds() {
        let n = nodes(&[(10., 0.), (50., 100.), (200., 40.)]);
        let left = align(&n, Align::Left);
        assert!(left.iter().all(|m| m.1[0] == 10.));
        assert_eq!(at(&left, n[1].0)[1], 100., "the other axis is preserved");
        let right = align(&n, Align::Right);
        assert!(right.iter().all(|m| m.1[0] == 200.));
        let center = align(&n, Align::CenterX);
        assert!(center.iter().all(|m| m.1[0] == 105.));
        let bottom = align(&n, Align::Bottom);
        assert!(bottom.iter().all(|m| m.1[1] == 100.));
        let middle = align(&n, Align::CenterY);
        assert!(middle.iter().all(|m| m.1[1] == 50.));
        assert!(align(&n[..1], Align::Top).is_empty());
    }

    #[test]
    fn canvas_003_distribute_keeps_ends_and_equalises_gaps() {
        let n = nodes(&[(0., 0.), (130., 9.), (400., 3.)]);
        let moved = distribute(&n, Axis::Horizontal);
        assert_eq!(at(&moved, n[0].0), [0., 0.]);
        assert_eq!(at(&moved, n[2].0), [400., 3.]);
        assert_eq!(at(&moved, n[1].0), [200., 9.]);
        assert!(distribute(&n[..2], Axis::Vertical).is_empty());
        let stacked = nodes(&[(0., 0.), (0., 0.), (0., 0.)]);
        for m in distribute(&stacked, Axis::Vertical) {
            assert!(m.1[1].is_finite());
        }
    }

    #[test]
    fn canvas_003_tidy_preserves_rows_and_reading_order() {
        let n = nodes(&[(300., 5.), (0., 0.), (20., 120.)]);
        let moved = tidy(&n);
        assert_eq!(at(&moved, n[1].0), [0., 0.]);
        assert_eq!(at(&moved, n[0].0), [132., 0.]);
        assert_eq!(at(&moved, n[2].0), [0., 82.]);
    }

    #[test]
    fn canvas_003_flow_reads_left_to_right_and_survives_cycles() {
        let n = nodes(&[(0., 0.), (0., 100.), (0., 200.)]);
        let (a, b, c) = (n[0].0, n[1].0, n[2].0);
        let chain = flow(&n, &[(a, b), (b, c)]);
        assert!(at(&chain, a)[0] < at(&chain, b)[0] && at(&chain, b)[0] < at(&chain, c)[0]);
        let cycle = flow(&n, &[(a, b), (b, c), (c, a)]);
        assert_eq!(cycle.len(), 3);
        assert!(at(&cycle, a)[0] < at(&cycle, c)[0]);
        let outside = flow(&n, &[(a, Uuid::new_v4())]);
        assert_eq!(outside.len(), 3, "edges outside the selection are ignored");
        assert_eq!(at(&outside, a)[0], at(&outside, c)[0]);
    }

    #[test]
    fn canvas_003_snap_prefers_the_closest_line_and_reports_guides() {
        let other = Rect::new(0., 0., 100., 50.);
        let (dx, dy, guides) = snap(Rect::new(3., 300., 100., 50.), &[other], 6.);
        assert_eq!((dx, dy), (-3., 0.));
        assert!(guides.contains(&Guide::Vertical {
            x: 0.,
            y0: 0.,
            y1: 350.
        }));
        let (dx, dy, guides) = snap(Rect::new(40., 300., 100., 50.), &[other], 6.);
        assert_eq!((dx, dy), (0., 0.));
        assert!(guides.is_empty());
    }

    #[test]
    fn canvas_003_group_clamp_keeps_relative_positions() {
        let origins = [[10., 10.], [500., 40.]];
        assert_eq!(clamp_delta(&origins, (-50., -50.), 1000.), (-10., -10.));
        assert_eq!(clamp_delta(&origins, (900., 0.), 1000.), (500., 0.));
    }
}
