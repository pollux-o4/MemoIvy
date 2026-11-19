import { describe, expect, test } from 'vitest';
import type { Note } from '../../../shared/engine_contract';
import { ancestry, canMoveInto, scopeDepth, scopeShortcuts } from './scope_model';

const note = (id: string, parent: string | null = null, project = true, pinned = false): Note => ({
  id, text: id, color: 'yellow', modified: 0,
  canvas: { position: null, next: [], done: false, project, parent, pinned },
});

describe('canvas scope model', () => {
  test('returns root to self and stops at a cyclic parent', () => {
    const notes = [note('a', 'b'), note('b', 'a')];
    expect(ancestry('a', notes).map(({ id }) => id)).toEqual(['b', 'a']);
    expect(scopeDepth('a', notes)).toBe(2);
  });

  test('blocks moving into scopes at depth three or deeper', () => {
    const notes = [note('root'), note('child', 'root'), note('leaf', 'child')];
    expect(canMoveInto('child', [], notes)).toBe(true);
    expect(canMoveInto('leaf', [], notes)).toBe(false);
  });

  test('blocks moving a scope into itself or its descendant', () => {
    const notes = [note('root'), note('child', 'root'), note('leaf', 'child')];
    expect(canMoveInto('leaf', ['child'], notes)).toBe(false);
    expect(canMoveInto('root', ['root'], notes)).toBe(false);
  });

  test('lists pinned projects before recent scopes, excluding path and duplicates', () => {
    const notes = [note('pinned-a', null, true, true), note('path', null, true, true), note('pinned-b', null, true, true), note('plain', null, false, true)];
    expect(scopeShortcuts(new Set(['path']), ['recent', 'pinned-a', 'recent-2'], notes, 5))
      .toEqual(['pinned-a', 'pinned-b', 'recent', 'recent-2']);
  });
});
