export type CanvasPoint = { x: number; y: number };
export const CARD_WIDTH = 240;
export const CARD_HEIGHT = 156;
/** 캔버스 위 카드가 차지하는 사각형. 카드 치수의 원본(CARD_WIDTH·CARD_HEIGHT)과 함께 둔다. */
/** 저장 좌표는 정수 픽셀. 저장 직전 반올림의 단일 출처. */
export const savedPoint = (point: CanvasPoint): [number, number] => [Math.round(point.x), Math.round(point.y)];
/**
 * 저장할 위치 변경만 고른다. 비교 기준은 저장된 위치다(끌기 미리보기가 섞인 화면 위치와 비교하면 늘 같아 저장이 빠진다).
 * 저장된 위치가 없는 카드(자동 배치)는 놓은 자리를 새로 저장한다.
 */
export function changedPositions(next: Record<string, CanvasPoint>, saved: Record<string, [number, number] | null | undefined>): Array<{ id: string; position: [number, number] }> {
  return Object.entries(next).flatMap(([id, point]) => {
    if (!(id in saved)) return [];
    const position = savedPoint(point); const before = saved[id];
    return before && before[0] === position[0] && before[1] === position[1] ? [] : [{ id, position }];
  });
}
export const cardRect = (point: CanvasPoint) => ({ x: point.x, y: point.y, w: CARD_WIDTH, h: CARD_HEIGHT });
export function canvasSlot(index: number): CanvasPoint {
  return { x: 64 + ((index % 4 + Math.floor(index / 336) * 4) % 60) * 320, y: 64 + (Math.floor(index / 4) % 84) * 232 };
}
export function clampCanvasDelta(points: CanvasPoint[], dx: number, dy: number): CanvasPoint {
  if (!points.length) return { x: dx, y: dy };
  const minX = Math.min(...points.map((point) => point.x)); const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y)); const maxY = Math.max(...points.map((point) => point.y));
  return { x: Math.min(Math.max(dx, -minX), Math.max(-minX, 20_000 - maxX)), y: Math.min(Math.max(dy, -minY), Math.max(-minY, 20_000 - maxY)) };
}
export type Alignment = 'left' | 'center' | 'right' | 'top' | 'middle' | 'bottom' | 'distribute-x' | 'distribute-y' | 'grid' | 'flow';

export function vacantGroup(ids: string[], positions: Record<string, CanvasPoint>, occupied: CanvasPoint[]): Record<string, CanvasPoint> {
  const members = ids.flatMap((id) => positions[id] ? [[id, positions[id]] as const] : []);
  if (!members.length) return {};
  const left = Math.min(...members.map(([, point]) => point.x)); const top = Math.min(...members.map(([, point]) => point.y));
  for (let index = 0; index < 5040; index++) {
    const origin = { x: 64 + (index % 60) * 320, y: 64 + Math.floor(index / 60) * 232 };
    const moved = members.map(([, point]) => ({ x: origin.x + point.x - left, y: origin.y + point.y - top }));
    const collision = moved.some((point) => occupied.some((other) => Math.abs(point.x - other.x) < CARD_WIDTH + 24 && Math.abs(point.y - other.y) < CARD_HEIGHT + 24));
    if (!collision) return Object.fromEntries(members.map(([id, point]) => [id, { x: origin.x + point.x - left, y: origin.y + point.y - top }]));
  }
  return Object.fromEntries(members.map(([id, point]) => [id, { x: Math.max(64, point.x), y: Math.max(64, point.y) }]));
}

export function alignPositions(ids: string[], positions: Record<string, CanvasPoint>, kind: Alignment, edges: Array<[string, string]> = []): Record<string, CanvasPoint> {
  if (ids.length < 2) return {};
  const points = ids.map((id) => positions[id]).filter((point): point is CanvasPoint => Boolean(point));
  if (points.length < 2) return {};
  const minX = Math.min(...points.map((point) => point.x));
  const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y));
  const maxY = Math.max(...points.map((point) => point.y));
  if (kind === 'distribute-x' || kind === 'distribute-y') {
    if (ids.length < 3) return {};
    const horizontal = kind === 'distribute-x';
    const sorted = ids.filter((id) => positions[id]).sort((a, b) => (horizontal ? positions[a].x - positions[b].x : positions[a].y - positions[b].y));
    const first = horizontal ? positions[sorted[0]].x : positions[sorted[0]].y;
    const last = horizontal ? positions[sorted.at(-1)!].x : positions[sorted.at(-1)!].y;
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
      const top = stack.at(-1)!; const target = outgoing[top[0]][top[1]];
      if (target === undefined) { marks[top[0]] = 2; stack.pop(); continue; }
      top[1] += 1;
      if (marks[target] === 1) continue;
      forward.push([top[0], target]);
      if (!marks[target]) { marks[target] = 1; stack.push([target, 0]); }
    }
  }
  const rank = order.map(() => 0);
  for (let round = 0; round < order.length; round++) for (const [from, to] of forward) rank[to] = Math.max(rank[to], rank[from] + 1);
  const columns: number[][] = Array.from({ length: Math.max(...rank) + 1 }, () => []);
  rank.forEach((value, index) => columns[value].push(index));
  const row = order.map(() => 0);
  for (const column of columns) {
    column.sort((a, b) => {
      const parents = (i: number) => forward.filter((edge) => edge[1] === i).map((edge) => row[edge[0]]);
      const aa = parents(a); const bb = parents(b);
      return (aa.length ? aa.reduce((s, n) => s + n, 0) / aa.length : a) - (bb.length ? bb.reduce((s, n) => s + n, 0) / bb.length : b) || a - b;
    });
    column.forEach((index, position) => { row[index] = position; });
  }
  const left = Math.min(...ids.map((id) => positions[id].x)); const top = Math.min(...ids.map((id) => positions[id].y));
  const tallest = Math.max(...columns.map((column) => column.length));
  return Object.fromEntries(columns.flatMap((column, c) => column.map((index, r) => [order[index], {
    x: left + c * (CARD_WIDTH + 96), y: top + (tallest - column.length) * (CARD_HEIGHT + 32) / 2 + r * (CARD_HEIGHT + 32),
  }])));
}
