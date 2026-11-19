import { describe, expect, it } from 'vitest';
import { anchorOnSide, chooseSides, layoutAnchors } from './anchors';
import { ANCHOR_INSET, type Edge, type Rect, type SideMemory } from './types';

const rect = (x: number, y: number, w = 200, h = 100): Rect => ({ x, y, w, h });

describe('chooseSides', () => {
  it('오른쪽에 있는 카드는 right/left를 고른다', () => {
    const a = rect(0, 0);
    const b = rect(300, 0);
    expect(chooseSides(a, b)).toEqual({ from: 'right', to: 'left' });
  });

  it('아래에 있는 카드는 bottom/top을 고른다', () => {
    const a = rect(0, 0);
    const b = rect(0, 200);
    expect(chooseSides(a, b)).toEqual({ from: 'bottom', to: 'top' });
  });

  it('대각 경계 근처에서는 previous를 유지한다', () => {
    // a의 대각선 기울기 h/w = 100/200 = 0.5. dx=200, dy=110이면 기울기 0.55 (경계 +10% < 15%)
    const a = rect(0, 0, 200, 100);
    const b = rect(200, 110, 200, 100);
    expect(chooseSides(a, b, { from: 'right', to: 'left' })).toEqual({ from: 'right', to: 'left' });
  });

  it('대각 경계를 오가며 끌어도 변은 한 번만 바뀐다', () => {
    // 2차 완료 기준: 드래그 중 경계 근처에서 연결점이 깜빡이지 않는다.
    const a = rect(0, 0, 200, 100);
    const path = [...Array.from({ length: 40 }, (_, i) => 80 + i * 2), ...Array.from({ length: 40 }, (_, i) => 160 - i), ...Array.from({ length: 40 }, (_, i) => 120 + i * 2)];
    let previous: ReturnType<typeof chooseSides> | undefined;
    let switches = 0;
    for (const dy of path) {
      const sides = chooseSides(a, rect(200, dy, 200, 100), previous);
      if (previous && sides.from !== previous.from) switches++;
      previous = sides;
    }
    expect(switches).toBe(1);
  });

  it('previous가 없으면 경계 근처에서도 기울기로 정상 선택한다', () => {
    const a = rect(0, 0, 200, 100);
    const b = rect(200, 110, 200, 100);
    // |dy|/|dx| = 0.55 > 0.5 이므로 세로(bottom/top)
    expect(chooseSides(a, b)).toEqual({ from: 'bottom', to: 'top' });
  });
});

describe('anchorOnSide', () => {
  it('변이 2*ANCHOR_INSET보다 짧으면 가운데를 반환한다', () => {
    const small = rect(10, 20, 20, 20);
    const anchor = anchorOnSide(small, 'left', 0);
    expect(anchor.point).toEqual({ x: 10, y: 30 });
    expect(anchor.normal).toEqual({ x: -1, y: 0 });
  });
});

describe('layoutAnchors', () => {
  it('한 변의 3개 간선은 서로 다른 점에 상대 y 순서로 배치된다', () => {
    const rects: Record<string, Rect> = {
      a: rect(0, 100, 200, 160),
      b1: rect(400, 20, 200, 100),
      b2: rect(400, 130, 200, 100),
      b3: rect(400, 240, 200, 100),
    };
    // 입력 순서를 섞어 정렬이 동작하는지 확인한다.
    const edges: Edge[] = [
      { from: 'a', to: 'b3' },
      { from: 'a', to: 'b1' },
      { from: 'a', to: 'b2' },
    ];
    const result = layoutAnchors(rects, edges);
    const byKey = new Map(result.map((item) => [item.key, item]));

    const y1 = byKey.get('a:b1')?.from.point.y ?? Number.NaN;
    const y2 = byKey.get('a:b2')?.from.point.y ?? Number.NaN;
    const y3 = byKey.get('a:b3')?.from.point.y ?? Number.NaN;

    expect(y1).toBeLessThan(y2);
    expect(y2).toBeLessThan(y3);
  });

  it('단일 간선의 투영 위치는 모서리 ANCHOR_INSET 안쪽에 놓인다', () => {
    const rects: Record<string, Rect> = {
      a: rect(0, 0, 200, 120),
      b: rect(400, 50, 200, 120),
    };
    const [item] = layoutAnchors(rects, [{ from: 'a', to: 'b' }]);
    // b 중심 y=110은 a의 [0, 120] 범위 끝쪽이라 ANCHOR_INSET 안쪽에 찍힌다.
    expect(item.from.side).toBe('right');
    expect(item.from.point.y).toBeGreaterThanOrEqual(ANCHOR_INSET);
    expect(item.from.point.y).toBeLessThanOrEqual(120 - ANCHOR_INSET);
  });

  it('양방향 간선은 같은 bend를 받아 진행 방향 왼쪽으로 각자 벌어진다', () => {
    const rects: Record<string, Rect> = {
      a: rect(0, 0),
      b: rect(300, 0),
    };
    const result = layoutAnchors(rects, [
      { from: 'a', to: 'b' },
      { from: 'b', to: 'a' },
    ]);
    expect(result).toHaveLength(2);
    expect(result[0].bend).not.toBe(0);
    expect(result[0].bend).toBe(result[1].bend);
  });

  it('rects에 없는 카드가 낀 간선은 건너뛴다', () => {
    const rects: Record<string, Rect> = {
      a: rect(0, 0),
      b: rect(300, 0),
    };
    const memory: SideMemory = new Map([['a:b', { from: 'right', to: 'left' }]]);
    const result = layoutAnchors(
      rects,
      [
        { from: 'a', to: 'missing' },
        { from: 'missing', to: 'b' },
        { from: 'a', to: 'b' },
      ],
      memory,
    );
    expect(result).toHaveLength(1);
    expect(result[0].key).toBe('a:b');
    expect(result[0].bend).toBe(0);
  });
});
