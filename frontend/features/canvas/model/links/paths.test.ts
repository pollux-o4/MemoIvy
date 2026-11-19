import { describe, expect, it } from 'vitest';
import { curvePath, orthogonalPath, PATH_STRATEGIES, straightPath, trimEnd } from './paths';
import { ARROW_LENGTH, type Anchor, type EdgeAnchors, type Side } from './types';

const NORMALS: Record<Side, { x: number; y: number }> = {
  left: { x: -1, y: 0 },
  right: { x: 1, y: 0 },
  top: { x: 0, y: -1 },
  bottom: { x: 0, y: 1 },
};

function makeAnchor(side: Side, x: number, y: number): Anchor {
  return { side, point: { x, y }, normal: NORMALS[side] };
}

function makeAnchors(from: Anchor, to: Anchor, bend = 0): EdgeAnchors {
  return { key: 'a:b', from, to, bend };
}

/** SVG path 끝 좌표를 [x, y] 숫자로 뽑는다. */
function lastCoords(d: string): [number, number] {
  const nums = d.match(/-?\d+(?:\.\d+)?/g)?.map(Number) ?? [];
  return [nums[nums.length - 2], nums[nums.length - 1]];
}

describe('paths', () => {
  it('trimEnd는 도착점을 normal 방향으로 length만큼 옮긴다', () => {
    const to = makeAnchor('left', 300, 120);
    expect(trimEnd(to)).toEqual({ x: 300 - ARROW_LENGTH, y: 120 });
    expect(trimEnd(to, 12)).toEqual({ x: 288, y: 120 });
  });

  it('세 모양 모두 M으로 시작하고 끝 좌표가 trimEnd와 같다', () => {
    const anchors = makeAnchors(makeAnchor('right', 100, 80), makeAnchor('left', 320, 180));
    const expectedEnd = trimEnd(anchors.to);

    for (const strategy of [curvePath, straightPath, orthogonalPath]) {
      const d = strategy(anchors);
      expect(d.startsWith('M ')).toBe(true);
      const [ex, ey] = lastCoords(d);
      expect(ex).toBe(expectedEnd.x);
      expect(ey).toBe(expectedEnd.y);
    }
  });

  it('curvePath의 첫 제어점은 from.normal 방향 쪽에 있다', () => {
    const anchors = makeAnchors(makeAnchor('right', 100, 100), makeAnchor('left', 400, 100));
    const d = curvePath(anchors);
    const match = d.match(/^M [^ ]+ [^ ]+ C (-?\d+(?:\.\d+)?) (-?\d+(?:\.\d+)?),/);
    expect(match).not.toBeNull();
    const x1 = Number(match?.[1]);
    const y1 = Number(match?.[2]);
    expect(x1).toBeGreaterThan(100);
    expect(y1).toBe(100);
  });

  it('curvePath 손잡이는 가로 간격에 맞춰 좁은 간격·큰 세로 차에서도 서로를 지나치지 않는다', () => {
    // 가로 60, 세로 200: 대각선 거리로 손잡이를 잡으면 c1.x > c2.x가 되어 꼬부랑 모양이 된다.
    const anchors = makeAnchors(makeAnchor('right', 100, 100), makeAnchor('left', 168, 300));
    const nums = curvePath(anchors).match(/-?\d+(?:\.\d+)?/g)?.map(Number) ?? [];
    const [c1x, , c2x] = nums.slice(2, 6);
    expect(c1x).toBeLessThanOrEqual(c2x);
  });

  it('curvePath는 상대가 뒤쪽이어도 바깥으로 돌아 나온다', () => {
    const anchors = makeAnchors(makeAnchor('right', 300, 100), makeAnchor('left', 108, 300));
    const nums = curvePath(anchors).match(/-?\d+(?:\.\d+)?/g)?.map(Number) ?? [];
    expect(nums[2]).toBeGreaterThan(300);
    expect(nums[4]).toBeLessThan(100);
  });

  it('straightPath는 bend가 0이면 직선, 0이 아니면 2차 곡선(Q)을 만든다', () => {
    const base = makeAnchors(makeAnchor('right', 0, 0), makeAnchor('left', 208, 0), 0);
    expect(straightPath(base)).toBe('M 0 0 L 200 0');

    const bent = makeAnchors(makeAnchor('right', 0, 0), makeAnchor('left', 208, 0), 25);
    const d = straightPath(bent);
    expect(d).toContain(' Q ');
    expect(d).not.toBe(straightPath(base));
  });

  it('bend가 세 가지 경로 모두를 바꾼다', () => {
    const from = makeAnchor('right', 100, 100);
    const to = makeAnchor('left', 320, 220);
    for (const strategy of [curvePath, straightPath, orthogonalPath]) {
      expect(strategy(makeAnchors(from, to, 20))).not.toBe(strategy(makeAnchors(from, to, 0)));
    }
  });

  it('orthogonalPath는 M/L/Q 명령만 쓰고 직선 구간은 수평·수직만 가진다', () => {
    const anchors = makeAnchors(makeAnchor('right', 100, 100), makeAnchor('left', 320, 200));
    const d = orthogonalPath(anchors);
    const commands = d.match(/[A-Za-z]/g) ?? [];
    expect(commands.every((cmd) => cmd === 'M' || cmd === 'L' || cmd === 'Q')).toBe(true);
    expect(commands).toContain('Q');

    // M/L 좌표쌍과 Q 끝점을 순서대로 따라가며 L 직선 구간이 수평 또는 수직인지 확인한다.
    const tokens = d.split(/\s+/);
    let cx = 0;
    let cy = 0;
    for (let i = 0; i < tokens.length; ) {
      const cmd = tokens[i++];
      if (cmd === 'M') {
        cx = Number(tokens[i++]);
        cy = Number(tokens[i++]);
      } else if (cmd === 'L') {
        const nx = Number(tokens[i++]);
        const ny = Number(tokens[i++]);
        expect(cx === nx || cy === ny).toBe(true);
        cx = nx;
        cy = ny;
      } else if (cmd === 'Q') {
        i += 2; // 제어점 건너뛰기
        cx = Number(tokens[i++]);
        cy = Number(tokens[i++]);
      }
    }
  });

  it('orthogonalPath는 짧은 꺾임에서 모서리 반경을 8px보다 작게 줄인다', () => {
    // 수직 거리가 6px뿐인 경우 반경은 3px 이하로 줄어들어야 한다.
    const anchors = makeAnchors(makeAnchor('right', 100, 100), makeAnchor('left', 300, 106));
    const d = orthogonalPath(anchors);
    // 첫 Q 제어점(y=100)에서 끝점(y=103)까지의 y 차이가 3 이하인지 확인한다.
    const qMatch = d.match(/Q (-?\d+(?:\.\d+)?) (-?\d+(?:\.\d+)?) (-?\d+(?:\.\d+)?) (-?\d+(?:\.\d+)?)/);
    expect(qMatch).not.toBeNull();
    const qy = Number(qMatch?.[2]);
    const endY = Number(qMatch?.[4]);
    expect(Math.abs(endY - qy)).toBeLessThan(8);
    expect(Math.abs(endY - qy)).toBeCloseTo(3, 1);
  });

  it('PATH_STRATEGIES는 curve, straight, orthogonal 세 키를 가진다', () => {
    expect(Object.keys(PATH_STRATEGIES).sort()).toEqual(['curve', 'orthogonal', 'straight']);
    expect(PATH_STRATEGIES.curve).toBe(curvePath);
    expect(PATH_STRATEGIES.straight).toBe(straightPath);
    expect(PATH_STRATEGIES.orthogonal).toBe(orthogonalPath);
  });
});

describe('양방향 간선 벌리기', () => {
  const left: Anchor = { side: 'right', point: { x: 100, y: 50 }, normal: { x: 1, y: 0 } };
  const right: Anchor = { side: 'left', point: { x: 400, y: 50 }, normal: { x: -1, y: 0 } };
  it.each(['curve', 'straight', 'orthogonal'] as const)('%s: A에서 B와 B에서 A가 같은 bend로 서로 다른 경로가 된다', (style) => {
    const ab = PATH_STRATEGIES[style]({ key: 'a:b', from: left, to: right, bend: 12 });
    const ba = PATH_STRATEGIES[style]({ key: 'b:a', from: { ...right, side: 'left' }, to: { ...left, side: 'right' }, bend: 12 });
    // 방향과 무관하게 비교하려고 좌표를 정렬한 묶음으로 본다. 같으면 두 선이 겹친다.
    const shape = (d: string) => (d.match(/-?\d+(\.\d+)?/g) ?? []).map(Number).sort((a, b) => a - b).join(',');
    expect(shape(ab)).not.toBe(shape(ba));
  });
});
