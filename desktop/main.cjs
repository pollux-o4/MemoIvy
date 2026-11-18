'use strict';

const { app, BrowserWindow, dialog, ipcMain, shell, session } = require('electron');
const windowKinds = require('./window_kinds.json');
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL, fileURLToPath } = require('node:url');
const { EngineTransport } = require('./engine_transport.cjs');
const { validateRequest, validateWindowTarget, isId } = require('./ipc_validation.cjs');

function overrideDataPath() {
  const override = process.env.MEMOIVY_DATA_DIR;
  if (override === undefined) return undefined;
  if (!path.isAbsolute(override) || path.resolve(override) === path.parse(override).root) throw new Error('MEMOIVY_DATA_DIR must be an absolute non-root path');
  return path.resolve(override);
}

app.setName('MemoIvy');
if (process.platform === 'win32') app.setAppUserModelId('io.github.pollux_o4.MemoIvy');
if (process.platform === 'linux') {
  app.setDesktopName('io.github.pollux_o4.MemoIvy.desktop');
  app.commandLine.appendSwitch('class', 'io.github.pollux_o4.MemoIvy');
}
const dataOverride = overrideDataPath();
if (dataOverride) app.setPath('userData', path.join(dataOverride, 'electron-profile'));
const gotLock = app.requestSingleInstanceLock();
if (!gotLock) app.quit();

// 개발 렌더러 주소. 이미 쓰는 dev 서버와 겹치지 않게 검수용 포트로 바꿀 수 있다.
const DEV_URL = process.env.MEMOIVY_RENDERER_URL || 'http://127.0.0.1:5173';
const CLOSE_TIMEOUT_MS = 12000;
const windows = new Set();
const windowByKey = new Map();
const pendingFlushAcks = new Map();
const closing = new WeakSet();
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
  if (dataOverride) return dataOverride;
  if (process.platform === 'linux') {
    const xdg = process.env.XDG_DATA_HOME;
    if (xdg && path.isAbsolute(xdg)) return path.join(xdg, 'memoivy');
    const home = process.env.HOME;
    if (home && path.isAbsolute(home)) return path.join(home, '.local', 'share', 'memoivy');
  }
  if (process.platform === 'darwin') return path.join(app.getPath('appData'), 'MemoIvy');
  return path.join(app.getPath('appData'), 'MemoIvy');
}

function legacyNotesPath(dataDir) {
  const candidates = [path.join(dataDir, 'notes.json')];
  if (process.platform === 'linux') {
    const xdg = process.env.XDG_DATA_HOME && path.isAbsolute(process.env.XDG_DATA_HOME)
      ? process.env.XDG_DATA_HOME
      : process.env.HOME && path.isAbsolute(process.env.HOME) ? path.join(process.env.HOME, '.local', 'share') : null;
    if (xdg) candidates.push(path.join(xdg, 'linux-sticky-memo', 'notes.json'));
    if (process.env.SNAP_USER_DATA && path.isAbsolute(process.env.SNAP_USER_DATA)) {
      candidates.push(path.join(process.env.SNAP_USER_DATA, 'notes.json'));
      candidates.push(path.join(process.env.SNAP_USER_DATA, 'memoivy', 'notes.json'));
    }
  }
  return candidates.find((candidate) => {
    try { return fs.lstatSync(candidate).isFile(); } catch { return false; }
  });
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

function isSafeExternal(url) {
  try {
    const parsed = new URL(url);
    return ['https:', 'http:', 'mailto:'].includes(parsed.protocol) && !parsed.username && !parsed.password;
  } catch { return false; }
}

function broadcast(event) {
  for (const win of windows) if (!win.isDestroyed() && !win.webContents.isDestroyed()) win.webContents.send('memoivy:event', event);
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

async function requestWindowClose(win, { hide = false } = {}) {
  if (!win || win.isDestroyed() || closing.has(win)) return;
  closing.add(win);
  try {
    await flushWindow(win);
    if (!win.isDestroyed()) {
      if (hide) win.hide();
      else { permittedClose.add(win); win.close(); }
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
      if (result.snapshot) currentSnapshot = result.snapshot;
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
    currentSnapshot = result.snapshot;
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
    if (typeof url !== 'string' || url.length > 4096 || !isSafeExternal(url)) throw new TypeError('Only HTTP and HTTPS links are allowed');
    await shell.openExternal(url);
  });
  ipcMain.handle('memoivy:import-legacy', async (event) => {
    const win = assertSender(event);
    const choice = await dialog.showOpenDialog(win, { title: 'Import MemoIvy notes', properties: ['openFile'], filters: [{ name: 'MemoIvy notes', extensions: ['json'] }] });
    if (choice.canceled || choice.filePaths.length !== 1) return { __memoivyResult: null };
    const selected = choice.filePaths[0];
    const info = fs.lstatSync(selected);
    if (!info.isFile() || info.isSymbolicLink()) throw new Error('Selected file must be a regular JSON file');
    try {
      if (engineFailure && engineConfig) await launchEngine();
      const result = await requireEngine().request('importLegacy', { path: selected });
      if (result.snapshot) currentSnapshot = result.snapshot;
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
  ipcMain.handle('memoivy:close-window', (event) => requestWindowClose(assertSender(event)));
}

function createWindow(kind = 'library', id) {
  const key = kind === 'note' ? `note:${id}` : kind;
  const existing = windowByKey.get(key);
  if (existing && !existing.isDestroyed()) {
    if (existing.isMinimized()) existing.restore();
    existing.focus();
    return existing;
  }
  const win = new BrowserWindow({
    ...windowKinds[kind],
    frame: false,
    transparent: true, // 둥근 창 모서리(GTK CSD와 동일)를 위해 창 바탕을 투명하게 둔다
    backgroundColor: '#00000000',
    show: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: true,
      webSecurity: true,
      allowRunningInsecureContent: false,
    },
  });
  // closed 시점에는 win.webContents 접근이 'Object has been destroyed'로 실패한다.
  const contents = win.webContents;
  windows.add(win);
  approvedContents.add(contents);
  windowByKey.set(key, win);
  win.once('ready-to-show', () => win.show());
  win.webContents.on('will-navigate', (event, target) => {
    if (!isApprovedFrame(target)) {
      event.preventDefault();
      if (isSafeExternal(target)) void shell.openExternal(target).catch((error) => console.error('External link failed:', error.message));
    }
  });
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (isSafeExternal(url)) void shell.openExternal(url).catch((error) => console.error('External link failed:', error.message));
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
    if (kind === 'note' && !isQuitting && !quitApproved && ![...windowByKey.keys()].some((windowKey) => windowKey.startsWith('note:'))) {
      const library = windowByKey.get('library');
      if (library && !library.isDestroyed()) library.show();
    }
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
    if (event.snapshot) currentSnapshot = event.snapshot;
    broadcast(event);
  });
  transport.on('failure', (error) => { if (engine === transport) setEngineFailure(error); });
  transport.on('stderr', (line) => { if (engine === transport) console.error(`[memoivy-engine] ${line.trimEnd()}`); });
  transport.start();
  const initial = await transport.request('snapshot', {});
  if (engine !== transport) throw new Error('A newer engine process replaced this startup');
  engineFailure = null;
  currentSnapshot = initial.snapshot;
  broadcast({ version: 1, event: 'snapshot', snapshot: currentSnapshot });
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
