// 큰 캔버스 화면을 오가고 카드를 끈 뒤 GC 후 힙이 회복되는지 검사한다.
// 사용: node scripts/test-memory.mjs [--no-build]
import { chromium } from 'playwright';
import { serveStorybook } from './lib/storybook-static.mjs';

const STORY = 'screens-window-states--canvas-large';
const CYCLES = 50;
const LIMIT = 0.10;
const storybook = await serveStorybook({ build: !process.argv.includes('--no-build') });
const browser = await chromium.launch({ headless: true, args: ['--js-flags=--expose-gc'] });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(`${storybook.url}/iframe.html?id=${STORY}&viewMode=story`);
  await page.waitForFunction(() => document.querySelectorAll('[data-note-id]').length >= 200, null, { timeout: 60000 });
  const baseline = await page.evaluate(async () => {
    for (let i = 0; i < 3; i++) { window.gc(); await new Promise((done) => setTimeout(done, 50)); }
    return performance.memory.usedJSHeapSize;
  });
  await page.evaluate(async (cycles) => {
    const switchTo = (kind) => window.dispatchEvent(new CustomEvent('memoivy:preview-window', { detail: { kind } }));
    const pointer = { bubbles: true, pointerId: 1, isPrimary: true, pointerType: 'mouse' };
    for (let i = 0; i < cycles; i++) {
      switchTo('library');
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
      switchTo('canvas');
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
      const card = document.querySelector('[data-note-id$="000000000021"]');
      const rect = card.getBoundingClientRect(); const x = rect.x + 80; const y = rect.y + 20;
      card.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, button: 0, buttons: 1, clientX: x, clientY: y }));
      card.dispatchEvent(new PointerEvent('pointermove', { ...pointer, button: 0, buttons: 1, clientX: x + 12, clientY: y + 8 }));
      card.dispatchEvent(new PointerEvent('pointerup', { ...pointer, button: 0, buttons: 0, clientX: x + 12, clientY: y + 8 }));
      await new Promise((done) => setTimeout(done, 0));
    }
  }, CYCLES);
  const after = await page.evaluate(async () => {
    for (let i = 0; i < 3; i++) { window.gc(); await new Promise((done) => setTimeout(done, 50)); }
    return performance.memory.usedJSHeapSize;
  });
  const delta = (after - baseline) / baseline;
  const passed = delta <= LIMIT;
  console.log(`${passed ? '✓' : '✗'} ${CYCLES}회 화면 전환·드래그 후 JS 힙: 시작 ${(baseline / 1048576).toFixed(1)}MB → 종료 ${(after / 1048576).toFixed(1)}MB · 변화 ${(delta * 100).toFixed(1)}% (기준 ±${LIMIT * 100}%)`);
  process.exitCode = passed ? 0 : 1;
} finally {
  await browser.close();
  storybook.close();
}
