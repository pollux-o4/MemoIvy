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
    // 연결선 모양은 엔진 설정으로 저장되어 오류 없이 선택 상태가 바뀐다.
    await settings.getByRole('radio', { name: '직각' }).click();
    await expect.poll(() => settings.getByRole('radio', { name: '직각' }).getAttribute('aria-checked')).toBe('true');
    expect(await settings.getByRole('alert').count()).toBe(0);
    await quit(app);

    const reopened = await launch();
    await expect.poll(() => reopened.library.evaluate(() => document.documentElement.classList.contains('theme-dark'))).toBe(true);
    await quit(reopened.app);
  });
  test('캔버스 마우스 가지: 손잡이 누르기·빈 곳에 놓기·카드에 놓기', async () => {
    const { app, library } = await launch();
    await library.getByRole('button', { name: '캔버스 열기' }).first().click();
    const canvas = await windowWith(app, '.canvas-screen');
    const cards = canvas.locator('[data-note-id]');
    const ids = () => cards.evaluateAll((items) => items.map((item) => (item as HTMLElement).dataset.noteId ?? ''));
    const linkCount = () => canvas.locator('.canvas-link').count();
    const before = await ids();
    await canvas.getByRole('button', { name: 'New' }).click();
    await canvas.getByRole('menuitem', { name: /메모/ }).click();
    await expect.poll(async () => (await ids()).length).toBe(before.length + 1);
    const root = (await ids()).find((id) => !before.includes(id)) ?? '';
    const card = (id: string) => canvas.locator(`[data-note-id="${id}"]`);
    const handle = (id: string) => card(id).locator('.card-branch');
    const box = async (locator: ReturnType<typeof card>) => { const found = await locator.boundingBox(); if (!found) throw new Error('보이지 않는 요소'); return found; };
    await canvas.locator('.canvas-screen').press('Escape');
    // 누르기: 하위 가지가 생기고 선 하나가 이어진다.
    const links0 = await linkCount();
    await card(root).hover();
    await handle(root).click();
    await expect.poll(async () => (await ids()).length).toBe(before.length + 2);
    await expect.poll(linkCount).toBe(links0 + 1);
    // 빈 곳에 놓기: 끄는 동안 미리보기 선이 보이고, 놓은 자리에 새 가지가 생긴다.
    await card(root).hover();
    const start = await box(handle(root));
    await canvas.mouse.move(start.x + start.width / 2, start.y + start.height / 2);
    await canvas.mouse.down();
    await canvas.mouse.move(start.x + 60, start.y + 240, { steps: 8 });
    await expect.poll(() => canvas.locator('.canvas-connect-preview path').count()).toBe(1);
    await canvas.mouse.up();
    await expect.poll(async () => (await ids()).length).toBe(before.length + 3);
    await expect.poll(linkCount).toBe(links0 + 2);
    await expect.poll(() => canvas.locator('.canvas-connect-preview').count()).toBe(0);
    // 카드에 놓기: 새 카드 없이 연결만 는다.
    const [first, second] = (await ids()).filter((id) => !before.includes(id) && id !== root);
    await card(second).hover();
    const from = await box(handle(second));
    const to = await box(card(first));
    await canvas.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    await canvas.mouse.down();
    await canvas.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 8 });
    await expect.poll(() => card(first).getAttribute('class')).toContain('link-target');
    await canvas.mouse.up();
    await expect.poll(linkCount).toBe(links0 + 3);
    expect((await ids()).length).toBe(before.length + 3);
    await quit(app);
  });

  test('캔버스 마인드맵: Tab·Enter 가지, 가지 개요, 실행 취소, 프로젝트 제자리 펼치기', async () => {
    const { app, library } = await launch();
    await library.getByRole('button', { name: '캔버스 열기' }).first().click();
    const canvas = await windowWith(app, '.canvas-screen');
    const cards = canvas.locator('[data-note-id]');
    const ids = () => cards.evaluateAll((items) => items.map((item) => (item as HTMLElement).dataset.noteId ?? ''));
    const before = await ids();
    await canvas.getByRole('button', { name: 'New' }).click();
    await canvas.getByRole('menuitem', { name: /메모/ }).click();
    await expect.poll(async () => (await ids()).length).toBe(before.length + 1);
    const root = (await ids()).find((id) => !before.includes(id)) ?? '';
    const card = (id: string) => canvas.locator(`[data-note-id="${id}"]`);
    const outlineDepths = () => canvas.locator('.inspector-outline li').evaluateAll((items) => items.map((item) => (item as HTMLElement).style.getPropertyValue('--depth')));

    // Tab: 하위 가지를 만들고 바로 이름 칸에 포커스한다.
    await card(root).click();
    await canvas.keyboard.press('Tab');
    await expect.poll(async () => (await ids()).length).toBe(before.length + 2);
    await expect.poll(() => canvas.evaluate(() => document.activeElement?.classList.contains('inspector-title'))).toBe(true);
    const child = (await ids()).find((id) => !before.includes(id) && id !== root) ?? '';
    // Enter: 같은 부모의 형제. 부모 없는 카드의 Enter는 그대로 열기라 자식 카드에서 누른다.
    await card(child).click();
    await canvas.keyboard.press('Enter');
    await expect.poll(async () => (await ids()).length).toBe(before.length + 3);
    await expect.poll(outlineDepths).toEqual(['0', '1', '1']);
    // 가지 개요 들여쓰기: 두 번째 가지를 첫 가지 아래로.
    await canvas.locator('.inspector-outline button').nth(2).focus();
    await canvas.keyboard.press('Tab');
    await expect.poll(outlineDepths).toEqual(['0', '1', '2']);
    await canvas.keyboard.press('Shift+Tab');
    await expect.poll(outlineDepths).toEqual(['0', '1', '1']);
    // 위 형제가 없는 첫 가지: Tab은 아무것도 바꾸지 않고 포커스도 그 자리에 둔다.
    const firstBranch = canvas.locator('.inspector-outline button').nth(1);
    await firstBranch.focus();
    await canvas.keyboard.press('Tab');
    await expect.poll(outlineDepths).toEqual(['0', '1', '1']);
    await expect.poll(() => firstBranch.evaluate((item) => item === document.activeElement)).toBe(true);
    await expect.poll(() => canvas.locator('.inspector-outline').locator('xpath=..').locator('.inspector-hint').textContent()).toBe('위에 같은 단계 항목이 없어요');
    // 마인드맵 배치 후 실행 취소로 원위치.
    await card(root).click();
    const placed = await card(child).evaluate((item) => item.style.cssText);
    await canvas.keyboard.press('Alt+m');
    await canvas.keyboard.press('Control+z');
    await expect.poll(() => card(child).evaluate((item) => item.style.cssText)).toBe(placed);
    // 프로젝트 제자리 펼치기: 묶은 프로젝트를 펼치면 주변은 흐려지고 Esc로 접힌다.
    await card(child).click();
    await canvas.keyboard.press('Alt+g');
    const project = canvas.locator('.canvas-note.project').filter({ has: canvas.locator('.canvas-peek') }).last();
    await project.locator('.canvas-peek button').click();
    await expect.poll(() => project.getAttribute('class')).toContain('is-peeking');
    await expect.poll(() => canvas.locator('.canvas-world.has-focus > .canvas-note:not(.in-focus)').count()).toBeGreaterThan(0);
    await canvas.locator('.canvas-screen').press('Escape');
    await expect.poll(() => project.getAttribute('class')).not.toContain('is-peeking');
    // 보이는 입구: 인스펙터의 하위 가지 버튼도 Tab과 같이 가지를 만든다(화면이 새 가지로 옮겨 가므로 마지막에 둔다).
    await card(root).click();
    const count = (await ids()).length;
    await canvas.locator('.canvas-inspector').getByRole('button', { name: /하위 가지/ }).click();
    await expect.poll(async () => (await ids()).length).toBe(count + 1);
    await quit(app);
  });
});
