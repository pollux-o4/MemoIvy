import type { Camera, Rect } from '../motion';

export type { Rect };

/** 카드와 보이는 영역 가장자리 사이에 남길 화면 px. */
const REVEAL_MARGIN = 16;

/**
 * 보드에서 겹쳐 뜬 패널(인스펙터 등)을 뺀 실제 보이는 영역(보드 좌표 px).
 * 패널이 한쪽 가장자리에 붙어 있다고 보고, 더 넓게 남는 쪽을 고른다.
 */
export function visibleArea(board: Rect, overlays: Rect[]): Rect {
  let area = { x: 0, y: 0, w: board.w, h: board.h };
  for (const o of overlays) {
    const left = Math.max(o.x - board.x, 0); const right = Math.min(o.x + o.w - board.x, board.w);
    if (right <= left || o.y + o.h <= board.y || o.y >= board.y + board.h) continue;
    const leftRoom = left - area.x; const rightRoom = area.x + area.w - right;
    area = leftRoom >= rightRoom ? { ...area, w: Math.max(leftRoom, 0) } : { ...area, x: right, w: Math.max(rightRoom, 0) };
  }
  return area;
}

/** 축 하나: [start, end]가 [lo, hi] 안에 들도록 필요한 최소 이동량(화면 px). 다 못 들어가면 앞쪽을 맞춘다. */
function shift(start: number, end: number, lo: number, hi: number): number {
  if (end - start > hi - lo) return lo - start;
  if (start < lo) return lo - start;
  if (end > hi) return hi - end;
  return 0;
}

/** 카드가 보이는 영역 안에 들도록 카메라를 최소한만 옮긴다. 이미 보이면 null(화면을 흔들지 않는다). */
export function revealCamera(target: Rect, camera: Camera, area: Rect, margin = REVEAL_MARGIN): Camera | null {
  const m = Math.min(margin, area.w / 4, area.h / 4);
  const left = (target.x - camera.x) * camera.zoom; const top = (target.y - camera.y) * camera.zoom;
  const dx = shift(left, left + target.w * camera.zoom, area.x + m, area.x + area.w - m);
  const dy = shift(top, top + target.h * camera.zoom, area.y + m, area.y + area.h - m);
  if (dx === 0 && dy === 0) return null;
  return { ...camera, x: camera.x - dx / camera.zoom, y: camera.y - dy / camera.zoom };
}
