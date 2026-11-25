// 스토어 스크린샷 4장 → ../screenshots/0N.png (web/shots.js가 그림). LIB=<lemo-opuscar> node shots.mjs
import path from 'node:path';
const { openDemo, closeServer } = await import(path.join(process.env.LIB, 'core/render/page.mjs'));
const here = path.dirname(new URL(import.meta.url).pathname);
const { browser, page } = await openDemo(path.join(here, 'web'), { w: 1920, h: 1080, q: 'shots=1' });
const n = await page.evaluate(() => window.DUR - 1);
for (let i = 1; i <= n; i++) { await page.evaluate(i => window.render(i), i); await page.screenshot({ path: path.join(here, '../screenshots', `0${i}.png`) }); }
await browser.close(); closeServer();
