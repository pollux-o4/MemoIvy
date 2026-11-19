import { type CanvasPoint, cardRect, clampCanvasDelta } from './align';
import { bounds } from './motion';
import { SNAP_PIXELS, type SnapGuide, snapRect } from './snap';

/** 관성 속도를 잴 최근 포인터 표본 구간(ms). */
const COAST_WINDOW_MS = 120;
export type PointerSample = { x: number; y: number; at: number };

/**
 * 끌고 있는 카드 묶음의 이동량: 캔버스 범위로 막고, 다른 카드 선에 붙인다(snapOff면 붙이지 않음).
 * origins는 끌기 시작 위치, others는 끌지 않는 카드 위치. 붙은 선을 guides로 돌려준다.
 */
export function dragDelta(origins: CanvasPoint[], others: CanvasPoint[], dx: number, dy: number, zoom: number, snapOff: boolean): { delta: CanvasPoint; guides: SnapGuide[] } {
  const raw = clampCanvasDelta(origins, dx, dy);
  if (snapOff || !origins.length) return { delta: raw, guides: [] };
  const moving = bounds(origins.map((point) => cardRect({ x: point.x + raw.x, y: point.y + raw.y })));
  if (!moving) return { delta: raw, guides: [] };
  const fit = snapRect(moving, others.map(cardRect), SNAP_PIXELS / zoom);
  return { delta: clampCanvasDelta(origins, raw.x + fit.dx, raw.y + fit.dy), guides: fit.guides };
}

/** 최근 표본 구간에 새 표본을 더한다(오래된 표본은 버린다). */
export const keepRecent = (samples: PointerSample[], next: PointerSample): PointerSample[] =>
  [...samples.filter((sample) => next.at - sample.at <= COAST_WINDOW_MS), next];

/** 표본 처음과 끝으로 놓을 때 속도(px/s). 표본이 모자라거나 시간이 흐르지 않았으면 null. */
export function releaseVelocity(samples: PointerSample[]): [number, number] | null {
  if (samples.length < 2) return null;
  const first = samples[0]; const last = samples[samples.length - 1]; const elapsed = (last.at - first.at) / 1000;
  return elapsed > 0 ? [(last.x - first.x) / elapsed, (last.y - first.y) / elapsed] : null;
}
