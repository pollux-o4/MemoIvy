import type { CanvasPoint } from '../model/align';

/** 범위 선택 상자(보드 화면 좌표). */
export function SelectionBox({ a, b }: { a: CanvasPoint; b: CanvasPoint }) {
  return <div className="selection-box" style={{ left: Math.min(a.x, b.x), top: Math.min(a.y, b.y), width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y) }}/>;
}
