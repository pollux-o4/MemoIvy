'use strict';

const { app, BrowserWindow, dialog, ipcMain, shell, session, nativeTheme, Menu } = require('electron');
const windowKinds = require('./window_kinds.json');
const fs = require('node:fs');
const { absoluteDirectory, linuxDataHome, snapDataDirectory, legacyNotesPath: findLegacyNotes } = require('./data_paths.cjs');
const path = require('node:path');
const { pathToFileURL, fileURLToPath } = require('node:url');
const { EngineTransport } = require('./engine_transport.cjs');
const { validateRequest, validateWindowTarget, isId } = require('./ipc_validation.cjs');
const { windowChrome } = require('./window_chrome.cjs');
const { homeToReplace, shouldShowLibrary } = require('./window_policy.cjs');
const { isSafeExternal, openExternal } = require('./external_links.cjs');

function overrideDataPath() {
  const override = process.env.MEMOIVY_DATA_DIR;
  if (override === undefined) return undefined;
  const directory = absoluteDirectory(override);
  if (!directory) throw new Error('MEMOIVY_DATA_DIR must be an absolute non-root path');
  return directory;
}

app.setName('MemoIvy');
if (process.platform === 'win32') app.setAppUserModelId('io.github.pollux_o4.MemoIvy');
if (process.platform === 'linux') {
  app.setDesktopName('io.github.pollux_o4.MemoIvy.desktop');
  app.commandLine.appendSwitch('class', 'io.github.pollux_o4.MemoIvy');
}
const dataOverride = overrideDataPath();
const snapDirectory = !dataOverride && process.platform === 'linux' ? snapDataDirectory(process.env) : undefined;
const profileDirectory = dataOverride ?? snapDirectory;
if (profileDirectory) app.setPath('userData', path.join(profileDirectory, 'electron-profile'));
const gotLock = app.requestSingleInstanceLock();
if (!gotLock) app.quit();

// 개발 렌더러 주소. 이미 쓰는 dev 서버와 겹치지 않게 검수용 포트로 바꿀 수 있다.
const DEV_URL = process.env.MEMOIVY_RENDERER_URL || 'http://127.0.0.1:5173';
const CLOSE_TIMEOUT_MS = 12000;
const windows = new Set();
const windowByKey = new Map();
const pendingFlushAcks = new Map();
const closing = new WeakMap();
const permittedClose = new WeakSet();
const approvedContents = new Set();
let engine;
let engineFailure;
let engineConfig;
let engineLaunchPromise;
let currentSnapshot;
let isQuitting = false;
let quitApproved = false;

function dataDirectory() {
  if (profileDirectory) return profileDirectory;
  const linuxHome = process.platform === 'linux' && linuxDataHome(process.env);
  return linuxHome ? path.join(linuxHome, 'memoivy') : path.join(app.getPath('appData'), 'MemoIvy');
}

function legacyNotesPath(dataDir) {
  // An explicit test or development directory must not discover real user data.
  return findLegacyNotes(dataDir, dataOverride ? {} : process.env, process.platform, snapDirectory);
}

function engineExecutable() {
  const name = process.platform === 'win32' ? 'memoivy-engine.exe' : 'memoivy-engine';
  return app.isPackaged
    ? path.join(process.resourcesPath, 'engine', name)
    : path.join(__dirname, '..', 'engine', 'target', 'debug', name);
}

function rendererTarget(kind, id) {
  const query = new URLSearchParams({ window: kind });
  if (id) query.set('id', id);
  if (app.isPackaged) return `${pathToFileURL(path.join(__dirname, '..', 'frontend', 'dist', 'index.html')).href}?${query}`;
  return `${DEV_URL}/?${query}`;
}

function isApprovedFrame(frameUrl) {
  try {
    const url = new URL(frameUrl);
    if (!app.isPackaged) return url.origin === DEV_URL;
    return url.protocol === 'file:' && path.resolve(fileURLToPath(url)) === path.resolve(path.join(__dirname, '..', 'frontend', 'dist', 'index.html'));
  } catch { return false; }
}

function broadcast(event) {
  for (const win of windows) if (!win.isDestroyed() && !win.webContents.isDestroyed()) win.webContents.send('memoivy:event', event);
}

// Engine snapshots alone determine shared state and deleted-note window lifetime.
function applySnapshot(snapshot) {
  if (!snapshot || (currentSnapshot && snapshot.revision <= currentSnapshot.revision)) return;
  currentSnapshot = snapshot;
  broadcast({ version: 1, event: 'snapshot', snapshot });
  const liveIds = new Set(snapshot.notes.map((note) => note.id));
  for (const [key, win] of windowByKey) {
    if (!key.startsWith('note:') || liveIds.has(key.slice(5)) || win.isDestroyed()) continue;
    // A committed deletion must not flush the removed note's obsolete draft.
    permittedClose.add(win);
    win.close();
  }
}

function openExternalWithoutWaiting(url) {
  void openExternal(url, (target) => shell.openExternal(target)).catch((error) => console.error('External link failed:', error.message));
}

function setEngineFailure(error) {
  engineFailure = error;
  broadcast({ version: 1, event: 'error', error: { code: error.code || 'ENGINE_UNAVAILABLE', message: error.message || 'MemoIvy engine is unavailable', snapshot: currentSnapshot } });
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

function requireEngine() {
  if (engineFailure) throw engineFailure;
  if (!engine) {
    const error = new Error('MemoIvy engine has not started');
    error.code = 'ENGINE_UNAVAILABLE';
    throw error;
  }
  return engine;
}

async function flushRenderer(win) {
  if (win.isDestroyed() || win.webContents.isDestroyed()) throw new Error('Window was destroyed during flush');
  const token = require('node:crypto').randomUUID();
  const ack = new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pendingFlushAcks.delete(token);
      reject(new Error('Renderer save timed out'));
    }, CLOSE_TIMEOUT_MS);
    pendingFlushAcks.set(token, { wc: win.webContents, resolve: (value) => { clearTimeout(timer); resolve(value); }, reject });
  });
  win.webContents.send('memoivy:flush', token);
  const response = await ack;
  if (!response.ok) throw new Error(response.message || 'Renderer could not save its draft');
}

async function flushWindow(win) {
  await flushRenderer(win);
  if (engineFailure && engineConfig) await launchEngine();
  await requireEngine().request('flush', {});
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
  if (library && !library.isDestroyed()) { library.show(); library.focus(); return; }
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
        permittedClose.add(win); win.close();
      }
    }
  } catch (error) {
    broadcast({ version: 1, event: 'error', error: { code: 'CLOSE_FLUSH_FAILED', message: error.message, snapshot: currentSnapshot } });
  } finally {
    closing.delete(win);
  }
}

function installIpc() {
  ipcMain.handle('memoivy:request', async (event, op, params) => {
    assertSender(event);
    validateRequest(op, params);
    try {
      if (engineFailure && engineConfig && ['snapshot', 'readAsset', 'flush'].includes(op)) await launchEngine();
      const result = await requireEngine().request(op, params);
      applySnapshot(result.snapshot);
      return { __memoivyResult: result };
    } catch (error) {
      if (error.code === 'ENGINE_UNAVAILABLE' || /Engine (?:exited|sent malformed|request timed out)/.test(error.message)) setEngineFailure(error);
      return { __memoivyError: { code: error.code || 'ENGINE_ERROR', message: error.message, snapshot: error.snapshot } };
    }
  });
  ipcMain.handle('memoivy:flush-ack', (event, token, response) => {
    assertSender(event);
    const pending = pendingFlushAcks.get(token);
    if (!pending || pending.wc !== event.sender) return false;
    pendingFlushAcks.delete(token);
    if (!response || typeof response.ok !== 'boolean' || (response.message !== undefined && (typeof response.message !== 'string' || response.message.length > 1000))) {
      pending.reject(new Error('Invalid renderer flush acknowledgement'));
      return false;
    }
    pending.resolve(response);
    return true;
  });
  ipcMain.handle('memoivy:open-note', async (event, id) => {
    assertSender(event);
    if (!isId(id)) throw new TypeError('Invalid note id');
    const result = await requireEngine().request('snapshot', {});
    applySnapshot(result.snapshot);
    if (!currentSnapshot?.notes?.some((note) => note.id === id)) throw new Error('Note no longer exists');
    createWindow('note', id);
  });
  ipcMain.handle('memoivy:open-window', (event, kind, id) => {
    assertSender(event);
    const target = validateWindowTarget(kind, id);
    createWindow(target.kind, target.id);
  });
  ipcMain.handle('memoivy:open-external', async (event, url) => {
    assertSender(event);
    await openExternal(url, (target) => shell.openExternal(target));
  });
  ipcMain.handle('memoivy:import-legacy', async (event) => {
    const win = assertSender(event);
    const choice = await dialog.showOpenDialog(win, { title: 'Import MemoIvy notes', properties: ['openFile'], filters: [{ name: 'MemoIvy notes', extensions: ['json'] }] });
    if (choice.canceled || choice.filePaths.length !== 1) return { __memoivyResult: null };
    const selected = choice.filePaths[0];
    const info = fs.lstatSync(selected);
    if (!info.isFile() || info.isSymbolicLink()) throw new Error('Selected file must be a regular JSON file');
    try {
      // shortcut: 자동 탐색한 JSON의 오류가 수동 가져오기 재시작도 막음, 가져오기 오류 복구를 개선할 때 자동 탐색 건너뛰기 검토.
      if (engineFailure && engineConfig) await launchEngine();
      const result = await requireEngine().request('importLegacy', { path: selected });
      applySnapshot(result.snapshot);
      return { __memoivyResult: result };
    } catch (error) {
      return { __memoivyError: { code: error.code || 'ENGINE_ERROR', message: error.message, snapshot: error.snapshot } };
    }
  });
  ipcMain.handle('memoivy:window-action', (event, action) => {
    const win = assertSender(event);
    if (!['minimize', 'maximize'].includes(action)) throw new TypeError('Invalid window action');
    if (action === 'minimize') win.minimize();
    else if (win.isMaximized()) win.unmaximize();
    else win.maximize();
  });
  ipcMain.handle('memoivy:window-state', (event) => windowState(assertSender(event)));
  ipcMain.handle('memoivy:close-window', (event) => requestWindowClose(assertSender(event)));
}

/** 렌더러 창 장식이 따르는 상태(DesktopBridge.onWindowState). */
function windowState(win) {
  return { maximized: win.isMaximized() || win.isFullScreen(), rounded: windowChrome(process.platform).transparent };
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
    ...windowChrome(process.platform, nativeTheme.shouldUseDarkColors),
    show: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
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
      openExternalWithoutWaiting(target);
    }
  });
  win.webContents.setWindowOpenHandler(({ url }) => {
    openExternalWithoutWaiting(url);
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
  const load = app.isPackaged ? win.loadURL(target) : win.loadURL(target).catch((error) => {
    console.error('Could not load the development renderer:', error.message);
    broadcast({ version: 1, event: 'error', error: { code: 'RENDERER_UNAVAILABLE', message: 'Start the frontend development server on 127.0.0.1:5173.' } });
  });
  void load;
  return win;
}

function configureCsp() {
  const scriptSource = app.isPackaged ? "'self'" : `'self' 'unsafe-inline' ${DEV_URL}`;
  const connectSource = app.isPackaged ? "'self'" : `'self' ${DEV_URL} ${DEV_URL.replace(/^http/, 'ws')}`;
  const csp = `default-src 'self'; script-src ${scriptSource}; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src ${connectSource}; object-src 'none'; frame-src 'none'; base-uri 'self'; form-action 'none'`;
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => callback({ responseHeaders: { ...details.responseHeaders, 'Content-Security-Policy': [csp] } }));
}

function launchEngine() {
  if (engineLaunchPromise) return engineLaunchPromise;
  engineLaunchPromise = spawnEngineAndReadSnapshot().finally(() => { engineLaunchPromise = null; });
  return engineLaunchPromise;
}

async function spawnEngineAndReadSnapshot() {
  const dir = dataDirectory();
  fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
  try { fs.chmodSync(dir, 0o700); } catch { /* ACL based platforms do not expose POSIX modes. */ }
  const executable = engineExecutable();
  const args = ['--data-dir', dir];
  const legacy = legacyNotesPath(dir);
  if (legacy) args.push('--legacy', legacy);
  engineConfig = { executable, args };
  const transport = new EngineTransport({ executable, args });
  engine = transport;
  transport.on('event', (event) => {
    if (engine !== transport) return;
    if (event.snapshot) applySnapshot(event.snapshot);
    else broadcast(event);
  });
  transport.on('failure', (error) => { if (engine === transport) setEngineFailure(error); });
  transport.on('stderr', (line) => { if (engine === transport) console.error(`[memoivy-engine] ${line.trimEnd()}`); });
  transport.start();
  const initial = await transport.request('snapshot', {});
  if (engine !== transport) throw new Error('A newer engine process replaced this startup');
  engineFailure = null;
  applySnapshot(initial.snapshot);
}

async function doQuit() {
  if (quitApproved) return;
  if (isQuitting) return;
  isQuitting = true;
  try {
    await Promise.all([...windows].map((win) => flushRenderer(win)));
    if (engineFailure && engineConfig) await launchEngine();
    await requireEngine().request('flush', {});
    await closeEngineProcess();
    quitApproved = true;
    app.quit();
  } catch (error) {
    isQuitting = false;
    broadcast({ version: 1, event: 'error', error: { code: 'QUIT_FLUSH_FAILED', message: error.message, snapshot: currentSnapshot } });
    for (const win of windows) if (!win.isDestroyed()) { win.show(); win.focus(); }
  }
}

async function closeEngineProcess() {
  const child = engine?.child;
  if (!child || engine.closed || engine.deadError) return;
  const exited = new Promise((resolve) => child.once('exit', resolve));
  engine.closed = true;
  engine.closing = true;
  child.stdin.end();
  let timer;
  const didExit = await Promise.race([exited.then(() => true), new Promise((resolve) => { timer = setTimeout(() => resolve(false), 2500); })]);
  clearTimeout(timer);
  if (!didExit) {
    child.kill();
    await Promise.race([exited, new Promise((resolve) => setTimeout(resolve, 1000))]);
  }
}

if (gotLock) {
  app.on('second-instance', () => {
    const win = windowByKey.get('library') || windows.values().next().value;
    if (win && !win.isDestroyed()) { if (win.isMinimized()) win.restore(); win.focus(); }
    else createWindow('library');
  });
  app.whenReady().then(async () => {
    if (process.platform !== 'darwin') Menu.setApplicationMenu(null);
    configureCsp();
    installIpc();
    createWindow('library');
    void launchEngine().catch(setEngineFailure);
    app.on('activate', () => { if (windows.size === 0) createWindow('library'); });
  }).catch((error) => {
    console.error('MemoIvy startup failed:', error);
    app.quit();
  });
  app.on('before-quit', (event) => {
    if (quitApproved) return;
    event.preventDefault();
    void doQuit();
  });
  app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
  const signalQuit = () => app.quit();
  process.on('SIGINT', signalQuit);
  process.on('SIGTERM', signalQuit);
}

module.exports = { dataDirectory, legacyNotesPath, isSafeExternal };
