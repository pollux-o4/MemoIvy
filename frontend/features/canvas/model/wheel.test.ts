import { expect, test } from 'vitest';
import { classifyWheel, TOUCHPAD_MAX_STEP, WHEEL_PAN, wheelIntent } from './wheel';
import { limitLead } from './motion';

const base = { deltaX: 0, deltaY: 0, deltaMode: 0, ctrlKey: false, metaKey: false, shiftKey: false };
const notch = { kind: 'pan', delta: [0, WHEEL_PAN], smooth: true };

test('one wheel notch pans a fixed distance smoothly on every platform unit', () => {
  expect(wheelIntent({ ...base, deltaY: 100 })).toEqual(notch); // Windows·macOS Chromium
  expect(wheelIntent({ ...base, deltaY: 99.99999628067008 })).toEqual(notch); // Chromium 배율 오차
  expect(wheelIntent({ ...base, deltaY: 160 / 3 })).toEqual(notch); // Linux Chromium
  expect(wheelIntent({ ...base, deltaY: 3, deltaMode: 1 })).toEqual(notch); // 줄 모드
});

test('Linux notch scaled by display factor is still one notch', () => {
  expect(classifyWheel({ ...base, deltaY: 160 / 3 * 1.25 }, 1.25)).toEqual({ device: 'wheel', notches: [0, 1] });
});

test('shift + wheel pans horizontally even when the browser already moved it to deltaX', () => {
  expect(wheelIntent({ ...base, deltaY: -100, shiftKey: true })).toEqual({ kind: 'pan', delta: [-WHEEL_PAN, 0], smooth: true });
  expect(wheelIntent({ ...base, deltaX: 100, shiftKey: true })).toEqual({ kind: 'pan', delta: [WHEEL_PAN, 0], smooth: true });
});

test('touchpad pixels pan 1:1 but a fling spike is capped per event', () => {
  expect(wheelIntent({ ...base, deltaX: 3.5, deltaY: -12.25 })).toEqual({ kind: 'pan', delta: [3.5, -12.25], smooth: false });
  expect(wheelIntent({ ...base, deltaY: 187.3 })).toEqual({ kind: 'pan', delta: [0, TOUCHPAD_MAX_STEP], smooth: false });
});

test('ctrl + wheel zooms in on scroll up and out on scroll down', () => {
  const zoomIn = wheelIntent({ ...base, deltaY: -100, ctrlKey: true });
  const zoomOut = wheelIntent({ ...base, deltaY: 100, ctrlKey: true });
  expect(zoomIn.kind === 'zoom' && zoomIn.factor).toBeCloseTo(Math.exp(0.18));
  expect(zoomOut.kind === 'zoom' && zoomOut.factor).toBeCloseTo(Math.exp(-0.18));
});

test('rapid notches cannot push the camera target far beyond the screen', () => {
  const current = { x: 0, y: 0, zoom: 2 };
  expect(limitLead(current, { x: 0, y: 500, zoom: 2 }, 200)).toEqual({ x: 0, y: 100, zoom: 2 });
  expect(limitLead(current, { x: 0, y: 50, zoom: 2 }, 200)).toEqual({ x: 0, y: 50, zoom: 2 });
});
