import { type ChildProcess, spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { _electron as electron, type ElectronApplication, type Page } from 'playwright';
import { afterAll, beforeAll, describe, expect, test } from 'vitest';

const root = path.resolve(import.meta.dirname, '..');
// Wayland 세션에서 Electron은 DISPLAY(Xvfb)를 무시하고 실제 화면에 창을 띄워 입력을 가져간다.
// mise run test:e2e 만 가상 X 화면을 보장하므로 그 밖의 실행은 거부한다.
if (process.env.MEMOIVY_HEADLESS !== '1') throw new Error('Run via `mise run test:e2e` (virtual display only).');
const { WAYLAND_DISPLAY: _wayland, ...headlessEnv } = process.env;
const dataDir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-'));
let vite: ChildProcess;

async function waitForServer(url: string) {
  for (const end = Date.now() + 30_000; Date.now() < end; await new Promise((r) => setTimeout(r, 200))) {
    if (await fetch(url).then(() => true, () => false)) return;
  }
  throw new Error('vite dev server did not start');
}
async function launch(): Promise<{ app: ElectronApplication; library: Page }> {
  const app = await electron.launch({ args: ['--ozone-platform=x11', '.'], cwd: root, env: { ...headlessEnv, MEMOIVY_DATA_DIR: dataDir } as Record<string, string> });
  const library = await app.firstWindow();
  await library.waitForSelector('.library-screen');
  return { app, library };
}
/** 사용자의 앱 종료와 같은 경로(before-quit → 초안 저장 → 엔진 flush)로 끝낸다. app.close()는 창을 하나씩 닫아 목록 창이 남는다. */
async function quit(app: ElectronApplication) {
  const exited = new Promise((resolve) => app.process().once('exit', resolve));
  void app.evaluate(({ app: electronApp }) => electronApp.quit()).catch(() => undefined);
  await exited;
}
async function windowWith(app: ElectronApplication, selector: string): Promise<Page> {
  for (const end = Date.now() + 10_000; Date.now() < end; await new Promise((r) => setTimeout(r, 100))) {
    for (const page of app.windows()) if (await page.$(selector)) return page;
  }
  throw new Error(`no window with ${selector}`);
}

beforeAll(async () => {
  await new Promise<void>((resolve, reject) => spawn('cargo', ['build', '--manifest-path', 'engine/Cargo.toml', '--locked'], { cwd: root, stdio: 'inherit' }).on('exit', (code) => (code ? reject(new Error('engine build failed')) : resolve())));
  vite = spawn(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js')], { cwd: root, stdio: 'ignore' });
  await waitForServer('http://127.0.0.1:5173');
});
afterAll(() => { vite?.kill(); rmSync(dataDir, { recursive: true, force: true }); });

describe('데스크톱 정합 작동 (합성 데이터 폴더)', () => {
  test('메모 작성 → 다른 창 동기화 → 재시작 후 복원', async () => {
    const { app, library } = await launch();
    await library.getByRole('button', { name: 'New' }).click();
    await library.getByRole('menuitem', { name: /메모/ }).click();
    const editor = await windowWith(app, '.editor-screen');
    await editor.locator('.cm-content').click();
    await editor.keyboard.type('정합 확인 메모');
    await expect.poll(() => editor.locator('.save-label').textContent(), { timeout: 10_000 }).toBe('저장됨');
    await expect.poll(() => library.locator('.note-card').filter({ hasText: '정합 확인 메모' }).count()).toBe(1);
    await quit(app);

    const reopened = await launch();
    await expect.poll(() => reopened.library.locator('.note-card').filter({ hasText: '정합 확인 메모' }).count()).toBe(1);
    await quit(reopened.app);
  });

  test('인스펙터 소개 입력이 포커스를 잃기 전에도 카드에 실시간 반영되고 유지된다', async () => {
    const { app, library } = await launch();
    await library.getByRole('button', { name: '캔버스 열기' }).first().click();
    const canvas = await windowWith(app, '.canvas-screen');
    await canvas.getByRole('button', { name: 'New' }).click();
    await canvas.getByRole('menuitem', { name: /프로젝트/ }).click();
    const intro = canvas.getByRole('textbox', { name: '프로젝트 소개' });
    await intro.click();
    await intro.pressSequentially('실시간 소개 반영');
    // 포커스는 그대로 두고 카드가 따라오는지만 본다.
    await expect.poll(() => canvas.locator('[data-note-id]').filter({ hasText: '실시간 소개 반영' }).count(), { timeout: 3_000 }).toBeGreaterThan(0);
    await expect.poll(() => intro.textContent()).toBe('실시간 소개 반영');
    if (process.env.E2E_SHOT) await canvas.screenshot({ path: process.env.E2E_SHOT });
    // 이름이 빈 프로젝트라도 소개가 이름으로 올라가지 않는다.
    await expect.poll(() => canvas.getByRole('textbox', { name: '프로젝트 이름' }).inputValue()).toBe('이름 없는 프로젝트');
    // 프로젝트 설명은 드로어에서 전체 메모 편집기로 열 수 있다.
    await canvas.getByRole('button', { name: '메모로 열기' }).click();
    const editor = await windowWith(app, '.editor-screen');
    await expect.poll(() => editor.locator('.cm-content').textContent()).toContain('실시간 소개 반영');
    await quit(app);

    const reopened = await launch();
    await reopened.library.getByRole('button', { name: '캔버스 열기' }).first().click();
    const again = await windowWith(reopened.app, '.canvas-screen');
    await expect.poll(() => again.getByText('실시간 소개 반영').count()).toBeGreaterThan(0);
    await quit(reopened.app);
  });

  test('설정의 테마·모션 선택이 열린 창 전체에 반영되고 유지된다', async () => {
    const { app, library } = await launch();
    await library.keyboard.press('Control+,');
    const settings = await windowWith(app, '.settings-screen');
    await settings.getByRole('radio', { name: '다크' }).click();
    await settings.getByRole('radiogroup', { name: '모션 줄이기' }).getByRole('radio', { name: '켜기' }).click();
    await expect.poll(() => library.evaluate(() => [document.documentElement.classList.contains('theme-dark'), document.documentElement.dataset.motion])).toEqual([true, 'reduce']);
    await quit(app);

    const reopened = await launch();
    await expect.poll(() => reopened.library.evaluate(() => document.documentElement.classList.contains('theme-dark'))).toBe(true);
    await quit(reopened.app);
  });
});
