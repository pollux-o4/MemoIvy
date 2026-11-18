import assert from 'node:assert/strict';
import { test } from 'vitest';
import { alignPositions, canvasSlot, changedPositions, clampCanvasDelta, vacantGroup } from './align';

test('aligns selected positions and preserves the other axis', () => {
  const positions = { a: { x: 10, y: 20 }, b: { x: 80, y: 60 }, c: { x: 40, y: 5 } };
  assert.deepEqual(alignPositions(['a', 'b', 'c'], positions, 'left'), {
    a: { x: 10, y: 20 }, b: { x: 10, y: 60 }, c: { x: 10, y: 5 },
  });
});

test('keeps existing row order and GTK grid spacing', () => {
  assert.deepEqual(alignPositions(['a', 'b', 'c'], { a: { x: 10, y: 20 }, b: { x: 80, y: 60 }, c: { x: 40, y: 5 } }, 'grid'), {
    a: { x: 10, y: 5 }, b: { x: 554, y: 5 }, c: { x: 282, y: 5 },
  });
});

test('distributes three cards while keeping the end cards fixed', () => {
  assert.deepEqual(alignPositions(['a', 'b', 'c'], { a: { x: 0, y: 2 }, b: { x: 200, y: 9 }, c: { x: 800, y: 4 } }, 'distribute-x'), {
    a: { x: 0, y: 2 }, b: { x: 400, y: 9 }, c: { x: 800, y: 4 },
  });
});

test('lays a cycle out left to right and drops its backward edge', () => {
  const points = { a: { x: 0, y: 0 }, b: { x: 0, y: 200 }, c: { x: 500, y: 0 } };
  const result = alignPositions(['a', 'b', 'c'], points, 'flow', [['a', 'b'], ['b', 'c'], ['c', 'a']]);
  assert.equal(result.a.x, 0);
  assert.equal(result.b.x, 336);
  assert.equal(result.c.x, 672);
});

test('matches GTK default card slots and places groups without collisions', () => {
  assert.deepEqual(canvasSlot(4), { x: 64, y: 296 });
  const moved = vacantGroup(['a', 'b'], { a: { x: 100, y: 100 }, b: { x: 400, y: 100 } }, [{ x: 64, y: 64 }]);
  assert.equal(moved.b.x - moved.a.x, 300);
  assert.notDeepEqual(moved.a, { x: 64, y: 64 });
});

test('clamps a selected group as one unit to GTK canvas limits', () => {
  const group = [{ x: 10, y: 20 }, { x: 500, y: 600 }];
  assert.deepEqual(clampCanvasDelta(group, -50, -50), { x: -10, y: -20 });
  assert.deepEqual(clampCanvasDelta(group, 20_000, 20_000), { x: 19_500, y: 19_400 });
});

test('changedPositions compares against saved positions, not the drag preview', () => {
  const saved = { a: [10, 10] as [number, number], b: null, c: [0, 0] as [number, number] };
  assert.deepEqual(changedPositions({ a: { x: 40.4, y: 10 }, b: { x: 5, y: 5 }, c: { x: 0.2, y: 0 }, gone: { x: 1, y: 1 } }, saved), [
    { id: 'a', position: [40, 10] },
    { id: 'b', position: [5, 5] },
  ]);
});
