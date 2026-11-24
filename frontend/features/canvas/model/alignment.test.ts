import assert from 'node:assert/strict';
import { test } from 'vitest';
import { alignPositions, vacantGroup } from './alignment';
import { CARD_HEIGHT, CARD_WIDTH, type CanvasPoint, pointBounds } from './geometry';

/** 같은 씨앗이면 같은 수열을 내는 선형 합동 난수. 무작위 보드를 재현 가능하게 만든다. */
const seededRandom = (initial: number) => { let seed = initial; return () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 2 ** 32; }; };

// 기존 구현을 그대로 얼려 둔 독립 기준(값을 공용 상수로 바꾸지 않는다). 기존 전체 탐색을 독립 기준으로 유지해 후보 순서·충돌 경계·최종 대체 위치를 비교한다.
function referenceVacantGroup(ids: string[], positions: Record<string, CanvasPoint>, occupied: CanvasPoint[]): Record<string, CanvasPoint> {
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

test('격자 조회는 칸·충돌 경계에서 전체 탐색과 같다', () => {
  const positions = { a: { x: -300.5, y: -180.25 }, b: { x: 20, y: 12 } };
  for (const x of [-264, 0, 264]) for (const y of [-180, 0, 180]) {
    for (const epsilon of [-0.001, 0, 0.001]) {
      const occupied = [{ x: 64 + x + epsilon, y: 64 + y + epsilon }];
      assert.deepEqual(vacantGroup(['missing', 'a', 'b'], positions, occupied), referenceVacantGroup(['missing', 'a', 'b'], positions, occupied));
    }
  }
});

test('격자 조회는 재현 가능한 무작위 보드에서 전체 탐색과 같다', () => {
  const random = seededRandom(42);
  for (let trial = 0; trial < 40; trial++) {
    const positions = Object.fromEntries(Array.from({ length: 8 }, (_, i) => [String(i), { x: random() * 1800 - 900, y: random() * 1200 - 600 }]));
    const occupied = Array.from({ length: 300 }, () => ({ x: random() * 8000 - 1000, y: random() * 4000 - 1000 }));
    const ids = Object.keys(positions);
    assert.deepEqual(vacantGroup(ids, positions, occupied), referenceVacantGroup(ids, positions, occupied));
  }
});

test('모든 칸이 차면 대체 위치를, 빈 선택이면 빈 결과를 낸다', () => {
  const occupied = Array.from({ length: 5040 }, (_, i) => ({ x: 64 + i % 60 * 320, y: 64 + Math.floor(i / 60) * 232 }));
  const positions = { a: { x: -50, y: 20 }, b: { x: 90, y: 100 } };
  assert.deepEqual(vacantGroup(['a', 'b'], positions, occupied), referenceVacantGroup(['a', 'b'], positions, occupied));
  assert.deepEqual(vacantGroup(['missing'], positions, occupied), {});
});

test('큰 묶음도 전개 인자 한도에 걸리지 않는다', () => {
  const positions = Object.fromEntries(Array.from({ length: 150000 }, (_, i) => [String(i), { x: i, y: -i }]));
  const result = vacantGroup(Object.keys(positions), positions, []);
  assert.deepEqual(result['0'], { x: 64, y: 150063 });
  assert.deepEqual(result['149999'], { x: 150063, y: 64 });
});

test('흐름 배치는 순환을 포함한 무작위 그래프 40개에서 기존 구현과 같다', () => {
  function referenceFlowPositions(ids: string[], positions: Record<string, CanvasPoint>, edges: Array<[string, string]>): Record<string, CanvasPoint> {
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
    for (let round = 0; round < order.length; round++) for (const [from, to] of forward) rank[to] = Math.max(rank[to], rank[from] + 1);
    let maxRank = -Infinity;
    for (const value of rank) maxRank = Math.max(maxRank, value);
    const columns: number[][] = Array.from({ length: maxRank + 1 }, () => []);
    rank.forEach((value, index) => { columns[value].push(index); });
    const row = order.map(() => 0);
    for (const column of columns) {
      column.sort((a, b) => {
        const parents = (i: number) => forward.filter((edge) => edge[1] === i).map((edge) => row[edge[0]]);
        const aa = parents(a); const bb = parents(b);
        return (aa.length ? aa.reduce((s, n) => s + n, 0) / aa.length : a) - (bb.length ? bb.reduce((s, n) => s + n, 0) / bb.length : b) || a - b;
      });
      column.forEach((index, position) => { row[index] = position; });
    }
    const { minX: left, minY: top } = pointBounds(ids.map((id) => positions[id]));
    let tallest = -Infinity;
    for (const column of columns) tallest = Math.max(tallest, column.length);
    return Object.fromEntries(columns.flatMap((column, c) => column.map((index, r) => [order[index], {
      x: left + c * (CARD_WIDTH + 96), y: top + (tallest - column.length) * (CARD_HEIGHT + 32) / 2 + r * (CARD_HEIGHT + 32),
    }])));
  }

  const random = seededRandom(20261125);
  for (let trial = 0; trial < 40; trial++) {
    const count = 3 + Math.floor(random() * 28);
    const ids = Array.from({ length: count }, (_, index) => `node-${index}`);
    const positions = Object.fromEntries(ids.map((id) => [id, { x: Math.floor(random() * 20) * 32 - 320, y: Math.floor(random() * 20) * 32 - 320 }]));
    const edges: Array<[string, string]> = [];
    for (const from of ids) for (const to of ids) if (random() < (trial % 4) / 5) edges.push([from, to]);
    if (trial % 2 === 0) edges.push([ids[0], ids[1]], [ids[1], ids[2]], [ids[2], ids[0]]);
    if (trial % 3 === 0) edges.push([ids[0], ids[1]], [ids[0], ids[1]], [ids[0], ids[0]], ['missing', ids[0]], [ids[0], 'missing']);
    assert.deepEqual(alignPositions(ids, positions, 'flow', edges), referenceFlowPositions(ids, positions, edges), `trial ${trial}`);
  }
});
