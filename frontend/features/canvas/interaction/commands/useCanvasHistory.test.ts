import { describe, expect, it } from 'vitest';
import { HISTORY_LIMIT, pushBounded } from './useCanvasHistory';

describe('pushBounded', () => {
  it('keeps the newest items up to the limit', () => {
    expect(pushBounded([1, 2, 3], 4, 3)).toEqual([2, 3, 4]);
    expect(pushBounded([1], 2, 3)).toEqual([1, 2]);
  });
  it('caps undo history at HISTORY_LIMIT', () => {
    let stack: number[] = [];
    for (let i = 0; i < HISTORY_LIMIT + 50; i++) stack = pushBounded(stack, i);
    expect(stack).toHaveLength(HISTORY_LIMIT);
    expect(stack[0]).toBe(50);
  });
});
