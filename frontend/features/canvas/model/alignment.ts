import { CARD_HEIGHT, CARD_WIDTH, type CanvasPoint, pointBounds, SLOT_GRID, slotAt } from './geometry';

export type Alignment = 'left' | 'center' | 'right' | 'top' | 'middle' | 'bottom' | 'distribute-x' | 'distribute-y' | 'grid' | 'flow' | 'mindmap';

export function vacantGroup(ids: string[], positions: Record<string, CanvasPoint>, occupied: CanvasPoint[]): Record<string, CanvasPoint> {
  const members = ids.flatMap((id) => positions[id] ? [[id, positions[id]] as const] : []);
  if (!members.length) return {};
  const { minX: left, minY: top } = pointBounds(members.map(([, point]) => point));
  const collides = occupiedCollision(occupied);
  // 격자 칸을 줄 단위로 훑어 처음 비는 자리에 묶음 왼쪽 위를 놓는다.
  for (let index = 0; index < SLOT_GRID.columns * SLOT_GRID.rows; index++) {
    const origin = slotAt(index % SLOT_GRID.columns, Math.floor(index / SLOT_GRID.columns));
    const collision = members.some(([, point]) => collides({ x: origin.x + point.x - left, y: origin.y + point.y - top }));
    if (!collision) return Object.fromEntries(members.map(([id, point]) => [id, { x: origin.x + point.x - left, y: origin.y + point.y - top }]));
  }
  return Object.fromEntries(members.map(([id, point]) => [id, { x: Math.max(SLOT_GRID.margin, point.x), y: Math.max(SLOT_GRID.margin, point.y) }]));
}

/** 붙여 놓을 묶음이 기존 카드와 띄울 최소 간격. */
const PLACEMENT_GAP = 24;

/** 차지한 카드를 (카드+간격) 크기 격자에 담아, 한 점의 충돌을 이웃 9칸만 보고 O(1)에 가깝게 판정한다. */
function occupiedCollision(occupied: readonly CanvasPoint[]): (point: CanvasPoint) => boolean {
  const width = CARD_WIDTH + PLACEMENT_GAP; const height = CARD_HEIGHT + PLACEMENT_GAP;
  const cells = new Map<string, CanvasPoint[]>();
  const key = (x: number, y: number) => `${x},${y}`;
  for (const point of occupied) {
    const cell = key(Math.floor(point.x / width), Math.floor(point.y / height));
    const bucket = cells.get(cell);
    if (bucket) bucket.push(point);
    else cells.set(cell, [point]);
  }
  return (point) => {
    const x = Math.floor(point.x / width); const y = Math.floor(point.y / height);
    for (let dx = -1; dx <= 1; dx++) for (let dy = -1; dy <= 1; dy++) {
      const bucket = cells.get(key(x + dx, y + dy));
      if (bucket?.some((other) => Math.abs(point.x - other.x) < width && Math.abs(point.y - other.y) < height)) return true;
    }
    return false;
  };
}

export function alignPositions(ids: string[], positions: Record<string, CanvasPoint>, kind: Alignment, edges: Array<[string, string]> = []): Record<string, CanvasPoint> {
  if (ids.length < 2) return {};
  const points = ids.map((id) => positions[id]).filter((point): point is CanvasPoint => Boolean(point));
  if (points.length < 2) return {};
  const { minX, maxX, minY, maxY } = pointBounds(points);
  if (kind === 'distribute-x' || kind === 'distribute-y') {
    if (ids.length < 3) return {};
    const horizontal = kind === 'distribute-x';
    const sorted = ids.filter((id) => positions[id]).sort((a, b) => (horizontal ? positions[a].x - positions[b].x : positions[a].y - positions[b].y));
    const first = horizontal ? positions[sorted[0]].x : positions[sorted[0]].y;
    const last = horizontal ? positions[sorted[sorted.length - 1]].x : positions[sorted[sorted.length - 1]].y;
    const size = horizontal ? CARD_WIDTH : CARD_HEIGHT;
    const gap = (last + size - first - sorted.length * size) / (sorted.length - 1);
    let cursor = first;
    return Object.fromEntries(sorted.map((id) => {
      const point = positions[id]; const next = horizontal ? { x: cursor, y: point.y } : { x: point.x, y: cursor };
      cursor += size + gap;
      return [id, next];
    }));
  }
  if (kind === 'flow') return flowPositions(ids, positions, edges);
  const ordered = [...ids].sort((a, b) => positions[a].y - positions[b].y || positions[a].x - positions[b].x);
  const rows: string[][] = [];
  for (const id of ordered) {
    const point = positions[id]; const row = rows.at(-1);
    if (row) {
      const center = row.reduce((sum, item) => sum + positions[item].y + CARD_HEIGHT / 2, 0) / row.length;
      if (Math.abs(point.y + CARD_HEIGHT / 2 - center) < CARD_HEIGHT / 2) { row.push(id); continue; }
    }
    rows.push([id]);
  }
  if (kind === 'grid') {
    const cellWidth = CARD_WIDTH + 32;
    const result: Record<string, CanvasPoint> = {};
    let y = minY;
    for (const row of rows) {
      row.sort((a, b) => positions[a].x - positions[b].x).forEach((id, column) => { result[id] = { x: minX + column * cellWidth, y }; });
      y += CARD_HEIGHT + 32;
    }
    return result;
  }
  return Object.fromEntries(ids.flatMap((id) => {
    const point = positions[id];
    if (!point) return [];
    const next = kind === 'left' ? { ...point, x: minX }
      : kind === 'right' ? { ...point, x: maxX }
        : kind === 'center' ? { ...point, x: (minX + maxX) / 2 }
          : kind === 'top' ? { ...point, y: minY }
            : kind === 'bottom' ? { ...point, y: maxY }
              : { ...point, y: (minY + maxY) / 2 };
    return [[id, next]];
  }));
}

function flowPositions(ids: string[], positions: Record<string, CanvasPoint>, edges: Array<[string, string]>): Record<string, CanvasPoint> {
  if (ids.length < 2) return {};
  const order = [...ids].sort((a, b) => positions[a].y - positions[b].y || positions[a].x - positions[b].x);
  const indexes = new Map(order.map((id, index) => [id, index]));
  const outgoing = order.map(() => [] as number[]);
  for (const [from, to] of edges) { const a = indexes.get(from); const b = indexes.get(to); if (a !== undefined && b !== undefined && a !== b) outgoing[a].push(b); }
  const marks = order.map(() => 0); const forward: Array<[number, number]> = [];
  for (let root = 0; root < order.length; root++) {
    if (marks[root]) continue;
    marks[root] = 1; const stack: Array<[number, number]> = [[root, 0]];
    while (stack.length) {
      const top = stack[stack.length - 1]; const target = outgoing[top[0]][top[1]];
      if (target === undefined) { marks[top[0]] = 2; stack.pop(); continue; }
      top[1] += 1;
      if (marks[target] === 1) continue;
      forward.push([top[0], target]);
      if (!marks[target]) { marks[target] = 1; stack.push([target, 0]); }
    }
  }
  const rank = order.map(() => 0);
  const children = order.map(() => [] as number[]);
  const parents = order.map(() => [] as number[]);
  const indegree = order.map(() => 0);
  for (const [from, to] of forward) {
    children[from].push(to);
    parents[to].push(from);
    indegree[to] += 1;
  }
  const queue = order.flatMap((_, index) => indegree[index] === 0 ? [index] : []);
  for (let head = 0; head < queue.length; head++) {
    const from = queue[head];
    for (const to of children[from]) {
      rank[to] = Math.max(rank[to], rank[from] + 1);
      indegree[to] -= 1;
      if (indegree[to] === 0) queue.push(to);
    }
  }
  let maxRank = -Infinity;
  for (const value of rank) maxRank = Math.max(maxRank, value);
  const columns: number[][] = Array.from({ length: maxRank + 1 }, () => []);
  rank.forEach((value, index) => { columns[value].push(index); });
  const row = order.map(() => 0);
  for (const column of columns) {
    const keys = new Map(column.map((index) => [index, parents[index].length
      ? parents[index].reduce((sum, parent) => sum + row[parent], 0) / parents[index].length : index]));
    column.sort((a, b) => (keys.get(a) ?? a) - (keys.get(b) ?? b) || a - b);
    column.forEach((index, position) => { row[index] = position; });
  }
  const { minX: left, minY: top } = pointBounds(ids.map((id) => positions[id]));
  let tallest = -Infinity;
  for (const column of columns) tallest = Math.max(tallest, column.length);
  return Object.fromEntries(columns.flatMap((column, c) => column.map((index, r) => [order[index], {
    x: left + c * (CARD_WIDTH + 96), y: top + (tallest - column.length) * (CARD_HEIGHT + 32) / 2 + r * (CARD_HEIGHT + 32),
  }])));
}
