'use strict';

const defaultFs = require('node:fs');
const { validateRequest, validateWindowTarget, isId } = require('./ipc_validation.cjs');

/**
 * 렌더러 IPC 경계를 검증하고 엔진 실행·창 관리자에 위임한다.
 */
function installIpc({
  ipcMain,
  dialog,
  fs = defaultFs,
  engineRuntime,
  windowManager,
  openExternal,
}) {
  const RECOVERY_OPS = new Set(['snapshot', 'readAsset', 'flush']);

  ipcMain.handle('memoivy:request', async (event, op, params) => {
    windowManager.assertSender(event);
    validateRequest(op, params);
    try {
      const result = await engineRuntime.request(op, params, {
        recover: RECOVERY_OPS.has(op),
        markFailureOnTransportError: true,
      });
      return { __memoivyResult: result };
    } catch (error) {
      return { __memoivyError: { code: error.code || 'ENGINE_ERROR', message: error.message, snapshot: error.snapshot } };
    }
  });

  ipcMain.handle('memoivy:flush-ack', (event, token, response) => {
    windowManager.assertSender(event);
    return windowManager.ackFlush(event.sender, token, response);
  });

  ipcMain.handle('memoivy:open-note', async (event, id) => {
    windowManager.assertSender(event);
    if (!isId(id)) throw new TypeError('Invalid note id');
    await engineRuntime.request('snapshot', {});
    if (!engineRuntime.snapshot()?.notes?.some((note) => note.id === id)) throw new Error('Note no longer exists');
    windowManager.createWindow('note', id);
  });

  ipcMain.handle('memoivy:open-window', (event, kind, id) => {
    windowManager.assertSender(event);
    const target = validateWindowTarget(kind, id);
    windowManager.createWindow(target.kind, target.id);
  });

  ipcMain.handle('memoivy:open-external', async (event, url) => {
    windowManager.assertSender(event);
    await openExternal(url);
  });

  ipcMain.handle('memoivy:import-legacy', async (event) => {
    const win = windowManager.assertSender(event);
    const choice = await dialog.showOpenDialog(win, {
      title: 'Import MemoIvy notes',
      properties: ['openFile'],
      filters: [{ name: 'MemoIvy notes', extensions: ['json'] }],
    });
    if (choice.canceled || choice.filePaths.length !== 1) return { __memoivyResult: null };
    const selected = choice.filePaths[0];
    const info = fs.lstatSync(selected);
    if (!info.isFile() || info.isSymbolicLink()) throw new Error('Selected file must be a regular JSON file');
    try {
      // shortcut: 자동 탐색한 JSON의 오류가 수동 가져오기 재시작도 막음, 가져오기 오류 복구를 개선할 때 자동 탐색 건너뛰기 검토.
      const result = await engineRuntime.request('importLegacy', { path: selected }, { recover: true });
      return { __memoivyResult: result };
    } catch (error) {
      return { __memoivyError: { code: error.code || 'ENGINE_ERROR', message: error.message, snapshot: error.snapshot } };
    }
  });

  ipcMain.handle('memoivy:window-action', (event, action) => {
    const win = windowManager.assertSender(event);
    if (!['minimize', 'maximize'].includes(action)) throw new TypeError('Invalid window action');
    if (action === 'minimize') win.minimize();
    else if (win.isMaximized()) win.unmaximize();
    else win.maximize();
  });

  ipcMain.handle('memoivy:window-state', (event) => windowManager.windowState(windowManager.assertSender(event)));
  ipcMain.handle('memoivy:close-window', (event) => windowManager.requestWindowClose(windowManager.assertSender(event)));
}

module.exports = { installIpc };
