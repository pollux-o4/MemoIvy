import type { Rect } from './motion';

export type SnapRect = Rect;
export type SnapGuide = { axis: 'x'; at: number; from: number; to: number } | { axis: 'y'; at: number; from: number; to: number };

/** 화면에서 이 거리(px) 안이면 다른 카드의 가장자리·중심에 붙인다. GTK SNAP_PIXELS와 같다. */
export const SNAP_PIXELS = 6;
const SAME = 0.01;

const columns = (r: SnapRect) => [r.x, r.x + r.w / 2, r.x + r.w];
const rows = (r: SnapRect) => [r.y, r.y + r.h / 2, r.y + r.h];

function nearest(own: number[], others: number[][], threshold: number): number {
  let found: number | undefined;
  for (const other of others) for (const a of own) for (const b of other) {
    const delta = b - a;
    if (Math.abs(delta) <= threshold && (found === undefined || Math.abs(delta) < Math.abs(found))) found = delta;
  }
  return found ?? 0;
}

function span(rects: SnapRect[], axis: 'x' | 'y'): [number, number] {
  const start = Math.min(...rects.map((r) => axis === 'x' ? r.y : r.x));
  const end = Math.max(...rects.map((r) => axis === 'x' ? r.y + r.h : r.x + r.w));
  return [start, end];
}

/** 가장 먼저 맞닿은 선 하나만 안내선으로 남긴다(축마다 최대 1개). */
function guideFor(axis: 'x' | 'y', snapped: SnapRect, others: SnapRect[]): SnapGuide | undefined {
  const lines = axis === 'x' ? columns : rows;
  for (const at of lines(snapped)) {
    const matching = others.filter((r) => lines(r).some((line) => Math.abs(line - at) < SAME));
    if (!matching.length) continue;
    const [from, to] = span([...matching, snapped], axis);
    return { axis, at, from, to };
  }
  return undefined;
}

/**
 * 움직이는 묶음(moving)을 다른 카드의 왼쪽·중심·오른쪽(위·가운데·아래) 선에 붙인다.
 * threshold는 월드 단위(SNAP_PIXELS / zoom). 반환한 dx·dy를 끌기 이동량에 더한다.
 */
export function snapRect(moving: SnapRect, others: SnapRect[], threshold: number): { dx: number; dy: number; guides: SnapGuide[] } {
  const dx = nearest(columns(moving), others.map(columns), threshold);
  const dy = nearest(rows(moving), others.map(rows), threshold);
  const snapped = { ...moving, x: moving.x + dx, y: moving.y + dy };
  const guides = [guideFor('x', snapped, others), guideFor('y', snapped, others)].filter((guide): guide is SnapGuide => Boolean(guide));
  return { dx, dy, guides };
}
