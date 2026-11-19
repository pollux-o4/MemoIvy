import { describe, expect, it } from 'vitest';
import { revealCamera, visibleArea } from '.';

const board = { x: 0, y: 0, w: 800, h: 600 };
const camera = { x: 0, y: 0, zoom: 1 };

describe('reveal', () => {
  it('오른쪽 패널을 뺀 영역이 보이는 영역이다', () => {
    expect(visibleArea(board, [{ x: 450, y: 80, w: 350, h: 500 }])).toEqual({ x: 0, y: 0, w: 450, h: 600 });
  });

  it('보드와 겹치지 않는 패널은 무시한다', () => {
    expect(visibleArea(board, [{ x: 900, y: 0, w: 100, h: 100 }])).toEqual({ x: 0, y: 0, w: 800, h: 600 });
  });

  it('패널에 가린 카드는 가려진 만큼만 옮긴다', () => {
    const area = visibleArea(board, [{ x: 450, y: 80, w: 350, h: 500 }]);
    const next = revealCamera({ x: 500, y: 200, w: 160, h: 120 }, camera, area);
    expect(next).toEqual({ x: 500 + 160 - 450 + 16, y: 0, zoom: 1 });
  });

  it('이미 보이는 카드는 카메라를 움직이지 않는다', () => {
    expect(revealCamera({ x: 100, y: 100, w: 160, h: 120 }, camera, board)).toBeNull();
  });

  it('확대 배율을 반영해 화면 px을 세계 좌표로 바꾼다', () => {
    const next = revealCamera({ x: -100, y: 50, w: 100, h: 100 }, { x: 0, y: 0, zoom: 2 }, board);
    expect(next?.x).toBe(-100 - 16 / 2);
  });
});
