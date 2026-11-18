import { describe, expect, it } from 'vitest';
import { snapRect } from './snap';

const card = (x: number, y: number) => ({ x, y, w: 240, h: 156 });

describe('snapRect', () => {
  it('snaps top edges within threshold and draws one horizontal guide spanning both cards', () => {
    const result = snapRect(card(400, 104), [card(0, 100)], 6);
    expect(result).toMatchObject({ dx: 0, dy: -4 });
    expect(result.guides).toEqual([{ axis: 'y', at: 100, from: 0, to: 640 }]);
  });

  it('prefers the closest line and snaps centers', () => {
    const result = snapRect(card(2, 400), [card(0, 0)], 6);
    expect(result.dx).toBe(-2);
    expect(result.guides).toEqual([{ axis: 'x', at: 0, from: 0, to: 556 }]);
  });

  it('leaves the rect alone and draws nothing outside threshold', () => {
    expect(snapRect(card(300, 300), [card(0, 0)], 6)).toEqual({ dx: 0, dy: 0, guides: [] });
  });
});
