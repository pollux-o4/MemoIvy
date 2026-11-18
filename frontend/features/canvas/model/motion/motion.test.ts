import assert from 'node:assert/strict';
import { test } from 'vitest';
import { Coast, Spring, centeredOn, fitting, firstLayout, pulseSample, zoomAt } from './index';

test('critical spring advances monotonically and settles at its target', () => {
  const spring = new Spring(0, 1, 0.3, 1, 0.0005);
  let last = 0;
  for (let i = 0; i < 120; i += 1) {
    spring.advance(1 / 60);
    assert.ok(spring.value >= last && spring.value <= 1);
    last = spring.value;
  }
  assert.equal(spring.settled(), true);
  assert.equal(spring.value, 1);
});

test('fitting uses 72px padding, centers bounds and limits zoom', () => {
  const camera = fitting({ x: 5000, y: 5000, w: 240, h: 156 }, [1000, 700]);
  assert.equal(camera.zoom, 1);
  assert.deepEqual(camera, centeredOn({ x: 5000, y: 5000, w: 240, h: 156 }, [1000, 700], 1));
  const huge = fitting({ x: 0, y: 0, w: 20_000, h: 20_000 }, [1000, 700]);
  assert.equal(huge.zoom, 0.05);
});

test('first layout fits all cards and recenters an offscreen selection at 100%', () => {
  const cards = [{ x: 0, y: 0, w: 200, h: 100 }, { x: 300, y: 0, w: 100, h: 100 }];
  assert.equal(firstLayout([], [], [1000, 700]), null);
  assert.deepEqual(firstLayout(cards, [], [1000, 700]), fitting({ x: 0, y: 0, w: 400, h: 100 }, [1000, 700]));
  const outside = { x: 2000, y: 1000, w: 100, h: 100 };
  assert.deepEqual(firstLayout(cards, [outside], [1000, 700]), centeredOn(outside, [1000, 700], 1));
});

test('coast slows exponentially, stops below threshold, and caps starting speed', () => {
  assert.equal(Coast.start([119, 0]), null);
  let coast = Coast.start([2400, 0]);
  assert.ok(coast);
  let first = coast.advance(0.05);
  assert.ok(first.delta[0] > 0 && first.delta[0] < 60);
  coast = first.next;
  assert.ok(coast);
  let total = first.delta[0];
  for (let i = 0; i < 100 && coast; i += 1) {
    const step = coast.advance(0.05);
    assert.ok(step.delta[0] >= 0);
    total += step.delta[0];
    coast = step.next;
  }
  assert.equal(coast, null);
  assert.ok(total > first.delta[0]);
});

test('reduced motion pulse is static and zoomAt clamps while preserving anchor', () => {
  assert.deepEqual(pulseSample(0.2, false), { travel: 1, alpha: 1 });
  const camera = { x: 10, y: -5, zoom: 0.5 };
  const anchor: [number, number] = [100, 80];
  const before = [anchor[0] / camera.zoom + camera.x, anchor[1] / camera.zoom + camera.y];
  const zoomed = zoomAt(camera, anchor, 99);
  assert.equal(zoomed.zoom, 2);
  assert.ok(Math.abs((anchor[0] / zoomed.zoom + zoomed.x) - before[0]) < 1e-10);
  assert.ok(Math.abs((anchor[1] / zoomed.zoom + zoomed.y) - before[1]) < 1e-10);
  assert.equal(zoomAt(camera, anchor, 0).zoom, 0.05);
});
