import canvasBounds from '../../../../desktop/canvas_bounds.json';

export type CanvasPoint = { x: number; y: number };
export const CARD_WIDTH = 240;
export const CARD_HEIGHT = 156;
/** 프로젝트 카드 뒤 종이 더미가 오른쪽 아래로 비낀 거리(project_fan.css .mid.resting과 같은 값). 연결점이 더미에 묻히지 않게 바깥으로 민다. */
export const PROJECT_STACK_OFFSET = 10;
/** 카드 모서리 반경(px). CSS는 --card-radius로만 읽고, 연결점은 이 안쪽으로 들어가지 않는다. */
export const CARD_RADIUS = 10;
/** 저장 가능한 좌표 절댓값 상한. 출처는 desktop/canvas_bounds.json(IPC 검증·엔진 LIMIT와 공유). 범위 밖 위치는 묶음 변경 전체가 거절된다. */
export const CANVAS_LIMIT: number = canvasBounds.limit;
/** 저장 가능한 좌표 하한. 보드는 원점 위·왼쪽으로도 열려 있다. */
export const CANVAS_MIN = -CANVAS_LIMIT;
/** 캔버스 위 카드가 차지하는 사각형. 카드 치수의 원본(CARD_WIDTH·CARD_HEIGHT)과 함께 둔다. */
export const cardRect = (point: CanvasPoint) => ({ x: point.x, y: point.y, w: CARD_WIDTH, h: CARD_HEIGHT });
export function canvasSlot(index: number): CanvasPoint {
  return { x: 64 + ((index % 4 + Math.floor(index / 336) * 4) % 60) * 320, y: 64 + (Math.floor(index / 4) % 84) * 232 };
}
/** 점들의 경계. 한 번 훑어 O(n)으로 구하고, 전개 인자(Math.min(...))의 호출 스택 한도를 피한다. 빈 목록은 ±Infinity. */
export function pointBounds(points: readonly CanvasPoint[]) {
  let minX = Infinity; let maxX = -Infinity; let minY = Infinity; let maxY = -Infinity;
  for (const { x, y } of points) {
    if (x < minX) minX = x; if (x > maxX) maxX = x;
    if (y < minY) minY = y; if (y > maxY) maxY = y;
  }
  return { minX, maxX, minY, maxY };
}
export function clampCanvasDelta(points: CanvasPoint[], dx: number, dy: number): CanvasPoint {
  if (!points.length) return { x: dx, y: dy };
  const { minX, maxX, minY, maxY } = pointBounds(points);
  return { x: Math.min(Math.max(dx, CANVAS_MIN - minX), Math.max(CANVAS_MIN - minX, CANVAS_LIMIT - maxX)), y: Math.min(Math.max(dy, CANVAS_MIN - minY), Math.max(CANVAS_MIN - minY, CANVAS_LIMIT - maxY)) };
}
