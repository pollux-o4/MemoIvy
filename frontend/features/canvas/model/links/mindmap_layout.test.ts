import { describe, expect, it } from 'vitest';
import { CARD_HEIGHT, CARD_WIDTH } from '../align';
import { mindmapLayout } from './mindmap_layout';

const treeOf = (adj: Record<string, string[]>) => (id: string) => adj[id] ?? [];

describe('mindmapLayout', () => {
  it('terminates on circular links and places each card once', () => {
    const children = treeOf({ a: ['b'], b: ['c'], c: ['a'] });
    const pos = mindmapLayout('a', children, {}, { x: 0, y: 0 });
    expect(Object.keys(pos).sort()).toEqual(['a', 'b', 'c']);
    expect(pos.b.x).toBeGreaterThan(pos.a.x);
    expect(pos.c.x).toBeGreaterThan(pos.b.x);
  });

  it('루트는 origin에 놓인다', () => {
    const pos = mindmapLayout('root', treeOf({}), {}, { x: 120, y: 240 });
    expect(pos.root).toEqual({ x: 120, y: 240 });
  });

  it('자식 x는 루트보다 오른쪽 열에 놓인다', () => {
    const pos = mindmapLayout(
      'root',
      treeOf({ root: ['a'], a: ['b'] }),
      { root: { w: 300, h: 156 }, a: { w: 200, h: 156 } },
      { x: 100, y: 50 },
      { columnGap: 80 },
    );
    expect(pos.root.x).toBe(100);
    expect(pos.a.x).toBe(100 + 300 + 80);
    expect(pos.b.x).toBe(100 + 300 + 80 + 200 + 80);
  });

  it('형제가 세로로 겹치지 않고 rowGap 이상 떨어진다', () => {
    const pos = mindmapLayout(
      'root',
      treeOf({ root: ['c1', 'c2', 'c3'] }),
      {},
      { x: 0, y: 0 },
      { rowGap: 24 },
    );
    expect(pos.c2.y - (pos.c1.y + CARD_HEIGHT)).toBeGreaterThanOrEqual(24);
    expect(pos.c3.y - (pos.c2.y + CARD_HEIGHT)).toBeGreaterThanOrEqual(24);
  });

  it('부모는 자식들의 세로 가운데에 놓인다', () => {
    const pos = mindmapLayout(
      'root',
      treeOf({ root: ['a', 'b'] }),
      {},
      { x: 0, y: 100 },
    );
    const rootCenter = pos.root.y + CARD_HEIGHT / 2;
    const firstCenter = pos.a.y + CARD_HEIGHT / 2;
    const lastCenter = pos.b.y + CARD_HEIGHT / 2;
    expect(pos.root.y).toBe(100);
    expect(rootCenter).toBe((firstCenter + lastCenter) / 2);
  });

  it('높이가 다른 카드(펼친 미리보기)와 기본 크기를 반영한다', () => {
    const pos = mindmapLayout(
      'root',
      treeOf({ root: ['tall', 'normal'] }),
      { tall: { w: CARD_WIDTH, h: 320 } },
      { x: 0, y: 0 },
      { rowGap: 30 },
    );
    expect(pos.normal.y - pos.tall.y).toBe(320 + 30);
    const rootCenter = pos.root.y + CARD_HEIGHT / 2;
    const expectedCenter = (pos.tall.y + 320 / 2 + (pos.normal.y + CARD_HEIGHT / 2)) / 2;
    expect(rootCenter).toBe(expectedCenter);
  });

  it('방문한 id는 다시 놓지 않아 순환·중복을 방어한다', () => {
    const pos = mindmapLayout(
      'root',
      treeOf({ root: ['a', 'b'], a: ['root', 'b', 'c'] }),
      {},
      { x: 10, y: 20 },
    );
    expect(Object.keys(pos).sort()).toEqual(['a', 'b', 'c', 'root']);
    expect(pos.root).toEqual({ x: 10, y: 20 });
    expect(pos.b.x).toBe(pos.a.x);
    expect(pos.c.x).toBeGreaterThan(pos.a.x);
  });
});
