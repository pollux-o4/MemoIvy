import { describe, expect, it } from 'vitest';
import type { Note } from '../../../shared/engine_contract';
import { CARD_WIDTH } from './align';
import { cardsInRange, topCardAt } from './hit';

const note = (id: string, project = false) => ({ id, text: '', color: 'yellow', modified: 0, canvas: { position: null, next: [], done: false, parent: null, project } }) as Note;
const notes = [note('a'), note('b', true)];
const positions = { a: { x: 0, y: 0 }, b: { x: 100, y: 0 } };

describe('hit', () => {
  it('겹친 자리에서는 나중에 그려진 카드를 고른다', () => {
    expect(topCardAt({ x: 120, y: 10 }, notes, positions)?.id).toBe('b');
  });
  it('accept로 거른 카드는 건너뛴다', () => {
    expect(topCardAt({ x: 120, y: 10 }, notes, positions, (n) => !n.canvas.project)?.id).toBe('a');
    expect(topCardAt({ x: CARD_WIDTH + 200, y: 10 }, notes, positions)).toBeUndefined();
  });
  it('범위와 겹치는 카드만 고른다(방향 무관)', () => {
    expect(cardsInRange({ x: 50, y: 50 }, { x: -10, y: -10 }, notes, positions)).toEqual(['a']);
    expect(cardsInRange({ x: 150, y: 50 }, { x: -10, y: -10 }, notes, positions)).toEqual(['a', 'b']);
    expect(cardsInRange({ x: -50, y: -50 }, { x: -10, y: -10 }, notes, positions)).toEqual([]);
  });
});
