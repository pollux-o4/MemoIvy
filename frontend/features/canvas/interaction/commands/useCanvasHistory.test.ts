import { describe, expect, it } from 'vitest';
import type { Note } from '../../../../shared/engine_contract';
import { deletionCommand, HISTORY_LIMIT, liveChanges, liveStep, popLive, pushBounded } from './useCanvasHistory';

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
    const live = { before: { changes: [{ id: 'a', canvas: { position: [0, 0] as [number, number] } }] }, after: { changes: [] } };
    const dead = { before: { changes: [{ id: 'gone', canvas: { position: [0, 0] as [number, number] } }] }, after: { changes: [] } };
    const { rest, command, step } = popLive([live, dead, dead], 'before', current);
    expect(command).toBe(live);
    expect(rest).toEqual([]);
    expect(step).toEqual(live.before);
    expect(popLive([dead], 'before', current).command).toBeUndefined();
  });
});

describe('deletion command', () => {
  const note = (id: string, canvas: Partial<Note['canvas']> = {}): Note => ({ id, text: id, color: 'yellow', modified: 0, canvas: { position: null, next: [], done: false, ...canvas } });
  const notes = [note('p', { project: true }), note('c', { parent: 'p' }), note('s', { next: ['p', 'c'] }), note('x')];
  const command = deletionCommand(notes, ['p']);
  it('captures deleted notes and the links and parents deletion rewrites', () => {
    expect(command.before.restore?.map((n) => n.id)).toEqual(['p']);
    expect(command.before.changes).toEqual([
      { id: 'c', canvas: { parent: 'p', next: [] } },
      { id: 's', canvas: { parent: null, next: ['p', 'c'] } },
    ]);
    expect(command.after).toEqual({ changes: [], remove: ['p'] });
  });
  it('restores only while the notes are gone and redoes only while they exist', () => {
    expect(liveStep(command.before, new Set(['c', 's', 'x']))?.restore).toHaveLength(1);
    expect(liveStep(command.before, new Set(['p', 'c']))).toBeUndefined();
    expect(liveStep(command.after, new Set(['c']))).toBeUndefined();
    expect(liveStep(command.after, new Set(['p']))).toEqual({ changes: [], remove: ['p'] });
  });
});
