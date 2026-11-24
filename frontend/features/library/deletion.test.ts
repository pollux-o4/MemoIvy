import { chromium, type Browser } from 'playwright';
import { createServer, type ViteDevServer } from 'vite';
import { afterAll, beforeAll, expect, test } from 'vitest';

let server: ViteDevServer;
let browser: Browser;
let url: string;
beforeAll(async () => {
  server = await createServer({ server: { host: '127.0.0.1', port: 0, strictPort: false } });
  await server.listen();
  const address = server.httpServer?.address();
  if (!address || typeof address === 'string') throw new Error('Missing test server port');
  url = `http://127.0.0.1:${address.port}`;
  browser = await chromium.launch({ headless: true });
});
afterAll(async () => { await browser?.close(); await server?.close(); });

test('library deletion confirmation is visible and sends deletion only after acceptance', async () => {
  const page = await browser.newPage({ viewport: { width: 440, height: 740 } });
  try {
    await page.goto(url);
    await page.locator('.note-more').first().click();
    await page.getByRole('menuitem', { name: '삭제…' }).click();
    const accept = page.getByRole('button', { name: '삭제', exact: true });
    expect(await accept.evaluate((element) => {
      const box = element.getBoundingClientRect();
      const x = box.left + box.width / 2;
      const y = box.top + box.height / 2;
      return x >= 0 && x < innerWidth && y >= 0 && y < innerHeight && element.contains(document.elementFromPoint(x, y));
    })).toBe(true);
    const before = await page.locator('.note-card').count();
    await accept.click();
    await expect.poll(() => page.locator('.note-card').count()).toBe(before - 1);
  } finally { await page.close(); }
});


test('failed library deletion keeps the card and displays the engine error', async () => {
  const page = await browser.newPage({ viewport: { width: 440, height: 740 } });
  try {
    await page.addInitScript(() => {
      const snapshot = { revision: 1, savedRevision: 1, assets: [], preferences: {}, notes: [{ id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1', text: '삭제 실패 보존', color: 'yellow', modified: 1, canvas: { position: null, next: [], done: false } }] };
      window.memoivy = {
        async request(op) { if (op === 'delete') throw new Error('삭제 저장 실패'); return { snapshot }; },
        subscribe() { return () => {}; },
        async openNote() {}, async openWindow() {}, async openExternal() {}, async importLegacy() { return {}; },
        windowAction() {}, onFlush() { return () => {}; }, onWindowState() { return () => {}; },
      };
    });
    await page.goto(url);
    await page.locator('.note-more').click();
    await page.getByRole('menuitem', { name: '삭제…' }).click();
    await page.getByRole('button', { name: '삭제', exact: true }).click();
    await page.getByRole('alert').waitFor();
    expect(await page.getByRole('alert').textContent()).toContain('삭제 저장 실패');
    expect(await page.locator('.note-card').count()).toBe(1);
  } finally { await page.close(); }
});
