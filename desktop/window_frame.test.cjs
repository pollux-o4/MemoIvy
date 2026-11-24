'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { EventEmitter } = require('node:events');
const { readFileSync } = require('node:fs');
const { createRequire } = require('node:module');
const vm = require('node:vm');
const path = require('node:path');

test('Linux keeps transparent rounded windows; Windows and macOS use opaque snappable windows', () => {
  const { windowChrome } = require('./window_chrome.cjs');
  assert.deepEqual(windowChrome('linux', false), { transparent: true, backgroundColor: '#00000000' });
  for (const platform of ['win32', 'darwin']) {
    assert.equal(windowChrome(platform, false).transparent, false, platform);
    assert.notEqual(windowChrome(platform, true).backgroundColor, windowChrome(platform, false).backgroundColor, platform);
  }
});

test('every window kind takes its chrome from the platform policy', () => {
  const localRequire = createRequire(__filename);
  class FakeWindow extends EventEmitter {
    constructor(options) {
      super(); this.options = options;
      this.webContents = new EventEmitter();
      this.webContents.setWindowOpenHandler = () => {};
    }
    loadURL() { return Promise.resolve(); }
  }
  const app = { setName() {}, setDesktopName() {}, commandLine: { appendSwitch() {} }, requestSingleInstanceLock: () => false, quit() {}, setPath() {} };
  const context = { require: (name) => name === 'electron' ? { app, BrowserWindow: FakeWindow, nativeTheme: { shouldUseDarkColors: false } } : localRequire(name), process, console, URL, URLSearchParams, __dirname, module: { exports: {} } };
  vm.runInNewContext(`${readFileSync(path.join(__dirname, 'main.cjs'), 'utf8')}\nmodule.exports.createWindow = createWindow;`, context);
  for (const kind of Object.keys(localRequire('./window_kinds.json'))) {
    const win = context.module.exports.createWindow(kind, kind === 'note' ? '49fcbca1-94f1-48bb-8f1d-8a2261d50bb4' : undefined);
    const { windowChrome } = require('./window_chrome.cjs');
    assert.deepEqual({ transparent: win.options.transparent, backgroundColor: win.options.backgroundColor }, windowChrome(process.platform, false), kind);
  }
});

test('main process pushes maximize and fullscreen changes as the window state', () => {
  const localRequire = createRequire(__filename);
  class FakeWindow extends EventEmitter {
    constructor(options) {
      super(); this.options = options; this.maximized = false; this.fullScreen = false; this.sent = [];
      this.webContents = new EventEmitter();
      this.webContents.setWindowOpenHandler = () => {};
      this.webContents.isDestroyed = () => false;
      this.webContents.send = (channel, payload) => this.sent.push([channel, payload]);
    }
    isMaximized() { return this.maximized; }
    isFullScreen() { return this.fullScreen; }
    loadURL() { return Promise.resolve(); }
  }
  const app = { setName() {}, setDesktopName() {}, commandLine: { appendSwitch() {} }, requestSingleInstanceLock: () => false, quit() {}, setPath() {} };
  const context = { require: (name) => name === 'electron' ? { app, BrowserWindow: FakeWindow, nativeTheme: { shouldUseDarkColors: false } } : localRequire(name), process, console, URL, URLSearchParams, __dirname, module: { exports: {} } };
  vm.runInNewContext(`${readFileSync(path.join(__dirname, 'main.cjs'), 'utf8')}\nmodule.exports.createWindow = createWindow;`, context);
  const win = context.module.exports.createWindow('canvas');
  win.maximized = true; win.emit('maximize');
  win.maximized = false; win.emit('unmaximize');
  win.fullScreen = true; win.emit('enter-full-screen');
  const { windowChrome } = require('./window_chrome.cjs');
  assert.ok(win.sent.every(([, state]) => state.rounded === windowChrome(process.platform).transparent));
  assert.deepEqual(win.sent.map(([channel, state]) => [channel, state.maximized]), [
    ['memoivy:window-state', true], ['memoivy:window-state', false], ['memoivy:window-state', true],
  ]);
});
