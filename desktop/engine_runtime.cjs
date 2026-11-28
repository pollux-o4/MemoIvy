'use strict';

const defaultFs = require('node:fs');
const { EngineTransport } = require('./engine_transport.cjs');

function isTransportFailure(error) {
  return error?.code === 'ENGINE_UNAVAILABLE' || /Engine (?:exited|sent malformed|request timed out)/.test(error?.message || '');
}

/**
 * Rust 엔진 자식 프로세스 수명 주기·스냅숏 버전 단조성·단일 시작 경합을 소유한다.
 */
function createEngineRuntime({
  dataDirectory,
  legacyNotesPath,
  engineExecutable,
  Transport = EngineTransport,
  fs = defaultFs,
  onSnapshot = () => {},
  onEvent = () => {},
  onError = () => {},
}) {
  let engine;
  let engineFailure;
  let engineConfig;
  let launchPromise;
  let currentSnapshot;

  function applySnapshot(snapshot) {
    if (!snapshot || (currentSnapshot && snapshot.revision <= currentSnapshot.revision)) return;
    currentSnapshot = snapshot;
    onSnapshot(snapshot);
  }

  function setFailure(error) {
    engineFailure = error;
    onError({
      code: error.code || 'ENGINE_UNAVAILABLE',
      message: error.message || 'MemoIvy engine is unavailable',
      snapshot: currentSnapshot,
    });
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

  async function spawnAndReadSnapshot() {
    const dir = dataDirectory();
    fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
    try { fs.chmodSync(dir, 0o700); } catch { /* ACL based platforms do not expose POSIX modes. */ }
    const executable = engineExecutable();
    const args = ['--data-dir', dir];
    const legacy = legacyNotesPath(dir);
    if (legacy) args.push('--legacy', legacy);
    engineConfig = { executable, args };
    const transport = new Transport({ executable, args });
    engine = transport;
    transport.on('event', (event) => {
      if (engine !== transport) return;
      if (event.snapshot) applySnapshot(event.snapshot);
      else onEvent(event);
    });
    transport.on('failure', (error) => { if (engine === transport) setFailure(error); });
    transport.on('stderr', (line) => { if (engine === transport) console.error(`[memoivy-engine] ${line.trimEnd()}`); });
    transport.start();
    const initial = await transport.request('snapshot', {});
    if (engine !== transport) throw new Error('A newer engine process replaced this startup');
    engineFailure = null;
    applySnapshot(initial.snapshot);
  }

  function launch() {
    if (launchPromise) return launchPromise;
    launchPromise = spawnAndReadSnapshot().finally(() => { launchPromise = null; });
    return launchPromise;
  }

  async function ensureRunning() {
    if (engineFailure && engineConfig) await launch();
  }

  async function request(op, params = {}, { recover = false, markFailureOnTransportError = false } = {}) {
    try {
      if (recover) await ensureRunning();
      const result = await requireEngine().request(op, params);
      applySnapshot(result.snapshot);
      return result;
    } catch (error) {
      if (markFailureOnTransportError && isTransportFailure(error)) setFailure(error);
      throw error;
    }
  }

  async function flush() {
    await ensureRunning();
    await requireEngine().request('flush', {});
  }

  async function close() {
    const child = engine?.child;
    if (!child || engine.closed || engine.deadError) return;
    const exited = new Promise((resolve) => child.once('exit', resolve));
    engine.closed = true;
    engine.closing = true;
    child.stdin.end();
    let timer;
    const didExit = await Promise.race([
      exited.then(() => true),
      new Promise((resolve) => { timer = setTimeout(() => resolve(false), 2500); }),
    ]);
    clearTimeout(timer);
    if (!didExit) {
      child.kill();
      await Promise.race([exited, new Promise((resolve) => setTimeout(resolve, 1000))]);
    }
  }

  return {
    launch,
    request,
    flush,
    close,
    setFailure,
    snapshot: () => currentSnapshot,
  };
}

module.exports = { createEngineRuntime };
