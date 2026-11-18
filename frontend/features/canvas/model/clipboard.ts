import type { Note } from '../../../shared/engine_contract';
import type { CanvasPoint } from './align';

/** 복사한 카드 하나. 위치는 묶음 왼쪽 위 기준 상대값, 연결은 묶음 안 순번. */
export type ClipItem = { text: string; color: string; project: boolean; done: boolean; offset: CanvasPoint; next: number[] };
export type ClipPayload = { items: ClipItem[]; origin: CanvasPoint };

/** 붙여 넣을 때마다 이만큼 비껴 놓아 원본·이전 사본과 겹치지 않게 한다. */
export const PASTE_STEP = 24;

/** 선택한 카드를 내용·색·완료·상대 위치·묶음 안 연결과 함께 담는다. 묶음 밖으로 가는 연결은 버린다. */
export function copyNotes(ids: string[], notes: Note[], positions: Record<string, CanvasPoint>): ClipPayload | undefined {
  const picked = notes.filter((note) => ids.includes(note.id) && positions[note.id]);
  if (!picked.length) return undefined;
  const origin = { x: Math.min(...picked.map((note) => positions[note.id].x)), y: Math.min(...picked.map((note) => positions[note.id].y)) };
  const order = new Map(picked.map((note, index) => [note.id, index]));
  return {
    origin,
    items: picked.map((note) => ({
      text: note.text, color: note.color, project: Boolean(note.canvas.project), done: note.canvas.done,
      offset: { x: positions[note.id].x - origin.x, y: positions[note.id].y - origin.y },
      next: note.canvas.next.flatMap((target) => order.has(target) ? [order.get(target) as number] : []),
    })),
  };
}

/** n번째 붙여넣기 위치: 원래 자리에서 PASTE_STEP씩 오른쪽 아래로. */
export function pastePositions(payload: ClipPayload, round: number): CanvasPoint[] {
  const shift = PASTE_STEP * Math.max(1, round);
  return payload.items.map((item) => ({ x: payload.origin.x + item.offset.x + shift, y: payload.origin.y + item.offset.y + shift }));
}

/** 다른 앱에 붙여 넣을 평문: 카드 본문을 빈 줄로 잇는다. */
export function plainText(payload: ClipPayload): string {
  return payload.items.map((item) => item.text.trim()).filter(Boolean).join('\n\n');
}
