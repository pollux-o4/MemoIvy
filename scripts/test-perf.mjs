// 캔버스 조작 부드러움 검사: 카드 200장·간선 400개 스토리에서 휠 이동·휠클릭 이동·Ctrl+휠 확대·카드 끌기를
// 헤드리스 브라우저로 재현하고 프레임당 작업 시간과 화면 이동 연속성을 예산과 비교한다.
// 사용: node scripts/test-perf.mjs [--no-build] [--scenario=zoom-burst]
import { chromium } from 'playwright';
import { serveStorybook } from './lib/storybook-static.mjs';

const STORY = 'screens-window-states--canvas-large';
/** 프레임 예산(ms). 60Hz 한 프레임 16.7ms 안에 그리기·합성 여유를 남긴다. */
const BUDGET = { avg: 10, p95: 16, click: 33 };
/** 한 프레임 이동량이 앞뒤 프레임 평균의 이 배수를 넘으면 튐(끊김)으로 본다. */
const JUMP_RATIO = 2.5;
const RUNS = 3;
const FRAMES = 120;

const scenes = [['휠 위아래 이동', 'wheel'], ['휠클릭 끌어 이동', 'pan'], ['Ctrl+휠 확대·축소', 'zoom'], ['고속 확대·축소 반전', 'zoom-burst'], ['카드 끌기', 'drag'], ['카드 선택·가지 초점 전환', 'focus']];
const requestedScenario = process.argv.find((arg) => arg.startsWith('--scenario='))?.slice('--scenario='.length);
const selectedScenes = scenes.filter(([, setup]) => !requestedScenario || requestedScenario === setup);
if (!selectedScenes.length) throw new Error(`Unknown scenario: ${requestedScenario}`);

const storybook = await serveStorybook({ build: !process.argv.includes('--no-build') });
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto(`${storybook.url}/iframe.html?id=${STORY}&viewMode=story`);
await page.waitForFunction(() => document.querySelectorAll('[data-note-id]').length >= 200, null, { timeout: 60000 });
await page.waitForTimeout(1000);

/**
 * 페이지 안에서 한 장면을 FRAMES 프레임 동안 돌린다. input(i)은 프레임마다 입력을 보낸다.
 * 프레임 작업 시간 = rAF 시작 → 입력 처리·React 반영 → 강제 스타일·레이아웃까지.
 * 이동량 = .canvas-world transform의 프레임 간 변화(화면 px).
 */
const scenario = (name, setup) => page.evaluate(async ({ name, setup, FRAMES }) => {
  const board = document.querySelector('.canvas-board');
  const world = document.querySelector('.canvas-world');
  const rect = board.getBoundingClientRect();
  const at = { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
  const pointer = { bubbles: true, pointerId: 1, isPrimary: true, pointerType: 'mouse' };
  const card = document.querySelector('[data-note-id$="000000000021"]');
  const offset = () => { const m = new DOMMatrixReadOnly(getComputedStyle(world).transform); return [m.e, m.f, m.a]; };
  const burstZoom = (deltaY) => board.dispatchEvent(new WheelEvent('wheel', { bubbles: true, cancelable: true, ctrlKey: true, clientX: at.x, clientY: at.y, deltaY, deltaMode: 0 }));
  const inputs = {
    wheel: { start() {}, frame(i) { if (i % 3 === 0 && i < FRAMES - 40) board.dispatchEvent(new WheelEvent('wheel', { bubbles: true, cancelable: true, clientX: at.x, clientY: at.y, deltaY: i % 60 < 30 ? 100 : -100, deltaMode: 0 })); }, end() {} },
    zoom: { start() {}, frame(i) { if (i % 2 === 0) board.dispatchEvent(new WheelEvent('wheel', { bubbles: true, cancelable: true, ctrlKey: true, clientX: at.x, clientY: at.y, deltaY: i % 60 < 30 ? -20 : 20, deltaMode: 0 })); }, end() {} },
    'zoom-burst': {
      start() {},
      frame(i) { for (let j = 0; j < 4; j++) burstZoom(i % 16 < 8 ? -32 : 32); },
      end() {},
    },
    pan: {
      start() { this.x = at.x; this.y = at.y; board.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, button: 1, buttons: 4, clientX: this.x, clientY: this.y })); },
      // 고빈도 마우스처럼 한 프레임에 이동 이벤트를 세 번 보낸다.
      frame(i) { for (let j = 0; j < 3; j++) { this.x += i % 60 < 30 ? 2 : -2; this.y += i % 40 < 20 ? 1 : -1; board.dispatchEvent(new PointerEvent('pointermove', { ...pointer, button: 1, buttons: 4, clientX: this.x, clientY: this.y })); } },
      end() { board.dispatchEvent(new PointerEvent('pointerup', { ...pointer, button: 1, buttons: 0, clientX: this.x, clientY: this.y })); },
    },
    drag: {
      start() { const r = card.getBoundingClientRect(); this.x = r.x + 80; this.y = r.y + 20; card.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, button: 0, buttons: 1, clientX: this.x, clientY: this.y })); },
      frame(i) { this.x += i % 60 < 30 ? 4 : -4; this.y += i % 40 < 20 ? 2 : -2; card.dispatchEvent(new PointerEvent('pointermove', { ...pointer, button: 0, buttons: 1, clientX: this.x, clientY: this.y })); },
      end() { card.dispatchEvent(new PointerEvent('pointerup', { ...pointer, button: 0, buttons: 0, clientX: this.x, clientY: this.y })); },
    },
    // 사람이 고르는 속도(0.5초마다 한 번)로 다른 가지를 고르고, 클릭 프레임과 이어지는 흐림 전환 프레임을 모두 잰다.
    focus: {
      start() {},
      frame(i) {
        if (i % 30 !== 0) return;
        const target = document.querySelector(`[data-note-id$="${String([0, 100, 1, 101][(i / 30) % 4]).padStart(12, '0')}"]`);
        const r = target.getBoundingClientRect(); const x = r.x + 80; const y = r.y + 20;
        target.dispatchEvent(new PointerEvent('pointerdown', { ...pointer, button: 0, buttons: 1, clientX: x, clientY: y }));
        target.dispatchEvent(new PointerEvent('pointerup', { ...pointer, button: 0, buttons: 0, clientX: x, clientY: y }));
      },
      end() {},
    },
  };
  const input = inputs[setup];
  const nextFrame = () => new Promise((done) => requestAnimationFrame(done));
  const afterTasks = () => new Promise((done) => { const channel = new MessageChannel(); channel.port1.onmessage = () => setTimeout(done, 0); channel.port2.postMessage(0); });
  input.start(); await nextFrame();
  const work = []; const steps = []; let last = offset();
  for (let i = 0; i < FRAMES; i++) {
    await nextFrame();
    const t0 = performance.now();
    input.frame(i);
    await afterTasks();
    document.body.offsetHeight;
    work.push(performance.now() - t0);
    const now = offset(); steps.push(Math.hypot(now[0] - last[0], now[1] - last[1])); last = now;
  }
  input.end();
  await new Promise((done) => setTimeout(done, 600));
  const sorted = [...work].sort((a, b) => a - b);
  // 튐: 한 프레임 이동량이 앞뒤 프레임 평균보다 얼마나 큰가. 스프링 가감속처럼 서서히 변하면 1 근처다.
  let jump = 0;
  for (let i = 1; i < steps.length - 1; i++) jump = Math.max(jump, steps[i] / Math.max((steps[i - 1] + steps[i + 1]) / 2, 0.5));
  return { name, avg: work.reduce((a, b) => a + b, 0) / work.length, p95: sorted[Math.floor(sorted.length * 0.95)], max: sorted.at(-1), jump, moved: steps.filter((step) => step > 0.01).length };
}, { name, setup, FRAMES });

let failed = false;
for (const [name, setup] of selectedScenes) {
  const runs = [];
  for (let k = 0; k < RUNS; k++) runs.push(await scenario(name, setup));
  // 부하에 흔들리는 측정이라 세 번 중 중앙값으로 판정한다.
  const mid = (key) => runs.map((r) => r[key]).sort((a, b) => a - b)[RUNS >> 1];
  const result = { avg: mid('avg'), p95: mid('p95'), max: mid('max'), jump: mid('jump'), moved: mid('moved') };
  const problems = [];
  if (result.avg > BUDGET.avg) problems.push(`평균 ${result.avg.toFixed(1)}ms > ${BUDGET.avg}`);
  if (result.p95 > BUDGET.p95) problems.push(`p95 ${result.p95.toFixed(1)}ms > ${BUDGET.p95}`);
  // 클릭 프레임은 p95에 묻히므로 따로 본다: 선택 한 번에 프레임 하나 넘게 밀리지 않는다.
  if (setup === 'focus' && result.max > BUDGET.click) problems.push(`클릭 프레임 ${result.max.toFixed(1)}ms > ${BUDGET.click}`);
  if (!['drag', 'focus'].includes(setup) && result.moved < FRAMES / 4) problems.push(`화면이 거의 움직이지 않음(${result.moved}프레임)`);
  if (setup === 'wheel' || setup === 'pan') if (result.jump > JUMP_RATIO) problems.push(`이동 튐 ${result.jump.toFixed(1)}배 > ${JUMP_RATIO}`);
  failed ||= problems.length > 0;
  console.log(`${problems.length ? '✗' : '✓'} ${name}: 평균 ${result.avg.toFixed(1)}ms · p95 ${result.p95.toFixed(1)}ms${setup === 'focus' ? ` · 클릭 ${result.max.toFixed(1)}ms` : ''}${setup === 'wheel' || setup === 'pan' ? ` · 튐 ${result.jump.toFixed(1)}배` : ''}${problems.length ? `  — ${problems.join(', ')}` : ''}`);
}
await browser.close();
storybook.close();
process.exit(failed ? 1 : 0);
