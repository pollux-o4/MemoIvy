import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { _electron as electron, chromium, type ElectronApplication, type Page } from 'playwright';
import { createServer, type ViteDevServer } from 'vite';
import { afterAll, beforeAll, describe, expect, test } from 'vitest';

const root = path.resolve(import.meta.dirname, '..');
const attachmentHash = 'dc76b5742600dc4aa297c35649b08ed26ee3b9f7972166b061752681cb134eb8';
const attachmentFile = path.join(root, 'fixtures/attachments', `${attachmentHash}.png`);
// Wayland 세션에서 Electron은 DISPLAY(Xvfb)를 무시하고 실제 화면에 창을 띄워 입력을 가져간다.
// mise run test:e2e 만 가상 X 화면을 보장하므로 그 밖의 실행은 거부한다.
if (process.env.MEMOIVY_HEADLESS !== '1') throw new Error('Run via `mise run test:e2e` (virtual display only).');
const { WAYLAND_DISPLAY: _wayland, ...headlessEnv } = process.env;
const dataDir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-'));
let vite: ViteDevServer | undefined;
let rendererUrl: string;

/** 카메라 애니메이션이 멈출 때까지 기다린다(두 번 연속 같은 transform). */
async function settle(page: Page): Promise<void> {
  const world = () => page.locator('.canvas-world').evaluate((item) => (item as HTMLElement).style.transform);
  let previous = '';
  await expect.poll(async () => { const now = await world(); const same = now === previous; previous = now; return same; }, { intervals: [120] }).toBe(true);
}
async function launch(dir = dataDir): Promise<{ app: ElectronApplication; library: Page }> {
  const app = await electron.launch({ args: ['--ozone-platform=x11', '.'], cwd: root, env: { ...headlessEnv, MEMOIVY_DATA_DIR: dir, XDG_DATA_HOME: dir, SNAP_USER_DATA: '', MEMOIVY_RENDERER_URL: rendererUrl } as Record<string, string> });
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
    // 목록↔캔버스 전환은 원래 창을 닫으므로 닫히는 중인 페이지는 건너뛴다.
    for (const page of app.windows()) if (!page.isClosed() && await page.$(selector).catch(() => null)) return page;
  }
  throw new Error(`no window with ${selector}`);
}

beforeAll(async () => {
  await new Promise<void>((resolve, reject) => spawn('cargo', ['build', '--manifest-path', 'engine/Cargo.toml', '--locked'], { cwd: root, stdio: 'inherit' }).on('exit', (code) => (code ? reject(new Error('engine build failed')) : resolve())));
  vite = await createServer({ configFile: path.join(root, 'vite.config.ts'), server: { port: 0, watch: null } });
  await vite.listen(0);
  const address = vite.httpServer?.address();
  if (!address || typeof address === 'string') throw new Error('Vite did not expose a test server port');
  rendererUrl = `http://127.0.0.1:${address.port}`;
});
afterAll(async () => {
  await vite?.close();
  rmSync(dataDir, { recursive: true, force: true });
});

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
    await editor.keyboard.press('F1');
    const noteHelp = editor.getByRole('dialog', { name: '단축키와 서식' });
    await expect.poll(() => noteHelp.getByRole('radio', { name: '메모' }).getAttribute('aria-checked')).toBe('true');
    const narrowBounds = await noteHelp.evaluate((item) => {
      const box = item.getBoundingClientRect();
      return box.left >= 10 && box.right <= innerWidth - 10 && box.top >= 10 && box.bottom <= innerHeight - 10;
    });
    expect(narrowBounds).toBe(true);
    await editor.keyboard.press('Escape');
    await noteHelp.waitFor({ state: 'hidden' });
    await quit(app);

    const reopened = await launch();
    await expect.poll(() => reopened.library.locator('.note-card').filter({ hasText: '정합 확인 메모' }).count()).toBe(1);
    await quit(reopened.app);
  });

  // 검증: DATA-001, DATA-003. 파일 선택만 대신하고 나머지는 실제 IPC와 엔진으로 실행한다.
  test.each(['자동', '수동'])('이전 메모 %s 가져오기는 원본과 첨부를 보존한다', async (mode) => {
    const temp = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-import-'));
    const dir = path.join(temp, 'data');
    const source = mode === '자동' ? dir : path.join(temp, 'legacy');
    const id = 'd0000000-0000-4000-8000-000000000001';
    const filename = path.basename(attachmentFile);
    const bytes = readFileSync(attachmentFile);
    const notes = path.join(source, 'notes.json');
    const asset = path.join(source, 'assets', filename);
    const title = '이전 메모 가져오기 검증';
    const text = `${title}\n\n![가져온 첨부](asset:${attachmentHash})`;
    const original = JSON.stringify([{ id, text, color: 'blue', modified: 100 }]);
    mkdirSync(path.dirname(asset), { recursive: true });
    writeFileSync(notes, original);
    writeFileSync(asset, bytes);

    let running: ElectronApplication | undefined;
    try {
      const { app, library } = await launch(dir);
      running = app;
      if (mode === '수동') {
        await app.evaluate(({ dialog }, file) => {
          dialog.showOpenDialog = async () => ({ canceled: false, filePaths: [file] });
        }, notes);
        await library.getByRole('button', { name: '설정', exact: true }).click();
        const settings = await windowWith(app, '.settings-screen');
        await settings.getByRole('button', { name: '가져오기', exact: true }).click();
        const status = settings.getByRole('status');
        await expect.poll(() => status.textContent(), { timeout: 30_000 }).not.toMatch(/^$|가져오는 중/);
        if (await status.textContent() !== '이전 메모를 가져왔습니다.') {
          // 화면 문구만으로는 원인을 알 수 없어 같은 경로를 다시 불러 엔진 오류를 실패 메시지에 남긴다.
          const cause = await settings.evaluate(() => window.memoivy?.importLegacy().then(() => 'ok', (error: Error) => `${(error as { code?: string }).code}: ${error.message}`));
          throw new Error(`가져오기 실패(${await status.textContent()}) — ${cause}`);
        }
      }
      await library.getByRole('button', { name: title, exact: true }).waitFor();
      await quit(app);
      running = undefined;

      const reopened = await launch(dir);
      running = reopened.app;
      await reopened.library.getByRole('button', { name: title, exact: true }).click();
      const editor = await windowWith(reopened.app, '.editor-screen');
      const image = editor.getByAltText('가져온 첨부');
      await expect.poll(() => image.evaluate((element) => (element as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
      expect(readFileSync(notes, 'utf8')).toBe(original);
      expect(readFileSync(`${notes}.memoivy-backup`, 'utf8')).toBe(original);
      expect(readFileSync(asset)).toEqual(bytes);
    } finally {
      if (running) await quit(running);
      rmSync(temp, { recursive: true, force: true });
    }
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
    await library.getByRole('button', { name: '캔버스 열기' }).first().click();
    const canvas = await windowWith(app, '.canvas-screen');
    const settingsButton = canvas.getByRole('button', { name: '설정', exact: true });
    await settingsButton.focus();
    await settingsButton.press('Enter');
    const settings = await windowWith(app, '.settings-screen');
    await settings.getByRole('radio', { name: '다크' }).click();
    await settings.getByRole('radiogroup', { name: '모션 줄이기' }).getByRole('radio', { name: '켜기' }).click();
    // 목록 창은 캔버스로 전환되며 닫혔으므로 열린 캔버스 창으로 전체 반영을 확인한다.
    await expect.poll(() => canvas.evaluate(() => [document.documentElement.classList.contains('theme-dark'), document.documentElement.dataset.motion])).toEqual([true, 'reduce']);
    // 연결선 모양은 엔진 설정으로 저장되어 오류 없이 선택 상태가 바뀐다.
    await settings.getByRole('radio', { name: '직각' }).click();
    await expect.poll(() => settings.getByRole('radio', { name: '직각' }).getAttribute('aria-checked')).toBe('true');
    expect(await settings.getByRole('alert').count()).toBe(0);
    await quit(app);

    const reopened = await launch();
    await expect.poll(() => reopened.library.evaluate(() => document.documentElement.classList.contains('theme-dark'))).toBe(true);
    await quit(reopened.app);
  });
  test('원점에서 끌어 음수 좌표로 옮기면 새로고침 및 앱 재시작 후 유지된다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-drag-'));
    const id = 'd0000000-0000-4000-8000-000000000001';
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([
      { id, text: '위치 확인 메모', color: 'yellow', modified: 1794576000, canvas: { position: [0, 0], next: [], done: false } },
    ]));
    const openCanvas = async (app: ElectronApplication, library: Page) => {
      await library.getByRole('button', { name: '캔버스 열기' }).first().click();
      return windowWith(app, '.canvas-screen');
    };
    const cardIn = (page: Page) => page.locator(`.canvas-note[data-note-id="${id}"]`);
    const positionIn = (page: Page) => cardIn(page).evaluate((item: HTMLElement) => [item.offsetLeft, item.offsetTop]);
    let running: ElectronApplication | undefined;
    try {
      const { app, library } = await launch(dir);
      running = app;
      const canvas = await openCanvas(app, library);
      await cardIn(canvas).waitFor();
      await settle(canvas);
      expect(await positionIn(canvas)).toEqual([0, 0]);
      const box = await cardIn(canvas).boundingBox();
      if (!box) throw new Error('카드가 보이지 않음');
      const x = box.x + box.width / 2;
      const y = box.y + box.height / 2;
      await canvas.mouse.move(x, y);
      await canvas.mouse.down();
      await canvas.mouse.move(x - 80, y - 65, { steps: 8 });
      await canvas.mouse.up();

      // 저장 완료 신호는 다른 렌더러가 받은 엔진 스냅숏으로 확인한다.
      const persisted = () => canvas.evaluate(async (noteId) => {
        const result = await window.memoivy?.request('snapshot');
        return result?.snapshot?.notes.find((note) => note.id === noteId)?.canvas.position;
      }, id);
      await expect.poll(async () => (await persisted())?.every((axis) => axis < 0)).toBe(true);
      const saved = await persisted();
      await canvas.reload();
      await expect.poll(() => positionIn(canvas)).toEqual(saved);
      await quit(app);
      running = undefined;

      const reopened = await launch(dir);
      running = reopened.app;
      const restored = await openCanvas(reopened.app, reopened.library);
      await expect.poll(() => positionIn(restored)).toEqual(saved);
    } finally {
      if (running) await quit(running);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('마지막 메모·캔버스 창을 닫을 때만 메모 목록으로 돌아간다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-close-canvas-'));
    const { app, library } = await launch(dir);
    try {
      await library.getByRole('button', { name: '캔버스 열기' }).first().click();
      const canvas = await windowWith(app, '.canvas-screen');
      await expect.poll(() => library.isClosed()).toBe(true);
      // 캔버스가 남아 있으면 메모 창을 닫아도 목록이 뜨지 않는다.
      await canvas.getByRole('button', { name: 'New' }).click();
      await canvas.getByRole('menuitem', { name: /메모/ }).click();
      const editor = await windowWith(app, '.editor-screen');
      await editor.getByRole('button', { name: '창 닫기' }).click();
      await expect.poll(() => editor.isClosed()).toBe(true);
      const libraryVisible = () => app.evaluate(({ BrowserWindow }) =>
        BrowserWindow.getAllWindows().some((w) => w.isVisible() && w.webContents.getURL().includes('window=library')));
      expect(await libraryVisible()).toBe(false);
      // 메모가 남아 있으면 캔버스를 닫아도 목록이 뜨지 않고, 마지막 메모를 닫으면 뜬다.
      await canvas.evaluate(async () => {
        const bridge = window.memoivy;
        if (!bridge) throw new Error('데스크톱 bridge 없음');
        const { snapshot } = await bridge.request('snapshot') as { snapshot: { notes: { id: string }[] } };
        await bridge.openNote(snapshot.notes[0].id);
      });
      const last = await windowWith(app, '.editor-screen');
      // OS 닫기(Alt+F4)도 닫기 버튼과 같은 경로를 지난다.
      await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows().find((w) => w.webContents.getURL().includes('window=canvas'))?.close());
      await expect.poll(() => canvas.isClosed()).toBe(true);
      expect(await libraryVisible()).toBe(false);
      await last.getByRole('button', { name: '창 닫기' }).click();
      const again = await windowWith(app, '.library-screen');
      await expect.poll(() => last.isClosed()).toBe(true);
      expect(again.isClosed()).toBe(false);
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('메모 창의 이동 메뉴는 펼쳐져 캔버스나 메모 목록으로 보낸다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-note-nav-'));
    const { app, library } = await launch(dir);
    try {
      await library.getByRole('button', { name: 'New' }).click();
      await library.getByRole('menuitem', { name: /메모/ }).click();
      const editor = await windowWith(app, '.editor-screen');
      // 마우스: 펼친 메뉴에 두 갈래가 모두 보이고, 캔버스를 고르면 메모 창은 남긴 채 목록 자리를 캔버스가 차지한다.
      await editor.getByRole('button', { name: '이동' }).click();
      const toCanvas = editor.getByRole('menuitem', { name: '캔버스', exact: true });
      const toLibrary = editor.getByRole('menuitem', { name: '메모 목록' });
      await expect.poll(() => Promise.all([toCanvas.isVisible(), toLibrary.isVisible()])).toEqual([true, true]);
      await toCanvas.click();
      const canvas = await windowWith(app, '.canvas-screen');
      // 목록과 캔버스는 홈 자리 하나를 나눠 쓴다.
      await expect.poll(() => library.isClosed()).toBe(true);
      expect(editor.isClosed()).toBe(false);
      // 키보드: 메뉴가 첫 항목에 포커스를 준 뒤에 화살표로 옮겨야 입력이 메뉴에 닿는다.
      await editor.getByRole('button', { name: '이동' }).click();
      await expect.poll(() => toCanvas.evaluate((node) => node === document.activeElement)).toBe(true);
      await editor.keyboard.press('ArrowDown');
      await expect.poll(() => toLibrary.evaluate((node) => node === document.activeElement)).toBe(true);
      await editor.keyboard.press('Enter');
      await windowWith(app, '.library-screen');
      await expect.poll(() => canvas.isClosed()).toBe(true);
      expect(editor.isClosed()).toBe(false);
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('삭제는 Ctrl+Z로 원래 id·연결 그대로 되살아나고 그 전 기록도 이어서 되돌린다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-undo-delete-'));
    const kept = 'd0000000-0000-4000-8000-000000000001';
    const removed = 'd0000000-0000-4000-8000-000000000002';
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([
      { id: kept, text: '남길 메모', color: 'yellow', modified: 100, canvas: { position: [0, 0], next: [removed] } },
      { id: removed, text: '지울 메모', color: 'blue', modified: 100, canvas: { position: [400, 0], next: [] } },
    ]));
    const { app, library } = await launch(dir);
    try {
      await library.getByRole('button', { name: '캔버스 열기' }).first().click();
      const canvas = await windowWith(app, '.canvas-screen');
      const card = (id: string) => canvas.locator(`.canvas-note[data-note-id="${id}"]`);
      await card(removed).waitFor();
      await settle(canvas);
      // 지울 메모를 옮겨 실행 취소 기록 맨 위에 그 메모의 변경을 남긴다.
      const box = await card(removed).boundingBox();
      if (!box) throw new Error('카드가 보이지 않음');
      await canvas.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await canvas.mouse.down();
      await canvas.mouse.move(box.x + box.width / 2 + 60, box.y + box.height / 2 + 40, { steps: 6 });
      await canvas.mouse.up();
      await card(removed).click();
      await canvas.locator('.canvas-screen').press('Delete');
      await canvas.getByRole('button', { name: '삭제', exact: true }).click();
      await expect.poll(() => card(removed).count()).toBe(0);
      await canvas.locator('.canvas-screen').press('Control+z');
      await new Promise((resolve) => setTimeout(resolve, 500));
      expect(await canvas.getByRole('alert').count()).toBe(0);
      await expect.poll(() => card(removed).count()).toBe(1);
      const links = async () => (await canvas.evaluate(() => window.memoivy.request('snapshot'))).snapshot?.notes.find((note) => note.id === 'd0000000-0000-4000-8000-000000000001')?.canvas.next;
      await expect.poll(links).toEqual([removed]);
      // 삭제 앞의 이동 기록도 되살아난 메모에 이어서 적용된다.
      const moved = await card(removed).boundingBox();
      await canvas.locator('.canvas-screen').press('Control+z');
      await expect.poll(async () => (await card(removed).boundingBox())?.x).not.toBe(moved?.x);
      await expect.poll(() => canvas.locator('.canvas-status').textContent()).not.toContain('실패');
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  // 검증: PROJECT-001. 큰 보드에서 나갈 때 보던 배율과 위치를 복원한다.
  test('프로젝트 복귀 시 이전 캔버스 위치와 확대율을 복원한다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-scope-camera-'));
    const project = 'd0000000-0000-4000-8000-000000000001';
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([
      { id: project, text: '복귀 프로젝트', color: 'yellow', modified: 100, canvas: { project: true, position: [170, 130] } },
      { id: 'd0000000-0000-4000-8000-000000000002', text: '내부 메모', color: 'blue', modified: 100, canvas: { parent: project, position: [400, 200] } },
      { id: 'd0000000-0000-4000-8000-000000000003', text: '멀리 있는 메모', color: 'blue', modified: 100, canvas: { position: [2400, 1800] } },
    ]));
    const { app, library } = await launch(dir);
    try {
      await library.getByRole('button', { name: '캔버스 열기', exact: true }).click();
      const canvas = await windowWith(app, '.canvas-screen');
      const camera = () => canvas.locator('.canvas-world').evaluate((world) => {
        const matrix = new DOMMatrix(getComputedStyle(world).transform);
        return [matrix.a, matrix.e, matrix.f];
      });
      await canvas.getByRole('button', { name: '확대', exact: true }).click();
      await expect.poll(async () => (await camera())[0]).toBeGreaterThan(1.1);
      await settle(canvas);
      const before = await camera();
      const enter = canvas.getByRole('button', { name: '복귀 프로젝트 프로젝트 들어가기', exact: true });
      const home = canvas.getByRole('navigation', { name: '프로젝트 경로' }).getByRole('button', { name: '전체', exact: true });
      const keyboardLeave = async () => {
        const child = canvas.locator('.canvas-note').filter({ hasText: '내부 메모' });
        await child.click();
        await expect.poll(() => child.evaluate((card) => card.classList.contains('selected'))).toBe(true);
        await canvas.locator('.canvas-screen').press('Alt+ArrowUp');
        const snapshot = await canvas.evaluate(() => window.memoivy.request('snapshot'));
        expect(snapshot.snapshot?.notes.find((note) => note.text === '내부 메모')?.canvas.position).toEqual([400, 200]);
      };
      // 마우스 뒤로 버튼은 키보드 뒤로와 같은 명령이다(Playwright 마우스는 뒤로 버튼이 없어 이벤트로 보낸다).
      const mouseBack = () => canvas.evaluate(() => {
        for (const type of ['mousedown', 'mouseup']) window.dispatchEvent(new MouseEvent(type, { button: 3, bubbles: true, cancelable: true }));
      });
      for (const leave of [() => home.click(), keyboardLeave, mouseBack]) {
        await enter.click();
        await canvas.locator('.canvas-note').getByText('내부 메모', { exact: true }).waitFor();
        await settle(canvas);
        await leave();
        await expect.poll(() => home.getAttribute('aria-current')).toBe('page');
        await expect.poll(async () => (await camera()).map((value, index) => Math.abs(value - before[index]) < (index === 0 ? 0.005 : 1))).toEqual([true, true, true]);
        await expect.poll(() => canvas.locator(`.canvas-note.selected[data-note-id="${project}"]`).count()).toBe(1);
        await settle(canvas);
      }
      // 최상위에서 뒤로 가면 들어왔던 목록으로 돌아가고 캔버스는 닫힌다.
      await mouseBack();
      await windowWith(app, '.library-screen');
      await expect.poll(() => app.windows().filter((page) => !page.isClosed() && page.url().includes('window=canvas')).length).toBe(0);
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  // 미리보기 창은 메모가 마운트 때 바로 있어 연결 신호 rAF가 StrictMode 재마운트 정리 중에 취소된다(Wayland 실기에서 발견).
  test('캔버스 휠·확대 버튼: 마운트 직후 예약한 프레임이 취소돼도 화면이 움직인다', async () => {
    const browser = await chromium.launch();
    try {
      const canvas = await browser.newPage({ viewport: { width: 900, height: 600 } });
      await canvas.goto(`${rendererUrl}/?window=canvas`);
      await canvas.locator('.canvas-link').first().waitFor();
      const world = () => canvas.locator('.canvas-world').evaluate((item) => (item as HTMLElement).style.transform);
      const before = await world();
      const box = await canvas.locator('.canvas-board').boundingBox();
      await canvas.mouse.move((box?.x ?? 0) + 40, (box?.y ?? 0) + 40);
      await canvas.mouse.wheel(0, 120);
      await expect.poll(world, { timeout: 3_000 }).not.toBe(before);
      await canvas.getByRole('button', { name: '확대', exact: true }).click();
      await expect.poll(() => canvas.locator('.zoom-value').textContent(), { timeout: 3_000 }).not.toBe('100%');
      await settle(canvas);
      const anchor = async () => canvas.locator('.canvas-board').evaluate((board) => {
        const world = board.querySelector<HTMLElement>('.canvas-world');
        const transform = new DOMMatrixReadOnly(world?.style.transform);
        return { zoom: transform.a, x: (140 - transform.e) / transform.a, y: (160 - transform.f) / transform.a };
      });
      const initial = await anchor();
      const delta = initial.zoom > 1 ? 6 : -6;
      await canvas.locator('.canvas-board').evaluate((board, deltaY) => {
        const rect = board.getBoundingClientRect();
        for (let i = 0; i < 6; i += 1) board.dispatchEvent(new WheelEvent('wheel', {
          bubbles: true, cancelable: true, ctrlKey: true, deltaY, clientX: rect.left + 140, clientY: rect.top + 160,
        }));
      }, delta);
      await expect.poll(async () => (await anchor()).zoom).toBeCloseTo(initial.zoom * Math.exp(-delta * .012 * 6), 4);
      const after = await anchor();
      expect(after.x).toBeCloseTo(initial.x, 2);
      expect(after.y).toBeCloseTo(initial.y, 2);
    } finally { await browser.close(); }
  });
  test('캔버스 손잡이 누르기: 고르기 상태에서 카드를 누르면 연결만 는다', async () => {
    const browser = await chromium.launch();
    try {
      const canvas = await browser.newPage({ viewport: { width: 1400, height: 900 } });
      await canvas.goto(`${rendererUrl}/?window=canvas`);
      await canvas.locator('.canvas-link').first().waitFor();
      const links = () => canvas.locator('.canvas-link').count();
      const cards = () => canvas.locator('[data-note-id]').count();
      const [links0, cards0] = [await links(), await cards()];
      const from = canvas.locator('[data-note-id]').filter({ hasText: '아이디어를 모으고' });
      const to = canvas.locator('[data-note-id]').filter({ hasText: '오늘의 작은 기록' });
      const center = async (locator: typeof from) => { const b = await locator.boundingBox(); if (!b) throw new Error('보이지 않는 요소'); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; };
      const c = await center(from); await canvas.mouse.move(c.x, c.y);
      const h = await center(from.locator('.card-branch') as typeof from); await canvas.mouse.click(h.x, h.y);
      await expect.poll(() => canvas.locator('.canvas-pick-hint').count()).toBe(1);
      const t = await center(to); await canvas.mouse.click(t.x, t.y);
      await expect.poll(links).toBe(links0 + 1);
      expect(await cards()).toBe(cards0);
      expect(await canvas.locator('.canvas-pick-hint').count()).toBe(0);
    } finally { await browser.close(); }
  });
  test('캔버스 좁은 창: 고른 카드가 인스펙터에 가리면 보이는 곳까지 옮긴다', async () => {
    const browser = await chromium.launch();
    try {
      const canvas = await browser.newPage({ viewport: { width: 700, height: 600 } });
      await canvas.goto(`${rendererUrl}/?window=canvas`);
      await canvas.locator('.canvas-link').first().waitFor();
      await canvas.setViewportSize({ width: 360, height: 600 });
      const title = await canvas.locator('.canvas-header .window-title').boundingBox();
      const settings = await canvas.getByRole('button', { name: '설정', exact: true }).boundingBox();
      const create = await canvas.getByRole('button', { name: 'New' }).boundingBox();
      if (!title || !settings || !create) throw new Error('캔버스 제목 줄이 보이지 않음');
      expect(title.x + title.width).toBeLessThan(settings.x);
      expect(create.x - (settings.x + settings.width)).toBeGreaterThanOrEqual(6);
      expect(Math.abs((settings.y + settings.height / 2) - (create.y + create.height / 2))).toBeLessThan(1);
      expect(create.x + create.width).toBeLessThan(360);
      await canvas.setViewportSize({ width: 700, height: 600 });
      await settle(canvas);
      const rightmost = canvas.locator('[data-note-id]').filter({ hasText: '생각을 연결하고' });
      const inside = await rightmost.boundingBox();
      if (!inside) throw new Error('카드가 보이지 않음');
      await canvas.mouse.click(inside.x + 20, inside.y + 20);
      const gap = async () => {
        const card = await rightmost.boundingBox(); const panel = await canvas.locator('.canvas-inspector').boundingBox();
        return card && panel ? panel.x - (card.x + card.width) : Number.NEGATIVE_INFINITY;
      };
      await expect.poll(gap, { timeout: 3_000 }).toBeGreaterThanOrEqual(0);
    } finally { await browser.close(); }
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
    const tapHandle = async (id: string) => { await hoverCard(id); const found = await box(handle(id)); await canvas.mouse.click(found.x + found.width / 2, found.y + found.height / 2); };
    const hoverCard = async (id: string) => { const found = await box(card(id)); await canvas.mouse.move(found.x + found.width / 2, found.y + found.height / 2); };
    // 새 카드는 골라지며 인스펙터가 열리고 카메라가 따라간다. 다음 조작 전에 선택을 풀고 전체를 보이게 한 뒤 멈출 때까지 기다린다.
    const overview = async () => {
      // 선택이 없을 때 Esc는 창을 닫으므로 인스펙터가 열려 있을 때만 누른다.
      // 이름 칸에 포커스가 있으면 첫 Esc는 포커스만 푼다.
      for (let i = 0; i < 3 && await canvas.locator('.canvas-inspector').count(); i += 1) await canvas.locator('.canvas-screen').press('Escape');
      await canvas.getByRole('button', { name: '전체 보기' }).click();
      await settle(canvas);
    };
    await overview();
    // 누르기: 바로 만들지 않고 고르기 상태가 된다. Esc는 취소, 빈 곳을 누르면 그 자리에 하위 가지.
    const links0 = await linkCount();
    const hint = canvas.locator('.canvas-pick-hint');
    await tapHandle(root);
    await expect.poll(() => hint.count()).toBe(1);
    expect((await ids()).length).toBe(before.length + 1);
    await canvas.keyboard.press('Escape');
    await expect.poll(() => hint.count()).toBe(0);
    await tapHandle(root);
    // 빈 곳: 보드 자체가 맞는 점을 찾는다(카드·패널 위를 누르면 연결이나 다른 동작이 된다).
    const empty = await canvas.locator('.canvas-board').evaluate((board) => {
      const r = board.getBoundingClientRect();
      for (let y = r.top + 60; y < r.bottom - 80; y += 40) for (let x = r.left + 40; x < r.right - 40; x += 40) {
        if (document.elementFromPoint(x, y) === board) return { x, y };
      }
      throw new Error('빈 곳 없음');
    });
    await canvas.mouse.click(empty.x, empty.y);
    await expect.poll(async () => (await ids()).length).toBe(before.length + 2);
    await expect.poll(linkCount).toBe(links0 + 1);
    await expect.poll(() => hint.count()).toBe(0);
    await overview();
    // 빈 곳에 놓기: 끄는 동안 미리보기 선이 보이고, 놓은 자리에 새 가지가 생긴다.
    await hoverCard(root);
    const start = await box(handle(root));
    await canvas.mouse.move(start.x + start.width / 2, start.y + start.height / 2);
    await canvas.mouse.down();
    await canvas.mouse.move(start.x + 60, start.y + 240, { steps: 8 });
    await expect.poll(() => canvas.locator('.canvas-connect-preview path').count()).toBe(1);
    await canvas.mouse.up();
    await expect.poll(async () => (await ids()).length).toBe(before.length + 3);
    await expect.poll(linkCount).toBe(links0 + 2);
    await expect.poll(() => canvas.locator('.canvas-connect-preview').count()).toBe(0);
    await overview();
    // 카드에 놓기: 새 카드 없이 연결만 는다.
    const [first, second] = (await ids()).filter((id) => !before.includes(id) && id !== root);
    await hoverCard(second);
    const from = await box(handle(second));
    const to = await box(card(first));
    await canvas.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
    await canvas.mouse.down();
    await canvas.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 8 });
    await expect.poll(() => card(first).getAttribute('class')).toContain('link-target');
    await canvas.mouse.up();
    await expect.poll(linkCount).toBe(links0 + 3);
    expect((await ids()).length).toBe(before.length + 3);
    await overview();
    await quit(app);
  });

  test('캔버스 마인드맵: Tab·Enter 가지, 가지 개요, 실행 취소, 프로젝트 미리보기', async () => {
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
    // 카드를 눌러 고른 뒤, 이름 칸 같은 입력에서 포커스가 빠질 때까지 기다린다. 그 전에 누른 키는 입력이 먹는다.
    const select = async (id: string) => {
      await card(id).click();
      await expect.poll(() => canvas.evaluate(() => !document.activeElement?.matches('input, textarea, [contenteditable="true"]'))).toBe(true);
    };
    const outlineDepths = () => canvas.locator('.inspector-outline li').evaluateAll((items) => items.map((item) => (item as HTMLElement).style.getPropertyValue('--depth')));

    // Tab: 하위 가지를 만들고 바로 이름 칸에 포커스한다.
    await select(root);
    await canvas.keyboard.press('Tab');
    await expect.poll(async () => (await ids()).length).toBe(before.length + 2);
    await expect.poll(() => canvas.evaluate(() => document.activeElement?.classList.contains('inspector-title'))).toBe(true);
    const child = (await ids()).find((id) => !before.includes(id) && id !== root) ?? '';
    // Enter: 같은 부모의 형제. 부모 없는 카드의 Enter는 그대로 열기라 자식 카드에서 누른다.
    await select(child);
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
    await select(root);
    const placed = await card(child).evaluate((item) => item.style.cssText);
    await canvas.keyboard.press('Alt+m');
    await canvas.keyboard.press('Control+z');
    await expect.poll(() => card(child).evaluate((item) => item.style.cssText)).toBe(placed);
    // 하위 메모 수는 정보로 남고, 종이 더미는 호버와 키보드 포커스로 미리 본다.
    await select(child);
    await canvas.keyboard.press('Alt+g');
    const project = canvas.locator('.canvas-note.project').filter({ hasText: '메모 1개' }).last();
    expect(await project.locator('.canvas-project-meta button').count()).toBe(0);
    const fanTitleOpacity = () => project.evaluate((item) => {
      const title = item.nextElementSibling?.querySelector('.canvas-fan-title');
      return title ? getComputedStyle(title).opacity : null;
    });
    await project.hover();
    await expect.poll(fanTitleOpacity).toBe('1');
    await canvas.getByRole('button', { name: '전체 보기' }).hover();
    await expect.poll(fanTitleOpacity).toBe('0');
    await project.locator('button').first().focus();
    await canvas.keyboard.press('Shift+Tab');
    await expect.poll(fanTitleOpacity).toBe('1');
    // 보이는 입구: 인스펙터의 하위 가지 버튼도 Tab과 같이 가지를 만든다(화면이 새 가지로 옮겨 가므로 마지막에 둔다).
    await select(root);
    const count = (await ids()).length;
    await canvas.locator('.canvas-inspector').getByRole('button', { name: /하위 가지/ }).click();
    await expect.poll(async () => (await ids()).length).toBe(count + 1);
    await quit(app);
  });

  test('좁고 낮은 캔버스에서도 펼침 버튼이 본문 위로 이동하고 카메라는 고정된다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-preview-size-'));
    const id = 'd0000000-0000-4000-8000-000000000001';
    const text = ['긴 메모', ...Array.from({ length: 32 }, (_, index) => `- [ ] 항목 ${index + 1}`)].join('\n');
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([{ id, text, color: 'blue', modified: 100, canvas: { position: [0, 0] } }]));
    const { app, library } = await launch(dir);
    try {
      await library.getByRole('button', { name: '캔버스 열기' }).first().click();
      const canvas = await windowWith(app, '.canvas-screen');
      const window = await app.browserWindow(canvas);
      const card = canvas.getByRole('article', { name: '긴 메모 카드' });
      const preview = card.locator('.canvas-checklist-preview');
      const disclosure = card.locator('.preview-disclosure button');
      const camera = () => canvas.locator('.canvas-world').evaluate((world: HTMLElement) => world.style.transform);
      for (const size of [[460, 420], [980, 360], [420, 660]]) {
        await window.evaluate((win, [width, height]) => win.setSize(width, height), size);
        await expect.poll(() => canvas.evaluate(() => [innerWidth, innerHeight])).toEqual(size);
        await canvas.getByRole('button', { name: '전체 보기' }).click();
        await settle(canvas);
        const initialCamera = await camera();
        const initialButton = await disclosure.boundingBox();
        const initialCard = await card.boundingBox();
        const triggerAboveBody = () => disclosure.evaluate((button) => {
          const body = button.closest('.preview-frame')?.querySelector('.preview-list');
          return Boolean(body && button.getBoundingClientRect().bottom <= body.getBoundingClientRect().top + 1);
        });
        await disclosure.click();
        await expect.poll(() => disclosure.getAttribute('aria-expanded')).toBe('true');
        await expect.poll(() => preview.evaluate((list) => list.scrollHeight > list.clientHeight)).toBe(true);
        await expect.poll(triggerAboveBody).toBe(true);
        expect((await card.boundingBox())?.y).toBe(initialCard?.y);
        expect(await disclosure.evaluate((button) => document.activeElement === button)).toBe(true);
        expect(await camera()).toBe(initialCamera);
        await preview.hover({ position: { x: 30, y: 20 } });
        await canvas.mouse.wheel(0, 250);
        await expect.poll(() => preview.evaluate((list) => list.scrollTop)).toBeGreaterThan(0);
        expect(await camera()).toBe(initialCamera);
        await disclosure.press('Space');
        await expect.poll(() => disclosure.getAttribute('aria-expanded')).toBe('false');
        await expect.poll(() => preview.evaluate((list) => list.scrollTop)).toBe(0);
        await expect.poll(() => disclosure.boundingBox()).toEqual(initialButton);
      }
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('링크 미리보기 카드 유지와 인라인 주소 수정', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-link-preview-'));
    const id = 'd0000000-0000-4000-8000-000000000002';
    const text = '링크 확인\n[링크](https://example.com)\n[네이버](naver.com)';
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([{ id, text, color: 'blue', modified: 100, canvas: { position: [0, 0] } }]));
    const { app, library } = await launch(dir);
    try {
      await library.getByRole('button', { name: '링크 확인', exact: true }).click();
      const editor = await windowWith(app, '.editor-screen');
      const content = editor.locator('.cm-content');
      const preview = editor.locator('.link-preview-card');
      await app.evaluate(({ shell }) => {
        shell.openExternal = async (url) => { process.env.MEMOIVY_TEST_OPENED_URL = url; };
      });
      for (const label of ['링크', '네이버']) {
        await content.getByText(label, { exact: true }).hover();
        await preview.getByText(label, { exact: true }).waitFor({ timeout: 3_000 });
        await new Promise((resolve) => setTimeout(resolve, 300));
        expect(await preview.getByText(label, { exact: true }).isVisible()).toBe(true);
        await preview.hover({ position: { x: 2, y: 2 } });
        // 링크 전환과 닫기 타이머가 모두 만료될 시간을 준다.
        await new Promise((resolve) => setTimeout(resolve, 450));
        expect(await preview.getByText(label, { exact: true }).isVisible()).toBe(true);
        if (label === '링크') {
          await preview.getByRole('button', { name: '열기', exact: true }).click({ timeout: 3_000 });
          await expect.poll(() => app.evaluate(() => process.env.MEMOIVY_TEST_OPENED_URL)).toBe('https://example.com');
        }
        await content.getByText('링크 확인', { exact: true }).hover();
        await preview.waitFor({ state: 'hidden' });
      }
      const editAddress = async () => {
        await content.getByText('네이버', { exact: true }).hover();
        await preview.getByRole('button', { name: '주소 수정' }).click();
        return preview.getByRole('textbox', { name: '링크 주소' });
      };
      const input = await editAddress();
      expect(await input.inputValue()).toBe('naver.com');
      await input.fill('naver.com/news');
      await content.getByText('링크 확인', { exact: true }).hover();
      await input.press('Escape');
      await preview.waitFor({ state: 'hidden' });
      expect(await (await editAddress()).inputValue()).toBe('naver.com');
      await input.fill('naver.com/news');
      await input.press('Enter');
      await preview.waitFor({ state: 'hidden' });
      await expect.poll(() => library.evaluate(async (id) => {
        const { snapshot } = await window.memoivy.request('snapshot');
        return snapshot?.notes.find((note) => note.id === id)?.text;
      }, id)).toBe(text.replace('(naver.com)', '(https://naver.com/news)'));
      await content.getByText('링크 확인', { exact: true }).hover();
      await content.getByText('네이버', { exact: true }).hover();
      await preview.getByText('naver.com/news', { exact: true }).waitFor({ timeout: 3_000 });
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('코드 펜스는 Enter로 확정할 때만 블록이 되고 아래 블록을 건드리지 않는다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-fence-'));
    const id = 'd0000000-0000-4000-8000-000000000001';
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([{ id, text: '펜스 확인\n위 글\n```\n아래 코드\n```\n아래 글', color: 'blue', modified: 100 }]));
    const { app, library } = await launch(dir);
    try {
      await library.getByRole('button', { name: '펜스 확인', exact: true }).click();
      const editor = await windowWith(app, '.editor-screen');
      const content = editor.locator('.cm-content');
      // 펜스 줄은 기호를 숨겨 빈 줄로 그린다.
      const codeTexts = () => content.locator('.md-code-block').allTextContents();
      await expect.poll(codeTexts).toEqual(['', '아래 코드', '']);
      await content.getByText('위 글', { exact: true }).click();
      await editor.keyboard.press('End');
      await editor.keyboard.press('Enter');
      await editor.keyboard.type('```');
      // 입력 중에는 아래 블록이 그대로이고, 새 줄은 일반 글이다.
      await expect.poll(codeTexts).toEqual(['', '아래 코드', '']);
      await editor.keyboard.press('Enter');
      await editor.keyboard.type('새 코드');
      // 커서가 든 새 블록은 펜스 기호를 드러내므로 블록 구조(두 블록 · 각 3줄)만 본다.
      await expect.poll(async () => (await codeTexts()).map((text) => text.replace(/^`{3}$/, ''))).toEqual(['', '새 코드', '', '', '아래 코드', '']);
      await expect.poll(() => content.locator('.cm-line').last().textContent()).toBe('아래 글');
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('이미지 옆 한글 조합과 후속 입력이 첨부를 보존한다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-editor-ime-'));
    const id = 'd0000000-0000-4000-8000-000000000001';
    const image = `![첨부](asset:${attachmentHash})`;
    const data = readFileSync(attachmentFile).toString('base64');
    const markdown = [
      '입력 확인', '**굵게**와 `인라인`',
      '- [ ] 실제 할 일', '  - [x] 하위 할 일',
      '```md', '- [ ] 코드 예시', '```', image,
    ].join('\n');
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([{ id, text: markdown, color: 'blue', modified: 100, canvas: { position: [0, 0] } }]));
    const { app, library } = await launch(dir);
    try {
      await library.evaluate(async (data) => {
        const { snapshot } = await window.memoivy.request('snapshot');
        await window.memoivy.request('importAsset', { name: '첨부.png', mime: 'image/png', data, baseRevision: snapshot?.revision });
      }, data);
      await library.getByRole('button', { name: '입력 확인', exact: true }).click();
      const editor = await windowWith(app, '.editor-screen');
      const content = editor.locator('.cm-content');
      const preview = content.locator('.cm-asset-preview img');
      await preview.waitFor({ timeout: 8_000 });
      await expect.poll(() => preview.evaluate((element) => element.naturalWidth)).toBeGreaterThan(0);
      expect(await content.locator('.md-checkbox').count()).toBe(2);
      await editor.keyboard.press('Control+End');
      const input = await app.context().newCDPSession(editor);
      try {
        for (const text of ['ㅎ', '하', '한']) {
          await input.send('Input.imeSetComposition', { text, selectionStart: 1, selectionEnd: 1 });
        }
        await input.send('Input.insertText', { text: '한' });
      } finally {
        await input.detach();
      }
      await expect.poll(() => content.locator('.cm-line').last().textContent()).toBe('한');
      await editor.keyboard.insertText(' 이어서 작성');
      await expect.poll(() => content.locator('.cm-line').last().textContent()).toBe('한 이어서 작성');
      await expect.poll(() => preview.count()).toBe(1);
      await expect.poll(async () => {
        const result = await library.evaluate(() => window.memoivy.request('snapshot'));
        return result.snapshot?.notes.find((note) => note.id === id)?.text;
      }, { timeout: 8_000 }).toBe(`${markdown}\n한 이어서 작성`);
      await editor.reload({ timeout: 8_000 });
      await expect.poll(() => content.locator('.cm-line').allTextContents()).toContain('한 이어서 작성');
      await expect.poll(() => preview.count()).toBe(1);
      expect(await content.locator('.md-checkbox').count()).toBe(2);
      const title = content.getByText('입력 확인', { exact: true });
      await preview.dragTo(title, { sourcePosition: { x: 40, y: 30 }, targetPosition: { x: 20, y: 2 } });
      await editor.keyboard.press('Control+Shift+m');
      await expect.poll(() => content.locator('.cm-line').first().textContent()).toBe(image);
      await editor.keyboard.press('Control+z');
      await expect.poll(() => content.locator('.cm-line').first().textContent()).toBe('입력 확인');
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('목록 본문은 목록 표식을 유지하고 프로젝트를 펼치면 소개의 할 일을 보여준다', async () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-markdown-preview-'));
    const project = 'd0000000-0000-4000-8000-000000000001';
    const note = 'd0000000-0000-4000-8000-000000000002';
    writeFileSync(path.join(dir, 'notes.json'), JSON.stringify([
      { id: 'd0000000-0000-4000-8000-000000000003', text: '최근 메모', color: 'yellow', modified: 200 },
      { id: project, text: '작업 프로젝트\n- [ ] 소개 할 일\n- [ ] 다음 할 일', color: 'yellow', modified: 100, canvas: { project: true } },
      { id: note, text: '작업 메모\n- [ ] 본문 할 일\n## 작은 제목\n  - 하위 글머리\n1. 번호 항목', color: 'blue', modified: 100, canvas: { parent: project } },
    ]));
    const { app, library } = await launch(dir);
    try {
      const window = await app.browserWindow(library);
      await window.evaluate((win) => win.setSize(360, 420));
      await expect.poll(() => library.evaluate(() => [innerWidth, innerHeight])).toEqual([360, 420]);
      const roots = library.getByRole('region', { name: '메모와 프로젝트' });
      await expect.poll(() => roots.locator(':scope > * .project-name, :scope > .note-card .card-title').allTextContents()).toEqual(['작업 프로젝트', '최근 메모']);
      const projectDisclosure = library.locator('.project-section > .disclosure');
      const beforeProject = await projectDisclosure.boundingBox();
      await library.getByRole('button', { name: '작업 프로젝트 펼치기' }).click();
      expect(await projectDisclosure.boundingBox()).toEqual(beforeProject);
      const intro = library.locator('.project-intro-preview');
      const tasks = intro.getByRole('checkbox');
      await tasks.first().waitFor();
      await tasks.evaluateAll((buttons) => {
        for (const button of buttons) (button as HTMLButtonElement).click();
      });
      await expect.poll(() => tasks.evaluateAll((buttons) => buttons.map((button) => button.getAttribute('aria-checked')))).toEqual(['true', 'true']);
      expect(await library.getByRole('alert').count()).toBe(0);

      const card = library.locator('.note-card').filter({ has: library.getByRole('button', { name: '작업 메모', exact: true }) });
      const gauge = card.getByRole('progressbar', { name: '작업 메모 진행률' });
      await expect.poll(() => gauge.getAttribute('aria-valuenow')).toBe('0');
      expect(await gauge.getAttribute('aria-valuemax')).toBe('1');
      expect(await roots.getByRole('progressbar', { name: '최근 메모 진행률' }).count()).toBe(0);
      const gaugeOffset = await gauge.evaluate((element: HTMLElement) => element.offsetTop);
      const preview = card.locator('.preview-list');
      const noteTask = preview.getByRole('checkbox', { name: '본문 할 일' });
      await noteTask.click();
      await expect.poll(() => noteTask.getAttribute('aria-checked')).toBe('true');
      const disclosure = card.locator('.preview-disclosure button');
      const hitSize = () => disclosure.evaluate((button) => {
        const area = getComputedStyle(button, '::before');
        return [area.width, area.height];
      });
      await expect.poll(hitSize).toEqual(['36px', '36px']);
      const beforeNote = await disclosure.boundingBox();
      await disclosure.click();
      await expect.poll(async () => (await disclosure.boundingBox())?.y ?? Infinity).toBeLessThan(beforeNote?.y ?? 0);
      expect(await preview.locator('.preview-heading').textContent()).toBe('작은 제목');
      expect(await preview.locator('.preview-bullet .preview-marker').textContent()).toBe('•');
      expect(await preview.locator('.preview-number .preview-marker').textContent()).toBe('1.');
      expect(await preview.locator('.preview-bullet').evaluate((row) => parseFloat(getComputedStyle(row).paddingInlineStart))).toBeGreaterThan(0);
      await expect.poll(() => card.locator('.task-chip').textContent()).toBe('1/1');
      expect(await gauge.getAttribute('aria-valuenow')).toBe('1');
      expect(await gauge.evaluate((element: HTMLElement) => element.offsetTop)).toBe(gaugeOffset);
      await expect.poll(() => library.locator('.project-heading .task-chip').textContent()).toBe('3/3');
      expect(app.windows().filter((page) => page.url().includes('window=note'))).toHaveLength(0);
      // 본문 영역의 포인터는 열기 버튼의 클릭 영역으로 전달된다.
      const heading = preview.locator('.preview-heading');
      const center = (item: Element) => {
        const { x, y, width, height } = item.getBoundingClientRect();
        const point = { x: x + width / 2, y: y + height / 2 };
        const button = document.elementFromPoint(point.x, point.y)?.closest('.card-open');
        return button && item.closest('.note-card')?.contains(button) ? point : null;
      };
      await heading.scrollIntoViewIfNeeded();
      await expect.poll(() => heading.evaluate(center)).not.toBeNull();
      const point = await heading.evaluate(center);
      if (!point) throw new Error('목록 본문이 보이지 않음');
      await library.mouse.click(point.x, point.y);
      const editor = await windowWith(app, '.editor-screen');
      await expect.poll(() => editor.locator('.md-checkbox.is-checked').count()).toBe(1);
      // 목록·캔버스는 동시에 열리지 않으므로(NOTE-001) 캔버스로 바꿔 같은 진행률을 확인한다.
      await library.evaluate((id) => window.memoivy?.openWindow('canvas', id), project);
      const canvas = await windowWith(app, '.canvas-screen');
      await expect.poll(() => canvas.getByRole('progressbar', { name: '작업 메모 진행률' }).getAttribute('aria-valuenow')).toBe('1');
    } finally {
      await quit(app);
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('캔버스 연결선: 표본 데이터의 화살촉이 카드·프로젝트 종이 더미에 가리지 않는다', async () => {
    // 개발 표본(펼친 미리보기·프로젝트·순환·여럿→한)으로 연결선 끝을 화면에서 직접 검사한다.
    const { sampleNotes } = createRequire(import.meta.url)('../tools/dev-data/sample_notes.cjs') as { sampleNotes: (names?: string[]) => unknown[] };
    const sampleDir = mkdtempSync(path.join(tmpdir(), 'memoivy-e2e-sample-'));
    writeFileSync(path.join(sampleDir, 'notes.json'), JSON.stringify(sampleNotes(['linkCases', 'linkEdgeCases'])));
    const { app, library } = await launch(sampleDir);
    try {
      await library.getByRole('button', { name: '캔버스 열기' }).first().click();
      const canvas = await windowWith(app, '.canvas-screen');
      await expect.poll(() => canvas.locator('.canvas-link path').count()).toBeGreaterThan(10);
      const checklist = canvas.getByRole('article', { name: '펼치면 길어지는 메모 카드' });
      const footerIsSeparated = () => checklist.evaluate((card) => {
        const tasks = card.querySelector('.canvas-card-meta small')?.getBoundingClientRect();
        const links = card.querySelector('footer button')?.getBoundingClientRect();
        return Boolean(tasks && links && tasks.right < links.left);
      });
      await expect.poll(footerIsSeparated).toBe(true);
      await settle(canvas);
      // 전체 보기 버튼이 실제로 카메라를 옮긴다(클릭 이벤트가 '선택만 맞추기' 인자로 새던 회귀 방지).
      const zoomOf = () => canvas.locator('.canvas-world').evaluate((item) => (item as HTMLElement).style.transform);
      const initial = await zoomOf();
      await canvas.getByRole('button', { name: '전체 보기' }).click();
      await expect.poll(zoomOf).not.toBe(initial);
      await settle(canvas);
      // 포인터가 움직여도 접힌 카드의 조작 영역은 같은 화면 좌표를 유지한다.
      const branchHandle = checklist.locator('.card-branch');
      const disclosure = checklist.locator('.preview-disclosure .disclosure');
      const clippedTask = checklist.locator('button[aria-label="펼친 미리보기 줄 4"]');
      expect(await clippedTask.evaluate((button: HTMLButtonElement) => {
        button.focus();
        return document.activeElement !== button && Boolean(button.closest('[inert]'));
      })).toBe(true);
      const controlBounds = async () => Promise.all([branchHandle.boundingBox(), disclosure.boundingBox()]);
      const restingControls = await controlBounds();
      await disclosure.hover();
      await branchHandle.hover();
      await branchHandle.evaluate(async (button) => {
        await Promise.all(button.getAnimations({ subtree: true }).map((animation) => animation.finished));
      });
      expect(await controlBounds()).toEqual(restingControls);
      expect(await disclosure.evaluate((button) => {
        const rect = button.getBoundingClientRect();
        const scale = rect.width / 28;
        const x = rect.left + rect.width / 2;
        const y = rect.top + rect.height / 2;
        const points = [[rect.left - 2 * scale, y], [rect.right + 2 * scale, y],
          [x, rect.top - 5 * scale], [x, rect.bottom + 5 * scale]];
        return points.map(([px, py]) => { const hit = document.elementFromPoint(px, py); return button.contains(hit) || hit?.className; });
      })).toEqual([true, true, true, true]);
      /**
       * 화면 안의 화살촉 끝이 카드·종이 더미 사각형 안이면 가린 것(선 층은 카드 아래라 겹치면 안 보인다).
       * 종이 더미는 pointer-events가 없어 elementFromPoint로는 못 잡으므로 사각형으로 판정한다.
       */
      const hiddenTips = () => canvas.evaluate(() => {
        const covers = [...document.querySelectorAll<HTMLElement>('.canvas-world > .canvas-note, .canvas-fan-sheet')]
          .filter((item) => getComputedStyle(item).opacity !== '0').map((item) => ({ rect: item.getBoundingClientRect(), label: item.closest('.canvas-fan') ? 'fan' : item.querySelector('h3')?.textContent ?? '' }));
        const hidden: string[] = [];
        let visible = 0;
        for (const path of document.querySelectorAll<SVGPathElement>('.canvas-link > path')) {
          const length = path.getTotalLength();
          const matrix = path.getScreenCTM();
          if (length < 2 || !matrix) continue;
          const end = path.getPointAtLength(length);
          const back = path.getPointAtLength(length - 1);
          // 화살촉 끝: 선 끝에서 진행 방향으로 marker 길이(viewBox 8-2=6 × 5.4/10 × 선 굵기)만큼.
          const reach = 6 * 0.54 * Number.parseFloat(path.getAttribute('stroke-width') ?? '1.5');
          const tip = { x: end.x + (end.x - back.x) * reach, y: end.y + (end.y - back.y) * reach };
          const x = tip.x * matrix.a + tip.y * matrix.c + matrix.e;
          const y = tip.x * matrix.b + tip.y * matrix.d + matrix.f;
          if (x < 0 || y < 0 || x > innerWidth || y > innerHeight) continue;
          visible += 1;
          const cover = covers.find(({ rect }) => x > rect.left + 1 && x < rect.right - 1 && y > rect.top + 1 && y < rect.bottom - 1);
          if (cover) hidden.push(`${Math.round(tip.x)},${Math.round(tip.y)} ← ${cover.label}`);
        }
        return { visible, hidden };
      });
      const atStart = await hiddenTips();
      expect(atStart.visible).toBeGreaterThan(15);
      expect(atStart.hidden).toEqual([]);
      // 펼친 미리보기: 본문을 펼친 카드도 아래 연결점이 카드 밖에 남는다.
      // 축소 화면에서는 버튼이 겹쳐 좌표 클릭이 다른 카드에 갈 수 있어 요소에 직접 누른다.
      await canvas.locator('.preview-disclosure .disclosure').evaluateAll((items) => { for (const item of items) (item as HTMLButtonElement).click(); });
      await expect.poll(() => canvas.locator('.canvas-note.is-expanded').count()).toBeGreaterThan(0);
      // 늘어나는 모션이 끝난 뒤(카드 높이가 두 번 연속 같을 때) 판정한다. 모션 중간에 보면 가림을 놓친다.
      const heightsNow = () => canvas.locator('.canvas-world > .canvas-note').evaluateAll((items) => items.map((item) => (item as HTMLElement).offsetHeight).join());
      let previous = '';
      await expect.poll(async () => { const now = await heightsNow(); const same = now === previous; previous = now; return same; }, { intervals: [150] }).toBe(true);
      expect((await hiddenTips()).hidden).toEqual([]);
      expect(await footerIsSeparated()).toBe(true);
      const firstTask = checklist.getByRole('checkbox', { name: '펼친 미리보기 줄 1', exact: true });
      const secondTask = checklist.getByRole('checkbox', { name: '펼친 미리보기 줄 2', exact: true });
      await firstTask.click();
      await expect.poll(() => firstTask.getAttribute('aria-checked')).toBe('true');
      expect(await secondTask.getAttribute('aria-checked')).toBe('false');
      expect(await checklist.locator('.canvas-card-meta').textContent()).toBe('할 일 1/8');
      expect(await checklist.getAttribute('class')).not.toContain('selected');
      await checklist.getByRole('heading').click();
      const inspectorTask = canvas.locator('.canvas-inspector').getByRole('checkbox', { name: '펼친 미리보기 줄 1', exact: true });
      await expect.poll(() => inspectorTask.getAttribute('aria-checked')).toBe('true');
      await inspectorTask.press('Space');
      await expect.poll(() => firstTask.getAttribute('aria-checked')).toBe('false');
      await secondTask.click();
      await expect.poll(() => secondTask.getAttribute('aria-checked')).toBe('true');
      const savedTask = await canvas.evaluate(async () => {
        const result = await window.memoivy?.request('snapshot');
        return result?.snapshot?.notes.find((note) => note.text.startsWith('펼치면 길어지는 메모'));
      });
      expect(savedTask?.text).toContain('- [ ] 펼친 미리보기 줄 1\n- [x] 펼친 미리보기 줄 2');
      expect(savedTask?.canvas.done).toBe(false);
      expect(app.windows().filter((page) => page.url().includes('window=note'))).toHaveLength(0);
      // 여러 부모: 가지 개요는 한 갈래 나무 + "다른 곳에서도 이어짐"으로 나머지 부모를 모두 보이고, 연결 칸은 같은 부모를 되풀이하지 않는다.
      await canvas.locator('.canvas-note', { hasText: '여러 곳에서 모이는 메모' }).click();
      await canvas.keyboard.press('Control+l');
      const linkPicker = canvas.getByRole('menu', { name: '연결할 다음 메모' });
      await linkPicker.waitFor({ state: 'visible' });
      await expect.poll(() => linkPicker.locator('[role="menuitem"]:focus').count()).toBe(1);
      await canvas.keyboard.press('Escape');
      await linkPicker.waitFor({ state: 'hidden' });
      expect(await canvas.getByRole('button', { name: '다음 메모 연결', exact: true }).evaluate((button) => button === document.activeElement)).toBe(true);
      const also = canvas.locator('.outline-also .inspector-chip');
      const dimmed = canvas.locator('.has-focus > .canvas-note:not(.in-focus)').first();
      await expect.poll(() => dimmed.evaluate((card) => getComputedStyle(card, '::after').opacity)).toBe('0.62');
      expect(await dimmed.evaluate((card) => getComputedStyle(card).opacity)).toBe('1');
      const inOutline = await canvas.locator('.inspector-outline [data-row]').count();
      await expect.poll(() => also.count()).toBeGreaterThan(0);
      expect(inOutline).toBeGreaterThan(1);
      expect(await also.count()).toBe(3); // 갈래 4개 중 나무 부모 1을 뺀 나머지
      expect(await canvas.locator('[aria-label="이전 메모"] .inspector-chip').count()).toBe(0);
      await expect.poll(() => canvas.locator('.outline-merge').textContent()).toBe('+3');
      const outlineHelp = canvas.getByRole('button', { name: '가지 개요 단축키' });
      await outlineHelp.click();
      await expect.poll(() => canvas.locator('.reveal.is-open #outline-shortcuts kbd').count()).toBe(4);
      await outlineHelp.click();
      await expect.poll(() => canvas.locator('.reveal.is-open #outline-shortcuts').count()).toBe(0);
      // 캔버스 사용법은 같은 창 안에서 열고 닫을 때 독으로 포커스를 돌려준다.
      const helpButton = canvas.getByRole('button', { name: '캔버스 사용법' });
      const windowCount = app.windows().length;
      await helpButton.click();
      const help = canvas.getByRole('dialog', { name: '단축키와 서식' });
      expect(app.windows().length).toBe(windowCount);
      // 캔버스에서 열면 캔버스 주제로 시작하고, 전체 목록은 묶음별로 접혀 있다가 펼친다.
      await expect.poll(() => help.getByRole('radio', { name: '캔버스' }).getAttribute('aria-checked')).toBe('true');
      const canvasTopic = help.getByRole('radio', { name: '캔버스' });
      const search = help.getByRole('textbox', { name: '도움말 검색' });
      const initialHelpSize = await help.boundingBox();
      const searchPosition = await search.boundingBox();
      await canvasTopic.press('ArrowLeft');
      expect(await help.getByRole('radio', { name: '메모' }).getAttribute('aria-checked')).toBe('true');
      expect(await help.boundingBox()).toEqual(initialHelpSize);
      expect(await search.boundingBox()).toEqual(searchPosition);
      await canvas.keyboard.press('ArrowRight');
      expect(await canvasTopic.getAttribute('aria-checked')).toBe('true');
      await search.focus();
      expect(await search.evaluate((input) => getComputedStyle(input).outlineStyle)).toBe('none');
      expect(await help.locator('.help-search:focus-within').count()).toBe(1);
      await search.fill('Ｃｔｒｌ ＋ Ｅｎｔｅｒ');
      const results = help.getByRole('region', { name: '검색 결과' });
      expect(await results.locator('li').count()).toBe(2);
      expect(await results.locator('.help-result-topic').allTextContents()).toEqual(['메모', '캔버스']);
      await search.fill('**글자**');
      expect(await results.locator('li').allTextContents()).toEqual(['메모굵게**글자**Ctrl+B']);
      await search.fill('Ctrl+L');
      expect(await results.locator('li').count()).toBe(1);
      expect(await results.locator('li').textContent()).toContain('다음 메모 잇기');
      await search.fill('일치하지 않는 검색어');
      await expect.poll(() => results.getByRole('status').textContent()).toBe('검색 결과가 없어요');
      await search.press('Escape');
      expect(await search.inputValue()).toBe('');
      expect(await canvasTopic.getAttribute('aria-checked')).toBe('true');
      expect(await help.boundingBox()).toEqual(initialHelpSize);
      expect(await search.boundingBox()).toEqual(searchPosition);
      expect(await help.locator('.help-basic').count()).toBe(4);
      expect(await help.locator('.help-reveal.is-open').count()).toBe(0);
      const animatedCards = () => help.locator('.help-basic').evaluateAll((cards) => cards.flatMap((card, index) =>
        card.getAnimations({ subtree: true }).some((animation) => animation instanceof CSSAnimation) ? [index] : []));
      expect(await animatedCards()).toEqual([]);
      const playbackHint = help.getByText('마우스를 올리면 재생');
      await playbackHint.waitFor({ state: 'visible' });
      expect(await help.locator('.help-demo-play:visible').count()).toBe(4);
      await help.locator('.help-basic').nth(1).hover();
      await expect.poll(animatedCards).toEqual([1]);
      await help.locator('.help-basic').nth(2).hover();
      await expect.poll(animatedCards).toEqual([2]);
      await canvasTopic.press('ArrowLeft');
      await canvas.keyboard.press('ArrowRight');
      await expect.poll(animatedCards).toEqual([]);
      await help.getByRole('heading', { name: '단축키와 서식' }).hover();
      await expect.poll(animatedCards).toEqual([]);
      await canvas.emulateMedia({ colorScheme: 'dark', reducedMotion: 'reduce' });
      await playbackHint.waitFor({ state: 'hidden' });
      expect(await help.locator('.help-demo-play:visible').count()).toBe(0);
      await help.locator('.help-basic').first().hover();
      expect(await animatedCards()).toEqual([]);
      await help.getByRole('button', { name: '프로젝트', exact: false }).first().click();
      await expect.poll(() => help.locator('.help-reveal.is-open .help-row').count()).toBe(5);
      if (process.env.MEMOIVY_SHOT) await canvas.screenshot({ path: process.env.MEMOIVY_SHOT, animations: 'disabled' });
      await canvas.keyboard.press('Escape');
      await help.waitFor({ state: 'hidden' });
      expect(await helpButton.evaluate((button) => button === document.activeElement)).toBe(true);
    } finally {
      await quit(app);
      rmSync(sampleDir, { recursive: true, force: true });
    }
  });
});
