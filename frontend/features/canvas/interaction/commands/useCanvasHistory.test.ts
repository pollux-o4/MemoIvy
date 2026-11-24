import { describe, expect, it } from 'vitest';
import { HISTORY_LIMIT, liveChanges, popLive, pushBounded } from './useCanvasHistory';

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

describe('history after deletion', () => {
  const current = new Set(['a', 'b']);
  it('drops changes and links that point at deleted notes', () => {
    expect(liveChanges([
      { id: 'a', canvas: { next: ['b', 'gone'] } },
      { id: 'gone', canvas: { position: [1, 2] } },
    ], current)).toEqual([{ id: 'a', canvas: { next: ['b'] } }]);
  });
  it('skips commands that only touched deleted notes', () => {
    const live = { before: [{ id: 'a', canvas: { position: [0, 0] as [number, number] } }], after: [] };
    const dead = { before: [{ id: 'gone', canvas: { position: [0, 0] as [number, number] } }], after: [] };
    const { rest, command, changes } = popLive([live, dead, dead], 'before', current);
    expect(command).toBe(live);
    expect(rest).toEqual([]);
    expect(changes).toEqual(live.before);
    expect(popLive([dead], 'before', current).command).toBeUndefined();
  });
});
