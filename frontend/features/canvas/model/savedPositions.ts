import { CANVAS_LIMIT, CANVAS_MIN, type CanvasPoint } from './geometry';

const clampAxis = (value: number) => Math.min(Math.max(Math.round(value), CANVAS_MIN), CANVAS_LIMIT);
/** 저장 좌표는 CANVAS_MIN..CANVAS_LIMIT 정수 픽셀. 저장 직전 반올림·범위 맞춤의 단일 출처. */
export const savedPoint = (point: CanvasPoint): [number, number] => [clampAxis(point.x), clampAxis(point.y)];
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

/**
 * 저장 요청이 끝나면(성공·실패 모두) 그 요청이 덮었던 미리보기 위치를 걷어 저장된 위치를 다시 원본으로 삼는다.
 * 그 사이 새로 만든 미리보기 객체는 같은 위치라도 남긴다(더 새 미리보기를 지우지 않게).
 */
export function settleDraft<T extends Record<string, CanvasPoint>>(draft: T, sent: Record<string, CanvasPoint>): T {
  const next = { ...draft };
  for (const [id, point] of Object.entries(sent)) if (next[id] === point) delete next[id];
  return next;
}
