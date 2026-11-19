import { ANCHOR_INSET, type Anchor, type CanvasPoint, type Edge, type EdgeAnchors, edgeKey, type Rect, type Side, type SideMemory } from './types';

const SIDE_NORMALS: Record<Side, CanvasPoint> = {
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
  top: { x: 0, y: -1 },
  bottom: { x: 0, y: 1 },
};

const HYSTERESIS_RATIO = 0.15;
const BIDIRECTIONAL_BEND = 12;

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));
const rectCenter = (rect: Rect): CanvasPoint => ({ x: rect.x + rect.w / 2, y: rect.y + rect.h / 2 });
const isLeftRight = (side: Side) => side === 'left' || side === 'right';
/** 직전 변이 여전히 상대 쪽을 보고 있는지(경계 근처에서 반대편 변을 붙잡지 않게). */
const facesTarget = (side: Side, dx: number, dy: number) => (side === 'right' && dx >= 0) || (side === 'left' && dx <= 0) || (side === 'bottom' && dy >= 0) || (side === 'top' && dy <= 0);

/** 두 카드의 중심 벡터와 대각선 기울기를 비교해 연결 변 쌍을 고른다. */
export function chooseSides(a: Rect, b: Rect, previous?: { from: Side; to: Side }): { from: Side; to: Side } {
  const dx = b.x + b.w / 2 - (a.x + a.w / 2);
  const dy = b.y + b.h / 2 - (a.y + a.h / 2);
  if (dx === 0 && dy === 0) return previous ?? { from: 'right', to: 'left' };

  const absDx = Math.abs(dx);
  const absDy = Math.abs(dy);

  if (a.w > 0 && a.h > 0) {
    const scaledY = absDy * a.w;
    const threshold = a.h * absDx;
    // 경계 기울기 ±15% 안쪽에서는 직전 변을 유지해 깜빡임을 막는다.
    const nearBoundary = scaledY >= threshold * (1 - HYSTERESIS_RATIO) && scaledY <= threshold * (1 + HYSTERESIS_RATIO);
    if (previous && nearBoundary && facesTarget(previous.from, dx, dy)) return previous;
    if (scaledY < threshold) return dx >= 0 ? { from: 'right', to: 'left' } : { from: 'left', to: 'right' };
    return dy >= 0 ? { from: 'bottom', to: 'top' } : { from: 'top', to: 'bottom' };
  }

  if (absDx > absDy) return dx >= 0 ? { from: 'right', to: 'left' } : { from: 'left', to: 'right' };
  return dy >= 0 ? { from: 'bottom', to: 'top' } : { from: 'top', to: 'bottom' };
}

/** 사각형의 한 변 위 비율 t(0~1)에 해당하는 연결점과 법선을 계산한다. */
export function anchorOnSide(rect: Rect, side: Side, t: number): Anchor {
  const vertical = isLeftRight(side);
  const start = vertical ? rect.y : rect.x;
  const length = vertical ? rect.h : rect.w;
  const clamped = clamp01(t);
  const pos = length <= 2 * ANCHOR_INSET
    ? start + length / 2
    : start + ANCHOR_INSET + (length - 2 * ANCHOR_INSET) * clamped;

  let point: CanvasPoint;
  if (side === 'left') point = { x: rect.x, y: pos };
  else if (side === 'right') point = { x: rect.x + rect.w, y: pos };
  else if (side === 'top') point = { x: pos, y: rect.y };
  else point = { x: pos, y: rect.y + rect.h };

  return { side, point, normal: { ...SIDE_NORMALS[side] } };
}

function projectT(rect: Rect, side: Side, targetCenter: CanvasPoint): number {
  if (isLeftRight(side)) return rect.h > 0 ? clamp01((targetCenter.y - rect.y) / rect.h) : 0.5;
  return rect.w > 0 ? clamp01((targetCenter.x - rect.x) / rect.w) : 0.5;
}

interface EndpointSlot {
  edgeIdx: number;
  isFrom: boolean;
  sortCoord: number;
  otherCenter: CanvasPoint;
}

/** 모든 유효 간선의 양 끝 연결점과 양방향 오프셋을 O(E log E)로 배치한다. */
export function layoutAnchors(rects: Record<string, Rect>, edges: Edge[], memory?: SideMemory): EdgeAnchors[] {
  const items: Array<{
    edge: Edge;
    key: string;
    fromRect: Rect;
    toRect: Rect;
    sides: { from: Side; to: Side };
    tFrom: number;
    tTo: number;
  }> = [];
  const validKeys = new Set<string>();
  const buckets = new Map<string, { cardRect: Rect; side: Side; endpoints: EndpointSlot[] }>();

  const pushEndpoint = (cardId: string, cardRect: Rect, side: Side, slot: EndpointSlot) => {
    const bucketKey = `${cardId}:${side}`;
    let bucket = buckets.get(bucketKey);
    if (!bucket) {
      bucket = { cardRect, side, endpoints: [] };
      buckets.set(bucketKey, bucket);
    }
    bucket.endpoints.push(slot);
  };

  for (const edge of edges) {
    if (!Object.hasOwn(rects, edge.from) || !Object.hasOwn(rects, edge.to)) continue;
    const fromRect = rects[edge.from];
    const toRect = rects[edge.to];
    const key = edgeKey(edge);
    const sides = chooseSides(fromRect, toRect, memory?.get(key));
    const edgeIdx = items.length;
    items.push({ edge, key, fromRect, toRect, sides, tFrom: 0.5, tTo: 0.5 });
    validKeys.add(key);

    const fromCenter = rectCenter(fromRect);
    const toCenter = rectCenter(toRect);
    pushEndpoint(edge.from, fromRect, sides.from, {
      edgeIdx,
      isFrom: true,
      sortCoord: isLeftRight(sides.from) ? toCenter.y : toCenter.x,
      otherCenter: toCenter,
    });
    pushEndpoint(edge.to, toRect, sides.to, {
      edgeIdx,
      isFrom: false,
      sortCoord: isLeftRight(sides.to) ? fromCenter.y : fromCenter.x,
      otherCenter: fromCenter,
    });
  }

  for (const { cardRect, side, endpoints } of buckets.values()) {
    if (endpoints.length === 1) {
      const ep = endpoints[0];
      const t = projectT(cardRect, side, ep.otherCenter);
      if (ep.isFrom) items[ep.edgeIdx].tFrom = t;
      else items[ep.edgeIdx].tTo = t;
      continue;
    }

    endpoints.sort((a, b) => a.sortCoord - b.sortCoord);
    const count = endpoints.length;
    for (let i = 0; i < count; i++) {
      const ep = endpoints[i];
      const t = (i + 1) / (count + 1);
      if (ep.isFrom) items[ep.edgeIdx].tFrom = t;
      else items[ep.edgeIdx].tTo = t;
    }
  }

  return items.map(({ edge, key, fromRect, toRect, sides, tFrom, tTo }) => {
    const bidirectional = edge.from !== edge.to && validKeys.has(`${edge.to}:${edge.from}`);
    // bend는 진행 방향 왼쪽으로 민다. 반대 방향 두 간선이 같은 값을 받으면 서로 반대편으로 벌어진다.
    const bend = bidirectional ? BIDIRECTIONAL_BEND : 0;
    return {
      key,
      from: anchorOnSide(fromRect, sides.from, tFrom),
      to: anchorOnSide(toRect, sides.to, tTo),
      bend,
    };
  });
}
