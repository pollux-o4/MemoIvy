import { describe, expect, test } from 'vitest';
import type { Note } from './engine_contract';
import { childCountLabel, createChildIndex } from './hierarchy';

const note = (id: string, parent: string | null = null): Note => ({ id, text: '', color: 'yellow', modified: 0, canvas: { position: null, next: [], done: false, parent, project: true } });

describe('child index', () => {
  test('lists direct children only', () => {
    const children = createChildIndex([note('p'), note('a', 'p'), note('sub', 'p'), note('b', 'sub')]);
    expect(children('p').map((item) => item.id)).toEqual(['a', 'sub']);
    expect(children('b')).toEqual([]);
    expect(childCountLabel(children('p').length)).toBe('메모 2개');
  });
});
