import { anchorOnSide, chooseSides } from './anchors';
import type { CanvasPoint, EdgeAnchors, Rect, Side } from './types';

const NORMALS: Record<Side, CanvasPoint> = { left: { x: -1, y: 0 }, right: { x: 1, y: 0 }, top: { x: 0, y: -1 }, bottom: { x: 0, y: 1 } };

/**
 * 끌어서 잇는 중의 미리보기 선 연결점. 놓을 카드가 있으면 실제 간선과 같은 변 규칙을, 없으면 포인터 쪽 변에서 포인터로 잇는다.
 * 실제 선과 같은 PathStrategy에 넘겨 놓기 전후 모양이 같게 한다.
 */
export function previewAnchors(from: Rect, to: CanvasPoint | Rect): EdgeAnchors {
  if ('w' in to) {
    const sides = chooseSides(from, to);
    return { key: 'preview', from: anchorOnSide(from, sides.from, 0.5), to: anchorOnSide(to, sides.to, 0.5), bend: 0 };
  }
  const sides = chooseSides(from, { x: to.x, y: to.y, w: 0, h: 0 });
  const side = sides.to;
  return { key: 'preview', from: anchorOnSide(from, sides.from, 0.5), to: { side, point: to, normal: NORMALS[side] }, bend: 0 };
}
