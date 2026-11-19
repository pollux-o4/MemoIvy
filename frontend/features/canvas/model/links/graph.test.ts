import { describe, expect, test } from 'vitest';
import type { Note } from '../../../../shared/engine_contract';
import { branchRoot, branchSet, buildGraph, outline, scopeEdges, treeParent } from './graph';

const note = (
  id: string,
  next: string[] = [],
  parent: string | null = null,
  project = false,
): Note => ({
  id,
  text: '',
  color: 'yellow',
  modified: 0,
  canvas: { position: null, next, done: false, parent, project },
});

describe('links graph', () => {
  test('buildGraph가 out과 in 인접 목록을 만든다', () => {
    const graph = buildGraph([
      { from: 'a', to: 'b' },
      { from: 'a', to: 'c' },
      { from: 'b', to: 'c' },
    ]);
    expect(graph.out.get('a')).toEqual(['b', 'c']);
    expect(graph.out.get('b')).toEqual(['c']);
    expect(graph.out.get('c')).toBeUndefined();
    expect(graph.in.get('c')).toEqual(['a', 'b']);
    expect(graph.in.get('a')).toBeUndefined();
  });

  test('scopeEdges가 다른 단계의 간선을 제외한다', () => {
    const notes = [
      note('a', ['b', 'sub1'], null),
      note('b', ['c'], null),
      note('sub1', ['sub2'], 'p1'),
      note('sub2', ['a'], 'p1'),
    ];
    expect(scopeEdges(notes, null)).toEqual([{ from: 'a', to: 'b' }]);
    expect(scopeEdges(notes, 'p1')).toEqual([{ from: 'sub1', to: 'sub2' }]);
  });

  test('branchSet이 조상과 자손만 포함하고 무관한 노드는 제외한다', () => {
    const graph = buildGraph([
      { from: 'root', to: 'mid' },
      { from: 'root', to: 'sibling' },
      { from: 'mid', to: 'leaf' },
      { from: 'otherParent', to: 'leaf' },
      { from: 'isolated1', to: 'isolated2' },
    ]);
    expect(branchSet(graph, 'mid')).toEqual(new Set(['root', 'mid', 'leaf']));
  });

  test('branchSet이 순환 A→B→A에서 멈춘다', () => {
    const graph = buildGraph([
      { from: 'A', to: 'B' },
      { from: 'B', to: 'A' },
    ]);
    expect(branchSet(graph, 'A')).toEqual(new Set(['A', 'B']));
  });

  test('treeParent가 여러 부모 중 in 목록의 첫 번째를 반환한다', () => {
    const graph = buildGraph([
      { from: 'p1', to: 'child' },
      { from: 'p2', to: 'child' },
    ]);
    expect(treeParent(graph, 'child')).toBe('p1');
    expect(treeParent(graph, 'p1')).toBeUndefined();
  });

  test('branchRoot가 순환에서 다시 만나기 직전에 멈춘다', () => {
    const graph = buildGraph([
      { from: 'A', to: 'B' },
      { from: 'B', to: 'C' },
      { from: 'C', to: 'A' },
    ]);
    expect(branchRoot(graph, 'B')).toBe('C');

    const tree = buildGraph([
      { from: 'root', to: 'child' },
      { from: 'child', to: 'grand' },
    ]);
    expect(branchRoot(tree, 'grand')).toBe('root');
  });

  test('outline이 깊이 우선 순서와 깊이, hasChildren, project를 계산한다', () => {
    const graph = buildGraph([
      { from: 'root', to: 'a' },
      { from: 'root', to: 'b' },
      { from: 'a', to: 'a1' },
    ]);
    expect(outline(graph, 'root', (id) => id === 'b')).toEqual([
      { id: 'root', depth: 0, hasChildren: true, project: false },
      { id: 'a', depth: 1, hasChildren: true, project: false },
      { id: 'a1', depth: 2, hasChildren: false, project: false },
      { id: 'b', depth: 1, hasChildren: false, project: true },
    ]);
  });

  test('outline에서 두 부모를 가진 노드가 한 번만 나온다', () => {
    const graph = buildGraph([
      { from: 'root', to: 'a' },
      { from: 'root', to: 'b' },
      { from: 'a', to: 'shared' },
      { from: 'b', to: 'shared' },
    ]);
    const rows = outline(graph, 'root', () => false);
    expect(rows.map((row) => [row.id, row.depth, row.hasChildren])).toEqual([
      ['root', 0, true],
      ['a', 1, true],
      ['shared', 2, false],
      ['b', 1, false],
    ]);
  });
});

describe('가지 밖 부모', () => {
  test('첫 부모가 가지 밖이어도 가지 안 부모 아래에 나온다', () => {
    const g = buildGraph([{ from: 'x', to: 'c' }, { from: 'r', to: 'a' }, { from: 'a', to: 'c' }]);
    expect(outline(g, 'r', () => false).map((row) => `${row.id}${row.depth}`)).toEqual(['r0', 'a1', 'c2']);
  });
});
