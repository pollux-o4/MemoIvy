'use strict';

const { randomUUID } = require('node:crypto');
const { homeToReplace, shouldShowLibrary } = require('./window_policy.cjs');

/**
 * 다중 BrowserWindow 생명주기·홈 자리 교체·초안 저장(flush) 조율을 맡는 Mediator.
 */
function createWindowManager({
  BrowserWindow,
  nativeTheme,
  windowKinds,
  windowChrome,
  platform = process.platform,
  preloadPath,
  isPackaged,
  rendererTarget,
  isApprovedFrame,
  flushEngine,
  openExternal,
  getSnapshot = () => undefined,
  closeTimeoutMs = 12000,
}) {
  const windows = new Set();
  const windowByKey = new Map();
  const pendingFlushAcks = new Map();
  const closing = new WeakMap();
  const permittedClose = new WeakSet();
  const approvedContents = new Set();
  let isQuitting = false;
  let quitApproved = false;

  function broadcast(event) {
    for (const win of windows) {
      if (!win.isDestroyed() && !win.webContents.isDestroyed()) win.webContents.send('memoivy:event', event);
    }
  }

  function broadcastError(code, message) {
    broadcast({ version: 1, event: 'error', error: { code, message, snapshot: getSnapshot() } });
  }

  function assertSender(event) {
    const wc = event.sender;
    if (!approvedContents.has(wc) || wc.isDestroyed() || !isApprovedFrame(event.senderFrame?.url || '')) {
      const error = new Error('Untrusted IPC sender');
      error.code = 'INVALID_SENDER';
      throw error;
    }
    return BrowserWindow.fromWebContents(wc);
  }

  async function flushRenderer(win) {
    if (win.isDestroyed() || win.webContents.isDestroyed()) throw new Error('Window was destroyed during flush');
    const token = randomUUID();
    const ack = new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pendingFlushAcks.delete(token);
        reject(new Error('Renderer save timed out'));
      }, closeTimeoutMs);
      pendingFlushAcks.set(token, {
        wc: win.webContents,
        resolve: (value) => { clearTimeout(timer); resolve(value); },
        reject,
      });
    });
    win.webContents.send('memoivy:flush', token);
    const response = await ack;
    if (!response.ok) throw new Error(response.message || 'Renderer could not save its draft');
  }

  function ackFlush(sender, token, response) {
    const pending = pendingFlushAcks.get(token);
    if (!pending || pending.wc !== sender) return false;
    pendingFlushAcks.delete(token);
    if (!response || typeof response.ok !== 'boolean' || (response.message !== undefined && (typeof response.message !== 'string' || response.message.length > 1000))) {
      pending.reject(new Error('Invalid renderer flush acknowledgement'));
      return false;
    }
    pending.resolve(response);
    return true;
  }

  async function flushWindow(win) {
    await flushRenderer(win);
    await flushEngine();
  }

  // 닫히는 창의 키와 나머지 보이는 창의 키를 한 번에 수집한다.
  function closeContext(win) {
    let closingKey;
    const visibleKeys = [];
    for (const [key, other] of windowByKey) {
      if (other === win) closingKey = key;
      else if (!other.isDestroyed() && other.isVisible()) visibleKeys.push(key);
    }
    return { closingKey, visibleKeys };
  }

  function replaceOtherHome(key) {
    const otherKey = homeToReplace(key, windowByKey.keys());
    if (otherKey !== null) void requestWindowClose(windowByKey.get(otherKey), { replaced: true });
  }

  function showLibrary() {
    const library = windowByKey.get('library');
    if (library && !library.isDestroyed()) {
      library.show();
      library.focus();
      return;
    }
    createWindow('library');
  }

  async function requestWindowClose(win, { hide = false, replaced = false } = {}) {
    if (!win || win.isDestroyed()) return;
    const pending = closing.get(win);
    if (pending) {
      if (replaced) pending.replaced = true;
      return;
    }
    const intent = { replaced };
    closing.set(win, intent);
    try {
      await flushWindow(win);
      if (!win.isDestroyed()) {
        if (hide && !intent.replaced) win.hide();
        else {
          const { closingKey, visibleKeys } = closeContext(win);
          if (!intent.replaced && closingKey !== undefined && shouldShowLibrary(closingKey, visibleKeys)) showLibrary();
          permittedClose.add(win);
          win.close();
        }
      }
    } catch (error) {
      broadcastError('CLOSE_FLUSH_FAILED', error.message);
    } finally {
      closing.delete(win);
    }
  }

  /** 렌더러 창 장식이 따르는 상태(DesktopBridge.onWindowState). */
  function windowState(win) {
    return { maximized: win.isMaximized() || win.isFullScreen(), rounded: windowChrome(platform).transparent };
  }

  function createWindow(kind = 'library', id) {
    const key = kind === 'note' ? `note:${id}` : kind;
    const existing = windowByKey.get(key);
    if (existing && !existing.isDestroyed()) {
      if (existing.isMinimized()) existing.restore();
      existing.show();
      existing.focus();
      replaceOtherHome(key);
      return existing;
    }
    const win = new BrowserWindow({
      ...windowKinds[kind],
      frame: false,
      ...windowChrome(platform, nativeTheme.shouldUseDarkColors),
      show: false,
      webPreferences: {
        preload: preloadPath,
        nodeIntegration: false,
        contextIsolation: true,
        sandbox: true,
        webSecurity: true,
        allowRunningInsecureContent: false,
        spellcheck: false,
      },
    });
    // closed 시점에는 win.webContents 접근이 'Object has been destroyed'로 실패한다.
    const contents = win.webContents;
    windows.add(win);
    approvedContents.add(contents);
    windowByKey.set(key, win);
    win.once('ready-to-show', () => {
      win.show();
      replaceOtherHome(key);
    });
    const sendWindowState = () => { if (!contents.isDestroyed()) contents.send('memoivy:window-state', windowState(win)); };
    for (const name of ['maximize', 'unmaximize', 'enter-full-screen', 'leave-full-screen']) win.on(name, sendWindowState);
    win.webContents.on('will-navigate', (event, target) => {
      if (!isApprovedFrame(target)) {
        event.preventDefault();
        openExternal(target);
      }
    });
    win.webContents.setWindowOpenHandler(({ url }) => {
      openExternal(url);
      return { action: 'deny' };
    });
    win.on('close', (event) => {
      if (quitApproved) return;
      if (isQuitting) { event.preventDefault(); return; }
      if (permittedClose.has(win)) { permittedClose.delete(win); return; }
      event.preventDefault();
      const hasOpenNote = [...windowByKey.keys()].some((windowKey) => windowKey.startsWith('note:'));
      void requestWindowClose(win, { hide: kind === 'library' && hasOpenNote });
    });
    win.on('closed', () => {
      windows.delete(win);
      approvedContents.delete(contents);
      if (windowByKey.get(key) === win) windowByKey.delete(key);
    });
    const target = rendererTarget(kind, id);
    const load = isPackaged ? win.loadURL(target) : win.loadURL(target).catch((error) => {
      console.error('Could not load the development renderer:', error.message);
      broadcast({ version: 1, event: 'error', error: { code: 'RENDERER_UNAVAILABLE', message: 'Start the frontend development server on 127.0.0.1:5173.' } });
    });
    void load;
    return win;
  }

  function pruneDeletedNotes(notes) {
    const liveIds = new Set(notes.map((note) => note.id));
    for (const [key, win] of windowByKey) {
      if (!key.startsWith('note:') || liveIds.has(key.slice(5)) || win.isDestroyed()) continue;
      // A committed deletion must not flush the removed note's obsolete draft.
      permittedClose.add(win);
      win.close();
    }
  }

  function focusPrimary() {
    const win = windowByKey.get('library') || windows.values().next().value;
    if (win && !win.isDestroyed()) {
      if (win.isMinimized()) win.restore();
      win.focus();
      return;
    }
    createWindow('library');
  }

  function revealAll() {
    for (const win of windows) {
      if (!win.isDestroyed()) { win.show(); win.focus(); }
    }
  }

  return {
    createWindow,
    requestWindowClose,
    pruneDeletedNotes,
    broadcast,
    broadcastError,
    assertSender,
    ackFlush,
    flushAll: () => Promise.all([...windows].map((win) => flushRenderer(win))),
    windowState,
    focusPrimary,
    revealAll,
    hasWindows: () => windows.size > 0,
    beginQuit: () => {
      if (quitApproved || isQuitting) return false;
      isQuitting = true;
      return true;
    },
    cancelQuit: () => { isQuitting = false; },
    approveQuit: () => { quitApproved = true; },
    isQuitApproved: () => quitApproved,
  };
}

module.exports = { createWindowManager };
