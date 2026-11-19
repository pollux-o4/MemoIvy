import { describe, expect, it } from 'vitest';
import { directChildren, outlineParent } from './outline_tree';

const row = (id: string, depth: number) => ({ id, depth, hasChildren: false, project: false });

describe('directChildren', () => {
  it('returns only the rows one level below the current row', () => {
    const rows = [row('root', 0), row('p', 1), row('me', 1), row('a', 2), row('a1', 3), row('b', 2), row('next', 1)];
    expect([...directChildren(rows, 'me')]).toEqual(['a', 'b']);
    expect(directChildren(rows, 'missing').size).toBe(0);
  });
});

describe('outlineParent', () => {
  it('finds the tree parent, not just any earlier row', () => {
    const rows = [row('root', 0), row('a', 1), row('a1', 2), row('b', 1), row('b1', 2)];
    expect(outlineParent(rows, 'b1')).toBe('b');
    expect(outlineParent(rows, 'root')).toBeUndefined();
  });
});
