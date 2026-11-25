'use strict';

const { contextBridge, ipcRenderer } = require('electron');
const NOTE_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const isId = (value) => typeof value === 'string' && NOTE_ID.test(value);

const flushListeners = new Set();
const engineRequest = (channel, ...args) => ipcRenderer.invoke(channel, ...args).then((envelope) => {
  if (envelope?.__memoivyError) {
    // contextBridge는 Error의 code·snapshot을 버리므로 평범한 객체로 넘기고 렌더러(desktop_errors)가 Error로 되살린다.
    const { message, code, snapshot } = envelope.__memoivyError;
    throw { __memoivyError: { message: message || 'MemoIvy engine request failed', code: code || 'ENGINE_ERROR', snapshot } };
  }
  return envelope?.__memoivyResult;
});

ipcRenderer.on('memoivy:flush', async (_event, token) => {
  if (typeof token !== 'string') return;
  const outcomes = await Promise.allSettled([...flushListeners].map((listener) => Promise.resolve().then(listener)));
  const failed = outcomes.find((outcome) => outcome.status === 'rejected');
  try {
    await ipcRenderer.invoke('memoivy:flush-ack', token, failed
      ? { ok: false, message: String(failed.reason?.message || failed.reason || 'Renderer flush failed').slice(0, 1000) }
      : { ok: true });
  } catch { /* The main process may be exiting after an acknowledged flush. */ }
});

const bridge = Object.freeze({
  request: (op, params = {}) => engineRequest('memoivy:request', op, params),
  subscribe(listener) {
    if (typeof listener !== 'function') throw new TypeError('Listener must be a function');
    const wrapped = (_event, payload) => listener(payload);
    ipcRenderer.on('memoivy:event', wrapped);
    return () => ipcRenderer.removeListener('memoivy:event', wrapped);
  },
  openNote: (id) => {
    if (!isId(id)) return Promise.reject(new TypeError('Invalid note id'));
    return ipcRenderer.invoke('memoivy:open-note', id);
  },
  // 샌드박스 preload는 window_kinds.json을 못 읽어 목록을 따로 둔다(note는 openNote 전용). 일치는 critical.test가 지킨다.
  openWindow: (kind, id) => {
    if (!['library', 'canvas', 'files', 'settings', 'help'].includes(kind) || (id !== undefined && !isId(id))) {
      return Promise.reject(new TypeError('Invalid window target'));
    }
    return ipcRenderer.invoke('memoivy:open-window', kind, id);
  },
  openExternal: (url) => {
    if (typeof url !== 'string' || url.length > 4096) return Promise.reject(new TypeError('Invalid URL'));
    return ipcRenderer.invoke('memoivy:open-external', url);
  },
  importLegacy: () => engineRequest('memoivy:import-legacy'),
  windowAction(action) {
    if (!['minimize', 'maximize', 'close'].includes(action)) throw new TypeError('Invalid window action');
    if (action === 'close') void ipcRenderer.invoke('memoivy:close-window');
    else void ipcRenderer.invoke('memoivy:window-action', action);
  },
  onWindowState(listener) {
    if (typeof listener !== 'function') throw new TypeError('Listener must be a function');
    const normalize = (state) => ({ maximized: state?.maximized === true, rounded: state?.rounded === true });
    const wrapped = (_event, state) => listener(normalize(state));
    ipcRenderer.on('memoivy:window-state', wrapped);
    void ipcRenderer.invoke('memoivy:window-state').then((state) => listener(normalize(state))).catch(() => undefined);
    return () => ipcRenderer.removeListener('memoivy:window-state', wrapped);
  },
  onFlush(listener) {
    if (typeof listener !== 'function') throw new TypeError('Listener must be a function');
    flushListeners.add(listener);
    return () => flushListeners.delete(listener);
  },
});

contextBridge.exposeInMainWorld('memoivy', bridge);
