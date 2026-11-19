import { describe, expect, it } from 'vitest';
import { previewAnchors } from './preview';

const card = { x: 0, y: 0, w: 200, h: 100 };

describe('previewAnchors', () => {
  it('오른쪽 빈 곳으로 끌면 오른쪽 변에서 포인터로 잇고 끝은 왼쪽에서 들어온다', () => {
    const anchors = previewAnchors(card, { x: 500, y: 50 });
    expect(anchors.from.side).toBe('right');
    expect(anchors.to.point).toEqual({ x: 500, y: 50 });
    expect(anchors.to.side).toBe('left');
  });
  it('아래로 끌면 아래 변에서 나간다', () => {
    expect(previewAnchors(card, { x: 100, y: 400 }).from.side).toBe('bottom');
  });
  it('놓을 카드가 있으면 실제 간선처럼 두 변을 고른다', () => {
    const anchors = previewAnchors(card, { x: 400, y: 0, w: 200, h: 100 });
    expect([anchors.from.side, anchors.to.side]).toEqual(['right', 'left']);
  });
});
