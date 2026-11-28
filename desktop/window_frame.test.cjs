'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { EventEmitter } = require('node:events');
const { createWindowManager } = require('./window_manager.cjs');
const { windowChrome } = require('./window_chrome.cjs');
const windowKinds = require('./window_kinds.json');

function createHarness() {
  class FakeWindow extends EventEmitter {
    constructor(options) {
      super();
      this.options = options;
      this.maximized = false;
      this.fullScreen = false;
      this.sent = [];
      this.webContents = new EventEmitter();
      this.webContents.setWindowOpenHandler = () => {};
      this.webContents.isDestroyed = () => false;
      this.webContents.send = (channel, payload) => this.sent.push([channel, payload]);
    }
    isMaximized() { return this.maximized; }
    isFullScreen() { return this.fullScreen; }
    loadURL() { return Promise.resolve(); }
  }
  return createWindowManager({
    BrowserWindow: FakeWindow,
    nativeTheme: { shouldUseDarkColors: false },
    windowKinds,
    windowChrome,
    preloadPath: 'preload.cjs',
    isPackaged: true,
    rendererTarget: (kind, id) => `file:///index.html?window=${kind}${id ? `&id=${id}` : ''}`,
    isApprovedFrame: () => true,
    flushEngine: async () => {},
    openExternal: () => {},
  });
}

test('Linux keeps transparent rounded windows; Windows and macOS use opaque snappable windows', () => {
  assert.deepEqual(windowChrome('linux', false), { transparent: true, backgroundColor: '#00000000' });
  for (const platform of ['win32', 'darwin']) {
    assert.equal(windowChrome(platform, false).transparent, false, platform);
    assert.notEqual(windowChrome(platform, true).backgroundColor, windowChrome(platform, false).backgroundColor, platform);
  }
});

test('every window kind takes its chrome from the platform policy and disables spellcheck', () => {
  const manager = createHarness();
  for (const kind of Object.keys(windowKinds)) {
    const win = manager.createWindow(kind, kind === 'note' ? '49fcbca1-94f1-48bb-8f1d-8a2261d50bb4' : undefined);
    assert.deepEqual({ transparent: win.options.transparent, backgroundColor: win.options.backgroundColor }, windowChrome(process.platform, false), kind);
    assert.equal(win.options.webPreferences.spellcheck, false, kind);
  }
});

test('main process pushes maximize and fullscreen changes as the window state', () => {
  const manager = createHarness();
  const win = manager.createWindow('canvas');
  win.maximized = true; win.emit('maximize');
  win.maximized = false; win.emit('unmaximize');
  win.fullScreen = true; win.emit('enter-full-screen');
  assert.ok(win.sent.every(([, state]) => state.rounded === windowChrome(process.platform).transparent));
  assert.deepEqual(win.sent.map(([channel, state]) => [channel, state.maximized]), [
    ['memoivy:window-state', true], ['memoivy:window-state', false], ['memoivy:window-state', true],
  ]);
});
