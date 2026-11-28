'use strict';

const { app, BrowserWindow, dialog, ipcMain, shell, session, nativeTheme, Menu } = require('electron');
const windowKinds = require('./window_kinds.json');
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL, fileURLToPath } = require('node:url');
const { absoluteDirectory, linuxDataHome, snapDataDirectory, legacyNotesPath: findLegacyNotes } = require('./data_paths.cjs');
const { windowChrome } = require('./window_chrome.cjs');
const { isSafeExternal, openExternal } = require('./external_links.cjs');
const { createEngineRuntime } = require('./engine_runtime.cjs');
const { createWindowManager } = require('./window_manager.cjs');
const { installIpc } = require('./ipc_router.cjs');

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

function openExternalWithoutWaiting(url) {
  void openExternal(url, (target) => shell.openExternal(target)).catch((error) => console.error('External link failed:', error.message));
}

function configureCsp() {
  const scriptSource = app.isPackaged ? "'self'" : `'self' 'unsafe-inline' ${DEV_URL}`;
  const connectSource = app.isPackaged ? "'self'" : `'self' ${DEV_URL} ${DEV_URL.replace(/^http/, 'ws')}`;
  const csp = `default-src 'self'; script-src ${scriptSource}; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src ${connectSource}; object-src 'none'; frame-src 'none'; base-uri 'self'; form-action 'none'`;
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => callback({ responseHeaders: { ...details.responseHeaders, 'Content-Security-Policy': [csp] } }));
}

let windowManager;
const engineRuntime = createEngineRuntime({
  dataDirectory,
  legacyNotesPath,
  engineExecutable,
  fs,
  onSnapshot: (snapshot) => {
    windowManager.broadcast({ version: 1, event: 'snapshot', snapshot });
    windowManager.pruneDeletedNotes(snapshot.notes);
  },
  onEvent: (event) => windowManager.broadcast(event),
  onError: (error) => windowManager.broadcast({ version: 1, event: 'error', error }),
});

windowManager = createWindowManager({
  BrowserWindow,
  nativeTheme,
  windowKinds,
  windowChrome,
  preloadPath: path.join(__dirname, 'preload.cjs'),
  isPackaged: app.isPackaged,
  rendererTarget,
  isApprovedFrame,
  flushEngine: () => engineRuntime.flush(),
  openExternal: openExternalWithoutWaiting,
  getSnapshot: () => engineRuntime.snapshot(),
});

async function doQuit() {
  if (!windowManager.beginQuit()) return;
  try {
    await windowManager.flushAll();
    await engineRuntime.flush();
    await engineRuntime.close();
    windowManager.approveQuit();
    app.quit();
  } catch (error) {
    windowManager.cancelQuit();
    windowManager.broadcastError('QUIT_FLUSH_FAILED', error.message);
    windowManager.revealAll();
  }
}

if (gotLock) {
  app.on('second-instance', () => windowManager.focusPrimary());
  app.whenReady().then(async () => {
    if (process.platform !== 'darwin') Menu.setApplicationMenu(null);
    configureCsp();
    installIpc({
      ipcMain,
      dialog,
      fs,
      engineRuntime,
      windowManager,
      openExternal: (url) => openExternal(url, (target) => shell.openExternal(target)),
    });
    windowManager.createWindow('library');
    void engineRuntime.launch().catch(engineRuntime.setFailure);
    app.on('activate', () => { if (!windowManager.hasWindows()) windowManager.createWindow('library'); });
  }).catch((error) => {
    console.error('MemoIvy startup failed:', error);
    app.quit();
  });
  app.on('before-quit', (event) => {
    if (windowManager.isQuitApproved()) return;
    event.preventDefault();
    void doQuit();
  });
  app.on('window-all-closed', () => { if (process.platform !== 'darwin') app.quit(); });
  const signalQuit = () => app.quit();
  process.on('SIGINT', signalQuit);
  process.on('SIGTERM', signalQuit);
}

module.exports = { dataDirectory, legacyNotesPath, isSafeExternal };
