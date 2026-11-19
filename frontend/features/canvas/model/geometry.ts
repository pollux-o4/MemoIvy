export type CanvasPoint = { x: number; y: number };
export const CARD_WIDTH = 240;
export const CARD_HEIGHT = 156;
/** 카드 모서리 반경(px). CSS는 --card-radius로만 읽고, 연결점은 이 안쪽으로 들어가지 않는다. */
export const CARD_RADIUS = 10;
/** 저장 가능한 좌표 범위 상한(엔진 persistence::LIMIT와 같다). 범위 밖 위치는 엔진이 묶음 변경 전체를 거절한다. */
export const CANVAS_LIMIT = 20_000;
/** 캔버스 위 카드가 차지하는 사각형. 카드 치수의 원본(CARD_WIDTH·CARD_HEIGHT)과 함께 둔다. */
export const cardRect = (point: CanvasPoint) => ({ x: point.x, y: point.y, w: CARD_WIDTH, h: CARD_HEIGHT });
export function canvasSlot(index: number): CanvasPoint {
  return { x: 64 + ((index % 4 + Math.floor(index / 336) * 4) % 60) * 320, y: 64 + (Math.floor(index / 4) % 84) * 232 };
}
export function clampCanvasDelta(points: CanvasPoint[], dx: number, dy: number): CanvasPoint {
  if (!points.length) return { x: dx, y: dy };
  const minX = Math.min(...points.map((point) => point.x)); const maxX = Math.max(...points.map((point) => point.x));
  const minY = Math.min(...points.map((point) => point.y)); const maxY = Math.max(...points.map((point) => point.y));
  return { x: Math.min(Math.max(dx, -minX), Math.max(-minX, CANVAS_LIMIT - maxX)), y: Math.min(Math.max(dy, -minY), Math.max(-minY, CANVAS_LIMIT - maxY)) };
}
