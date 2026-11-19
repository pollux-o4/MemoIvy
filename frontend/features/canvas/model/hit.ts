import type { Note } from '../../../shared/engine_contract';
import { CARD_HEIGHT, CARD_WIDTH, type CanvasPoint } from './align';

type Positions = Record<string, CanvasPoint>;

/** 캔버스 좌표가 카드 안인지. 카드 크기의 원본은 align. */
function cardContains(card: CanvasPoint, point: CanvasPoint): boolean {
  return point.x >= card.x && point.x <= card.x + CARD_WIDTH && point.y >= card.y && point.y <= card.y + CARD_HEIGHT;
}

/**
 * 점 아래의 맨 위 카드(나중에 그려진 카드 우선). accept로 후보를 거른다.
 * 끌기 중 매 이동마다 elementFromPoint(강제 레이아웃)를 부르지 않으려고 위치로 판정한다.
 */
export function topCardAt(point: CanvasPoint, notes: Note[], positions: Positions, accept: (note: Note) => boolean = () => true): Note | undefined {
  for (let index = notes.length - 1; index >= 0; index--) {
    const note = notes[index]; const card = positions[note.id];
    if (card && accept(note) && cardContains(card, point)) return note;
  }
  return undefined;
}

/** 두 모서리로 정한 범위와 겹치는 카드 id. */
export function cardsInRange(a: CanvasPoint, b: CanvasPoint, notes: Note[], positions: Positions): string[] {
  const left = Math.min(a.x, b.x); const top = Math.min(a.y, b.y); const right = Math.max(a.x, b.x); const bottom = Math.max(a.y, b.y);
  return notes.filter((note) => { const card = positions[note.id]; return card && card.x < right && card.x + CARD_WIDTH > left && card.y < bottom && card.y + CARD_HEIGHT > top; }).map((note) => note.id);
}
