import { describe, expect, it } from 'vitest';
import { dragDelta, keepRecent, releaseVelocity } from './drag';

describe('drag', () => {
  it('붙일 카드가 없으면 범위만 막는다', () => {
    expect(dragDelta([{ x: 10, y: 10 }], [], -50, 5, 1, false)).toEqual({ delta: { x: -10, y: 5 }, guides: [] });
  });
  it('가까운 카드 선에 붙는다(snapOff면 붙지 않는다)', () => {
    const snapped = dragDelta([{ x: 100, y: 100 }], [{ x: 400, y: 103 }], 0, 0, 1, false);
    expect(snapped.delta.y).toBe(3);
    expect(dragDelta([{ x: 100, y: 100 }], [{ x: 400, y: 103 }], 0, 0, 1, true).delta).toEqual({ x: 0, y: 0 });
  });
  it('표본 구간 밖은 버리고 처음·끝으로 속도를 잰다', () => {
    const samples = keepRecent([{ x: 0, y: 0, at: 0 }, { x: 10, y: 0, at: 100 }], { x: 30, y: 0, at: 200 });
    expect(samples).toHaveLength(2);
    expect(releaseVelocity(samples)).toEqual([200, 0]);
    expect(releaseVelocity(samples.slice(0, 1))).toBeNull();
  });
});
