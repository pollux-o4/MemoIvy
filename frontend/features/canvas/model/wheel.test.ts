import { expect, test } from 'vitest';
import { wheelIntent } from './wheel';

const base = { deltaX: 0, deltaY: 0, deltaMode: 0, ctrlKey: false, metaKey: false, shiftKey: false };

test('mouse wheel notch pans vertically by 60px, smoothly', () => {
  expect(wheelIntent({ ...base, deltaY: 100 })).toEqual({ kind: 'pan', delta: [0, 60], smooth: true });
});

test('Chromium float notch (99.99999…) still counts as one wheel notch', () => {
  expect(wheelIntent({ ...base, deltaY: 99.99999628067008 })).toEqual({ kind: 'pan', delta: [0, 60], smooth: true });
});

test('shift + wheel pans horizontally even when the browser already moved it to deltaX', () => {
  expect(wheelIntent({ ...base, deltaY: -100, shiftKey: true })).toEqual({ kind: 'pan', delta: [-60, 0], smooth: true });
  expect(wheelIntent({ ...base, deltaX: 100, shiftKey: true })).toEqual({ kind: 'pan', delta: [60, 0], smooth: true });
});

test('touchpad pixels pan 1:1 without smoothing', () => {
  expect(wheelIntent({ ...base, deltaX: 3.5, deltaY: -12.25 })).toEqual({ kind: 'pan', delta: [3.5, -12.25], smooth: false });
});

test('ctrl + wheel zooms in on scroll up and out on scroll down', () => {
  const zoomIn = wheelIntent({ ...base, deltaY: -100, ctrlKey: true });
  const zoomOut = wheelIntent({ ...base, deltaY: 100, ctrlKey: true });
  expect(zoomIn.kind === 'zoom' && zoomIn.factor).toBeCloseTo(Math.exp(0.18));
  expect(zoomOut.kind === 'zoom' && zoomOut.factor).toBeCloseTo(Math.exp(-0.18));
});

test('line-mode wheel counts three lines as one notch', () => {
  expect(wheelIntent({ ...base, deltaY: 3, deltaMode: 1 })).toEqual({ kind: 'pan', delta: [0, 60], smooth: true });
});
