import {
  ARROW_LENGTH,
  type Anchor,
  type CanvasPoint,
  type EdgeAnchors,
  type LinkStyle,
  type PathStrategy,
} from './types';

const ORTHOGONAL_OFFSET = 20;
const ORTHOGONAL_RADIUS = 8;

/** 숫자를 소수 첫째 자리까지 반올림한다(-0 방지). */
const round1 = (value: number): number => Math.round(value * 10) / 10 || 0;

/** 좌표를 SVG path 문자열 조각으로 바꾼다. */
const fmt = (point: CanvasPoint): string => `${round1(point.x)} ${round1(point.y)}`;

/** 두 점 사이의 거리와 단위 수직 벡터를 구한다. */
function segmentGeometry(start: CanvasPoint, end: CanvasPoint): { dist: number; perp: CanvasPoint } {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const dist = Math.hypot(dx, dy);
  if (dist === 0) return { dist: 0, perp: { x: 0, y: 0 } };
  return { dist, perp: { x: -dy / dist, y: dx / dist } };
}

/** 도착 연결점을 normal 방향(카드 밖)으로 length 만큼 뺀 점을 반환한다. */
export function trimEnd(anchor: Anchor, length = ARROW_LENGTH): CanvasPoint {
  return {
    x: anchor.point.x + anchor.normal.x * length,
    y: anchor.point.y + anchor.normal.y * length,
  };
}

/** 3차 베지어 곡선 SVG path를 만든다(React Flow getBezierPath 방식). */
export const curvePath: PathStrategy = ({ from, to, bend }: EdgeAnchors): string => {
  const start = from.point;
  const end = trimEnd(to);
  const { dist, perp } = segmentGeometry(start, end);
  const k = Math.max(40, dist * 0.4);
  const c1: CanvasPoint = {
    x: start.x + from.normal.x * k + perp.x * bend,
    y: start.y + from.normal.y * k + perp.y * bend,
  };
  const c2: CanvasPoint = {
    x: end.x + to.normal.x * k + perp.x * bend,
    y: end.y + to.normal.y * k + perp.y * bend,
  };
  return `M ${fmt(start)} C ${fmt(c1)}, ${fmt(c2)}, ${fmt(end)}`;
};

/** 직선 SVG path를 만든다. bend가 0이 아니면 중간점을 수직으로 민 2차 곡선을 만든다. */
export const straightPath: PathStrategy = ({ from, to, bend }: EdgeAnchors): string => {
  const start = from.point;
  const end = trimEnd(to);
  if (bend === 0) {
    return `M ${fmt(start)} L ${fmt(end)}`;
  }
  const { perp } = segmentGeometry(start, end);
  const mid: CanvasPoint = {
    x: (start.x + end.x) / 2 + perp.x * bend,
    y: (start.y + end.y) / 2 + perp.y * bend,
  };
  return `M ${fmt(start)} Q ${fmt(mid)} ${fmt(end)}`;
};

/** 두 점이 같은 위치인지 비교한다. */
const samePoint = (a: CanvasPoint, b: CanvasPoint): boolean =>
  Math.abs(a.x - b.x) < 1e-6 && Math.abs(a.y - b.y) < 1e-6;

/** 중간 점 b가 a-c 선분 위에 같은 방향으로 놓여 불필요한지 판별한다. */
const isCollinearBetween = (a: CanvasPoint, b: CanvasPoint, c: CanvasPoint): boolean => {
  const sameX = Math.abs(a.x - b.x) < 1e-6 && Math.abs(b.x - c.x) < 1e-6;
  const sameY = Math.abs(a.y - b.y) < 1e-6 && Math.abs(b.y - c.y) < 1e-6;
  if (sameX) return (b.y - a.y) * (c.y - b.y) >= 0;
  if (sameY) return (b.x - a.x) * (c.x - b.x) >= 0;
  return false;
};

/** 직교 경로의 꺾임 꼭짓점 목록을 계산한다. */
function orthogonalWaypoints(
  start: CanvasPoint,
  end: CanvasPoint,
  from: Anchor,
  to: Anchor,
  bend: number,
): CanvasPoint[] {
  const p1: CanvasPoint = {
    x: start.x + from.normal.x * ORTHOGONAL_OFFSET,
    y: start.y + from.normal.y * ORTHOGONAL_OFFSET,
  };
  const p2: CanvasPoint = {
    x: end.x + to.normal.x * ORTHOGONAL_OFFSET,
    y: end.y + to.normal.y * ORTHOGONAL_OFFSET,
  };

  const fromHoriz = from.normal.x !== 0;
  const toHoriz = to.normal.x !== 0;

  if (fromHoriz && toHoriz) {
    if (from.normal.x === to.normal.x) {
      const outerX = (from.normal.x > 0 ? Math.max(p1.x, p2.x) : Math.min(p1.x, p2.x)) + bend;
      return [start, p1, { x: outerX, y: p1.y }, { x: outerX, y: p2.y }, p2, end];
    }
    if ((p2.x - p1.x) * from.normal.x >= 0) {
      const midX = (p1.x + p2.x) / 2 + bend;
      return [start, p1, { x: midX, y: p1.y }, { x: midX, y: p2.y }, p2, end];
    }
    const midY = (p1.y + p2.y) / 2 + bend;
    return [start, p1, { x: p1.x, y: midY }, { x: p2.x, y: midY }, p2, end];
  }

  if (!fromHoriz && !toHoriz) {
    if (from.normal.y === to.normal.y) {
      const outerY = (from.normal.y > 0 ? Math.max(p1.y, p2.y) : Math.min(p1.y, p2.y)) + bend;
      return [start, p1, { x: p1.x, y: outerY }, { x: p2.x, y: outerY }, p2, end];
    }
    if ((p2.y - p1.y) * from.normal.y >= 0) {
      const midY = (p1.y + p2.y) / 2 + bend;
      return [start, p1, { x: p1.x, y: midY }, { x: p2.x, y: midY }, p2, end];
    }
    const midX = (p1.x + p2.x) / 2 + bend;
    return [start, p1, { x: midX, y: p1.y }, { x: midX, y: p2.y }, p2, end];
  }

  if (fromHoriz) {
    const forwardX = (p2.x - p1.x) * from.normal.x >= 0;
    const forwardY = (p1.y - p2.y) * to.normal.y >= 0;
    if (forwardX && forwardY && bend === 0) {
      return [start, p1, { x: p2.x, y: p1.y }, p2, end];
    }
    if (!forwardX && forwardY) {
      const midY = (p1.y + p2.y) / 2 + bend;
      return [start, p1, { x: p1.x, y: midY }, { x: p2.x, y: midY }, p2, end];
    }
    const midX = (p1.x + p2.x) / 2 + bend;
    return [start, p1, { x: midX, y: p1.y }, { x: midX, y: p2.y }, p2, end];
  }

  const forwardY = (p2.y - p1.y) * from.normal.y >= 0;
  const forwardX = (p1.x - p2.x) * to.normal.x >= 0;
  if (forwardY && forwardX && bend === 0) {
    return [start, p1, { x: p1.x, y: p2.y }, p2, end];
  }
  if (!forwardY && forwardX) {
    const midX = (p1.x + p2.x) / 2 + bend;
    return [start, p1, { x: midX, y: p1.y }, { x: midX, y: p2.y }, p2, end];
  }
  const midY = (p1.y + p2.y) / 2 + bend;
  return [start, p1, { x: p1.x, y: midY }, { x: p2.x, y: midY }, p2, end];
}

/** 중복·일직선 중간점을 제거한 꼭짓점 목록을 만든다. */
function simplifyWaypoints(raw: CanvasPoint[]): CanvasPoint[] {
  const deduped: CanvasPoint[] = [];
  for (const point of raw) {
    if (deduped.length === 0 || !samePoint(deduped[deduped.length - 1], point)) {
      deduped.push(point);
    }
  }
  const simplified: CanvasPoint[] = [];
  for (const point of deduped) {
    while (
      simplified.length >= 2 &&
      isCollinearBetween(simplified[simplified.length - 2], simplified[simplified.length - 1], point)
    ) {
      simplified.pop();
    }
    simplified.push(point);
  }
  return simplified;
}

/** 직교(둥근 모서리) SVG path를 만든다(React Flow smoothstep 방식). */
export const orthogonalPath: PathStrategy = ({ from, to, bend }: EdgeAnchors): string => {
  const start = from.point;
  const end = trimEnd(to);
  // bend는 진행 방향 왼쪽 기준이라, 축 이동량으로 바꿀 때 진행 방향 부호를 곱한다(반대 방향 간선은 반대로 밀린다).
  const travel = Math.abs(end.x - start.x) >= Math.abs(end.y - start.y) ? Math.sign(end.x - start.x) : Math.sign(end.y - start.y);
  const points = simplifyWaypoints(orthogonalWaypoints(start, end, from, to, bend * (travel || 1)));
  if (points.length <= 2) {
    return `M ${fmt(start)} L ${fmt(end)}`;
  }

  const parts: string[] = [`M ${fmt(points[0])}`];
  let cursor = points[0];

  for (let i = 1; i < points.length - 1; i += 1) {
    const prev = points[i - 1];
    const curr = points[i];
    const next = points[i + 1];
    const d1 = Math.hypot(curr.x - prev.x, curr.y - prev.y);
    const d2 = Math.hypot(next.x - curr.x, next.y - curr.y);
    const radius = Math.min(ORTHOGONAL_RADIUS, d1 / 2, d2 / 2);

    if (radius <= 0) {
      parts.push(`L ${fmt(curr)}`);
      cursor = curr;
      continue;
    }

    const before: CanvasPoint = {
      x: curr.x - ((curr.x - prev.x) / d1) * radius,
      y: curr.y - ((curr.y - prev.y) / d1) * radius,
    };
    const after: CanvasPoint = {
      x: curr.x + ((next.x - curr.x) / d2) * radius,
      y: curr.y + ((next.y - curr.y) / d2) * radius,
    };

    if (!samePoint(cursor, before)) {
      parts.push(`L ${fmt(before)}`);
    }
    parts.push(`Q ${fmt(curr)} ${fmt(after)}`);
    cursor = after;
  }

  parts.push(`L ${fmt(points[points.length - 1])}`);
  return parts.join(' ');
};

/** 선 모양별 경로 생성 전략 맵. */
export const PATH_STRATEGIES: Record<LinkStyle, PathStrategy> = {
  curve: curvePath,
  straight: straightPath,
  orthogonal: orthogonalPath,
};
