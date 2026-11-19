import { describe, expect, it } from 'vitest';
import { indentRow, outdentRow } from './outline_edit';
import type { OutlineRow } from './types';

const row = (id: string, depth: number, hasChildren = false, project = false): OutlineRow => ({
  id,
  depth,
  hasChildren,
  project,
});

const nextOf = (adj: Record<string, readonly string[]>) => (id: string) => adj[id] ?? [];

describe('outline_edit', () => {
  it('들여쓰기 정상: 부모 next에서 빼고 바로 위 형제 next 끝에 더한다', () => {
    const rows: OutlineRow[] = [
      row('root', 0, true),
      row('a', 1, true),
      row('a1', 2),
      row('b', 1),
      row('c', 1),
    ];
    const adj = {
      root: ['a', 'b', 'c'],
      a: ['a1'],
      b: [],
    };

    const changes = indentRow(rows, 'b', nextOf(adj));

    expect(changes).toEqual([
      { id: 'root', next: ['a', 'c'] },
      { id: 'a', next: ['a1', 'b'] },
    ]);
    expect(adj.root).toEqual(['a', 'b', 'c']);
    expect(adj.a).toEqual(['a1']);
  });

  it('들여쓰기 시 바로 위 형제가 없으면 빈 배열을 반환한다', () => {
    const rows: OutlineRow[] = [
      row('root', 0, true),
      row('a', 1, true),
      row('a1', 2),
    ];
    const adj = { root: ['a'], a: ['a1'] };

    expect(indentRow(rows, 'a', nextOf(adj))).toEqual([]);
    expect(indentRow(rows, 'a1', nextOf(adj))).toEqual([]);
    expect(indentRow(rows, 'missing', nextOf(adj))).toEqual([]);
  });

  it('뿌리(depth 0)는 들여쓰기할 수 없다', () => {
    const rows: OutlineRow[] = [row('root', 0, true), row('a', 1)];
    expect(indentRow(rows, 'root', nextOf({ root: ['a'] }))).toEqual([]);
  });

  it('내어쓰기 정상: 부모 next에서 빼고 조부모 next의 부모 바로 뒤에 넣는다', () => {
    const rows: OutlineRow[] = [
      row('root', 0, true),
      row('a', 1, true),
      row('a1', 2),
      row('a2', 2),
      row('b', 1),
    ];
    const adj = {
      root: ['a', 'b'],
      a: ['a1', 'a2'],
    };

    const changes = outdentRow(rows, 'a1', nextOf(adj));

    expect(changes).toEqual([
      { id: 'a', next: ['a2'] },
      { id: 'root', next: ['a', 'a1', 'b'] },
    ]);
    expect(adj.root).toEqual(['a', 'b']);
    expect(adj.a).toEqual(['a1', 'a2']);
  });

  it('depth 1(또는 depth 0)은 내어쓰기할 수 없다', () => {
    const rows: OutlineRow[] = [row('root', 0, true), row('a', 1)];
    const adj = { root: ['a'] };

    expect(outdentRow(rows, 'a', nextOf(adj))).toEqual([]);
    expect(outdentRow(rows, 'root', nextOf(adj))).toEqual([]);
    expect(outdentRow(rows, 'missing', nextOf(adj))).toEqual([]);
  });

  it('이미 대상 next에 id가 있으면 중복 추가하지 않는다', () => {
    const rowsIndent: OutlineRow[] = [
      row('root', 0, true),
      row('a', 1),
      row('b', 1),
    ];
    const adjIndent = {
      root: ['a', 'b'],
      a: ['b'],
    };
    expect(indentRow(rowsIndent, 'b', nextOf(adjIndent))).toEqual([
      { id: 'root', next: ['a'] },
      { id: 'a', next: ['b'] },
    ]);

    const rowsOutdent: OutlineRow[] = [
      row('root', 0, true),
      row('a', 1, true),
      row('a1', 2),
      row('b', 1),
    ];
    const adjOutdent = {
      root: ['a', 'b', 'a1'],
      a: ['a1'],
    };
    expect(outdentRow(rowsOutdent, 'a1', nextOf(adjOutdent))).toEqual([
      { id: 'a', next: [] },
      { id: 'root', next: ['a', 'b', 'a1'] },
    ]);
  });
});
