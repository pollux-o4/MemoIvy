import { chromium, type Browser, type Page } from 'playwright';
import { afterAll, beforeAll, beforeEach, expect, test } from 'vitest';
import { createServer, type ViteDevServer } from 'vite';

let server: ViteDevServer;
let browser: Browser;
let page: Page;
beforeAll(async () => {
  server = await createServer({ server: { port: 0, strictPort: false }, cacheDir: '/tmp/memoivy-m2-vite' });
  await server.listen();
  browser = await chromium.launch({ headless: true });
  page = await browser.newPage();
});
afterAll(async () => { await browser?.close(); await server?.close(); });
beforeEach(async () => {
  await page.goto(server.resolvedUrls?.local[0] ?? '');
  await page.evaluate(async () => {
    const load = new Function('path', 'return import(path)');
    const React = (await load('/@id/react')).default;
    const { createRoot } = (await load('/@id/react-dom/client')).default;
    const { ConfirmDialog } = await load('/shared/ConfirmDialog.tsx');
    document.body.innerHTML = '<button id="outside">outside</button><div id="fixture"></div>';
    createRoot(document.getElementById('fixture')).render(React.createElement(ConfirmDialog, {
      question: '이 메모를 삭제할까요?', detail: '삭제 확인', acceptLabel: '삭제',
      onCancel: () => document.body.dataset.result = 'cancel',
      onAccept: () => document.body.dataset.result = 'accept',
    }));
  });
  await page.getByRole('button', { name: '취소', exact: true }).waitFor();
});

test('dialog arrows and Home/End move focus without accepting', async () => {
  const cancel = page.getByRole('button', { name: '취소', exact: true });
  const accept = page.getByRole('button', { name: '삭제', exact: true });
  await expect.poll(() => cancel.evaluate((button) => button === document.activeElement)).toBe(true);
  for (const [key, button] of [['ArrowRight', accept], ['ArrowRight', cancel], ['ArrowLeft', accept], ['Home', cancel], ['End', accept]] as const) {
    await page.keyboard.press(key);
    expect(await button.evaluate((element) => element === document.activeElement)).toBe(true);
  }
  expect(await page.locator('body').getAttribute('data-result')).toBeNull();
});

test('Tab stays inside the dialog in both directions, including outside focus', async () => {
  for (const [key, name] of [['Shift+Tab', '삭제'], ['Tab', '취소'], ['Tab', '삭제'], ['Tab', '취소']] as const) {
    await page.keyboard.press(key);
    expect(await page.getByRole('button', { name, exact: true }).evaluate((element) => element === document.activeElement)).toBe(true);
  }
  await page.locator('#outside').focus();
  await page.keyboard.press('Tab');
  expect(await page.getByRole('button', { name: '취소', exact: true }).evaluate((element) => element === document.activeElement)).toBe(true);
  await page.keyboard.press('Escape');
  expect(await page.locator('body').getAttribute('data-result')).toBe('cancel');
});

test.each([true, false])('vertical hook skips unavailable buttons and handles boundaries (loop=%s)', async (loop) => {
  await page.evaluate(async (loop) => {
    const load = new Function('path', 'return import(path)');
    const React = (await load('/@id/react')).default;
    const { createRoot } = (await load('/@id/react-dom/client')).default;
    const { useRovingFocus } = await load('/shared/useRovingFocus.ts');
    // biome-ignore lint/correctness/noComponentHookFactories: 브라우저 실행 컨텍스트에 독립 테스트 컴포넌트를 한 번 마운트한다.
    function Fixture() {
      const ref = React.useRef(null);
      useRovingFocus(ref, { orientation: 'vertical', loop });
      return React.createElement('div', { ref, id: 'roving' },
        ...['first', 'disabled', 'hidden', 'last'].map((id) => React.createElement('button', { id, key: id, disabled: id === 'disabled', hidden: id === 'hidden' }, id)));
    }
    const host = document.createElement('div');
    document.body.append(host);
    createRoot(host).render(React.createElement(Fixture));
  }, loop);
  await page.locator('#first').focus();
  await page.keyboard.press('ArrowRight');
  expect(await page.locator('#first').evaluate((e) => e === document.activeElement)).toBe(true);
  for (const [key, id] of [['ArrowDown', 'last'], ['ArrowDown', loop ? 'first' : 'last'], ['ArrowUp', loop ? 'last' : 'first'], ['Home', 'first'], ['End', 'last']] as const) {
    await page.keyboard.press(key);
    expect(await page.locator(`#${id}`).evaluate((e) => e === document.activeElement)).toBe(true);
  }
  await page.locator('#last').evaluate((e) => { (e as HTMLButtonElement).disabled = true; });
  await page.locator('#first').focus();
  await page.keyboard.press('ArrowDown');
  expect(await page.locator('#first').evaluate((e) => e === document.activeElement)).toBe(true);
});
