// 실제 앱 화면 캡처: 표본 데이터(seed.cjs)로 Electron을 가상 X 화면에서 띄워 shots/에 찍는다.
// 사용: env -u WAYLAND_DISPLAY xvfb-run -a -s "-screen 0 1920x1080x24" node capture.mjs
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { _electron as electron } from 'playwright';
import { createServer } from 'vite';

const HERE = import.meta.dirname, root = path.resolve(HERE, '../../../../..'), OUT = path.join(HERE, 'shots');
mkdirSync(OUT, { recursive: true });
const data = mkdtempSync(path.join(tmpdir(), 'memoivy-capture-'));
execFileSync('node', [path.join(HERE, 'seed.cjs'), data], { stdio: 'inherit' });
execFileSync('cargo', ['build', '--manifest-path', 'engine/Cargo.toml', '--locked'], { cwd: root, stdio: 'inherit' });
const vite = await createServer({ configFile: path.join(root, 'vite.config.ts'), server: { port: 0, watch: null } });
await vite.listen(0);
const url = `http://127.0.0.1:${vite.httpServer.address().port}`;
const app = await electron.launch({ args: ['--ozone-platform=x11', '.'], cwd: root, env: { ...process.env, MEMOIVY_DATA_DIR: data, XDG_DATA_HOME: data, SNAP_USER_DATA: '', MEMOIVY_RENDERER_URL: url } });
const shot = async (page, name) => { await page.waitForTimeout(700); await page.screenshot({ path: path.join(OUT, `${name}.png`) }); console.log('shot', name); };
const step = async (name, fn) => { try { await fn(); } catch (e) { console.log('skip', name, e.message.split('\n')[0]); } };
try {
  const lib = await app.firstWindow(); await lib.waitForSelector('.library-screen');
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows().forEach(w => w.setBounds({ x: 0, y: 0, width: 1280, height: 900 })));
  await shot(lib, '01-library');
  await step('search', async () => { await lib.locator('input').first().fill('숙소'); await shot(lib, '02-search'); await lib.locator('input').first().fill(''); });
  await step('expand', async () => { await lib.getByRole('button', { name: '주말 여행 펼치기' }).click(); await shot(lib, '03-library-expand'); });
  const win = async sel => { for (const end = Date.now() + 10000; Date.now() < end; await new Promise(r => setTimeout(r, 150))) for (const p of app.windows()) if (!p.isClosed() && await p.$(sel).catch(() => null)) return p; throw new Error('no ' + sel); };
  await step('canvas', async () => {
    await lib.getByRole('button', { name: '주말 여행 캔버스 열기' }).click();
    const cv = await win('.canvas-world'); await cv.setViewportSize?.({ width: 1280, height: 900 }).catch(() => {});
    await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows().forEach(w => w.setBounds({ x: 0, y: 0, width: 1280, height: 900 })));
    await shot(cv, '04-canvas');
    console.log(await cv.evaluate(() => [...document.querySelectorAll('[aria-label],button')].slice(0, 60).map(e => `${e.tagName}.${e.className.toString().slice(0, 30)} ${e.getAttribute('aria-label') || ''} ${(e.textContent || '').trim().slice(0, 24)}`).join('\n')));
    const card = cv.locator('text=주말 여행 준비').first(); await card.dblclick();
    const ed = await win('.cm-editor, [contenteditable=true]');
    await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows().forEach((w, i) => w.getTitle && null));
    await shot(ed, '05-editor');
    await ed.keyboard.press('Control+End'); await shot(ed, '06-editor-caret-end');
    await shot(cv, '07-canvas-after');
  });
  await step('photo', async () => { const cv = await win('.canvas-world'); await cv.locator('text=여행 사진').first().dblclick(); await new Promise(r => setTimeout(r, 800)); const ws = app.windows().filter(p => !p.isClosed()); await shot(ws[ws.length - 1], '08-editor-photo'); });
  await step('link', async () => { const cv = await win('.canvas-world'); await cv.locator('article:has-text("체크인 15시")').first().dblclick(); await new Promise(r => setTimeout(r, 2500)); const ws = app.windows().filter(p => !p.isClosed()); await shot(ws[ws.length - 1], '09-editor-link'); });
} finally { await app.close().catch(() => {}); await vite.close(); }
