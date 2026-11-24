'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const { roleOf, homeToReplace, shouldShowLibrary } = require('./window_policy.cjs');

function once(values) {
  return (function* () { yield* values; })();
}

test('roles distinguish home, note work, and auxiliary windows', () => {
  for (const key of ['library', 'canvas']) assert.equal(roleOf(key), 'home');
  assert.equal(roleOf('note:123'), 'work');
  for (const key of ['settings', 'note', '', 'other']) assert.equal(roleOf(key), 'aux');
});

test('home replacement selects only the other home in either direction', () => {
  assert.equal(homeToReplace('canvas', once(['note:1', 'canvas', 'settings', 'library'])), 'library');
  assert.equal(homeToReplace('library', once(['library', 'canvas', 'note:1'])), 'canvas');
  assert.equal(homeToReplace('library', once(['library', 'note:1'])), null);
  assert.equal(homeToReplace('canvas', []), null);
  for (const key of ['note:1', 'settings']) {
    assert.equal(homeToReplace(key, once(['library', 'canvas'])), null);
  }
});

test('closing library never reopens library', () => {
  for (const keys of [[], ['note:1'], ['canvas'], ['settings']]) {
    assert.equal(shouldShowLibrary('library', once(keys)), false);
  }
});

test('remaining notes, canvas, or library prevent a return from home/work', () => {
  for (const closingKey of ['note:1', 'canvas']) {
    for (const key of ['note:2', 'canvas', 'library']) {
      assert.equal(shouldShowLibrary(closingKey, once(['settings', key])), false);
    }
  }
});

test('closing last home/work shows library even if only auxiliary windows remain', () => {
  for (const closingKey of ['note:1', 'canvas']) {
    assert.equal(shouldShowLibrary(closingKey, once([])), true);
    assert.equal(shouldShowLibrary(closingKey, once(['settings', 'help'])), true);
  }
});

test('auxiliary close shows library only when no visible windows remain', () => {
  assert.equal(shouldShowLibrary('settings', once([])), true);
  for (const key of ['library', 'canvas', 'note:1', 'help']) {
    assert.equal(shouldShowLibrary('settings', once([key])), false);
  }
});

function mainHarness() {
  const { EventEmitter } = require('node:events');
  const { readFileSync } = require('node:fs');
  const { createRequire } = require('node:module');
  const vm = require('node:vm');
  const path = require('node:path');
  const localRequire = createRequire(__filename);
  const created = [];
  const flushed = [];
  let flush = async (win) => { flushed.push(win); };
  class FakeWindow extends EventEmitter {
    constructor() {
      super(); this.visible = false; this.destroyed = false;
      this.webContents = new EventEmitter();
      this.webContents.setWindowOpenHandler = () => {};
      created.push(this);
    }
    isDestroyed() { return this.destroyed; }
    isVisible() { return this.visible; }
    isMinimized() { return false; }
    show() { this.visible = true; }
    hide() { this.visible = false; }
    focus() {}
    close() {
      let prevented = false;
      this.emit('close', { preventDefault() { prevented = true; } });
      if (!prevented) { this.destroyed = true; this.visible = false; this.emit('closed'); }
    }
    loadURL() { return Promise.resolve(); }
  }
  const app = { setName() {}, setDesktopName() {}, commandLine: { appendSwitch() {} }, requestSingleInstanceLock: () => false, quit() {}, setPath() {} };
  const context = {
    require: (name) => name === 'electron' ? { app, BrowserWindow: FakeWindow, nativeTheme: { shouldUseDarkColors: false } } : localRequire(name),
    process, console, URL, URLSearchParams, __dirname, module: { exports: {} },
    flush: (win) => flush(win),
  };
  vm.runInNewContext(`${readFileSync(path.join(__dirname, 'main.cjs'), 'utf8')}\nflushWindow = flush; module.exports = { createWindow, requestWindowClose };`, context);
  return { ...context.module.exports, created, flushed, setFlush: (fn) => { flush = fn; } };
}

const settle = () => new Promise((resolve) => setImmediate(resolve));

test('new home replaces the previous home only after ready-to-show and flush', async () => {
  const h = mainHarness();
  const library = h.createWindow('library');
  library.emit('ready-to-show');
  let finishFlush;
  h.setFlush(() => new Promise((resolve) => { finishFlush = resolve; }));
  const canvas = h.createWindow('canvas');
  assert.equal(library.isDestroyed(), false);
  canvas.emit('ready-to-show');
  assert.equal(canvas.isVisible(), true);
  assert.equal(library.isDestroyed(), false);
  finishFlush();
  await settle();
  assert.equal(library.isDestroyed(), true);
  assert.equal(h.created.length, 2);
});

test('reused hidden home is shown and replaces another home without reopening library', async () => {
  const h = mainHarness();
  const canvas = h.createWindow('canvas');
  canvas.emit('ready-to-show');
  canvas.hide();
  const library = h.createWindow('library');
  assert.equal(h.createWindow('canvas'), canvas);
  await settle();
  assert.equal(canvas.isVisible(), true);
  assert.equal(library.isDestroyed(), true);
  assert.deepEqual(h.flushed, [library]);
  assert.equal(h.created.length, 2);
});

test('OS closing library with an open note still hides it after flush', async () => {
  const h = mainHarness();
  const library = h.createWindow('library');
  library.emit('ready-to-show');
  h.createWindow('note', '1').emit('ready-to-show');
  library.close();
  await settle();
  assert.equal(library.isDestroyed(), false);
  assert.equal(library.isVisible(), false);
  assert.deepEqual(h.flushed, [library]);
});

test('replacement upgrades an in-flight hide to a real close', async () => {
  const h = mainHarness();
  const library = h.createWindow('library');
  library.emit('ready-to-show');
  h.createWindow('note', '1').emit('ready-to-show');
  let finishFlush;
  h.setFlush(() => new Promise((resolve) => { finishFlush = resolve; }));
  library.close();
  h.createWindow('canvas').emit('ready-to-show');
  finishFlush();
  await settle();
  assert.equal(library.isDestroyed(), true);
  assert.equal(h.created.length, 3);
});
