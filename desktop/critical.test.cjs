'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { EventEmitter } = require('node:events');
const { PassThrough, Writable } = require('node:stream');
const { validateRequest, validateWindowTarget } = require('./ipc_validation.cjs');
const { EngineTransport } = require('./engine_transport.cjs');
const { isSafeExternal, openExternal } = require('./external_links.cjs');

const noteId = '49fcbca1-94f1-48bb-8f1d-8a2261d50bb4';

test('external URL policy is shared and only valid bounded web or mail links open', async () => {
  for (const url of ['https://example.com/path', 'http://example.com', 'mailto:user@example.com']) assert.equal(isSafeExternal(url), true);
  for (const url of ['file:///etc/passwd', 'javascript:alert(1)', 'https://user:pass@example.com', 'https://example.com/' + 'a'.repeat(4096), null]) assert.equal(isSafeExternal(url), false);

  const opened = [];
  await openExternal('https://example.com', async (url) => opened.push(url));
  assert.deepEqual(opened, ['https://example.com']);
  await assert.rejects(openExternal('file:///etc/passwd', async () => {}), /HTTP, HTTPS, and mailto/);
  await assert.rejects(openExternal('https://example.com', async () => { throw new Error('shell failed'); }), /shell failed/);
});

test('IPC window targets allow settings and reject invalid kinds or ids', () => {
  assert.deepEqual(validateWindowTarget('settings'), { kind: 'settings', id: undefined });
  assert.deepEqual(validateWindowTarget('canvas', noteId), { kind: 'canvas', id: noteId });
  assert.throws(() => validateWindowTarget('unknown'), /Invalid window target/);
  assert.throws(() => validateWindowTarget('settings', 'invalid'), /Invalid window target/);
});

test('IPC boundary rejects unsafe, stale-shape and invalid preference payloads', () => {
  assert.throws(() => validateRequest('exec', {}), { code: 'INVALID_REQUEST' });
  assert.throws(() => validateRequest('delete', { baseRevision: 1, id: noteId, path: '/etc/passwd' }), { code: 'INVALID_REQUEST' });
  assert.throws(() => validateRequest('create', { baseRevision: 0, color: '#ffffff' }), { code: 'INVALID_REQUEST' });
  assert.throws(() => validateRequest('canvas', { baseRevision: 0, changes: [{ id: noteId, canvas: { position: [-20001, 0] } }] }), { code: 'INVALID_REQUEST' });
  assert.throws(() => validateRequest('preference', { baseRevision: 0, key: 'arbitrary', value: true }), { code: 'INVALID_REQUEST' });
  assert.doesNotThrow(() => validateRequest('preference', { baseRevision: 4, key: 'expandedNotes', value: [noteId] }));
  const pastedImage = Buffer.alloc(128 * 1024, 0x61).toString('base64');
  assert.doesNotThrow(() => validateRequest('importAsset', { baseRevision: 4, name: 'pasted.png', data: pastedImage, mime: 'image/png' }));
  assert.throws(() => validateRequest('importAsset', { baseRevision: 4, name: 'pasted.png', data: `${pastedImage}!` }), { code: 'INVALID_REQUEST' });
});

class FakeEngine extends EventEmitter {
  constructor(respond = true) {
    super();
    this.stdout = new PassThrough();
    this.stderr = new PassThrough();
    this.killed = false;
    this.stdin = new Writable({
      write: (chunk, _encoding, callback) => {
        if (respond) {
          const request = JSON.parse(String(chunk));
          setImmediate(() => this.stdout.write(`${JSON.stringify({ version: 1, id: request.id, result: { snapshot: { revision: 7, savedRevision: 7, notes: [], assets: [] } } })}\n`));
        }
        callback();
      },
    });
  }
  kill() { this.killed = true; }
}

test('engine transport correlates a protocol response and terminates on process death', async () => {
  const child = new FakeEngine();
  const transport = new EngineTransport({ executable: 'test-engine', spawnProcess: () => child });
  const result = await transport.request('snapshot', {});
  assert.equal(result.snapshot.revision, 7);

  const stopped = new FakeEngine(false);
  const terminal = new EngineTransport({ executable: 'test-engine', timeoutMs: 3000, spawnProcess: () => stopped });
  const pending = terminal.request('create', { baseRevision: 7, text: 'draft' });
  stopped.emit('exit', 1, null);
  await assert.rejects(pending, /Engine exited/);
  await assert.rejects(terminal.request('create', { baseRevision: 7, text: 'draft' }), /Engine exited/);
  assert.equal(stopped.killed, true);
});

test('IPC boundary accepts the canvas link style preference and rejects unknown shapes', () => {
  validateRequest('preference', { baseRevision: 0, key: 'canvasLinkStyle', value: 'orthogonal' });
  assert.throws(() => validateRequest('preference', { baseRevision: 0, key: 'canvasLinkStyle', value: 'zigzag' }), /canvasLinkStyle/);
});

test('IPC boundary accepts the appearance preference and rejects unknown choices', () => {
  validateRequest('preference', { baseRevision: 0, key: 'appearance', value: { theme: 'light', motion: 'full' } });
  assert.throws(() => validateRequest('preference', { baseRevision: 0, key: 'appearance', value: { theme: 'sepia', motion: 'full' } }), /appearance/);
  assert.throws(() => validateRequest('preference', { baseRevision: 0, key: 'appearance', value: { theme: 'light' } }));
});

test('preload window allowlist matches window_kinds.json (except note)', () => {
  const source = require('node:fs').readFileSync(require('node:path').join(__dirname, 'preload.cjs'), 'utf8');
  const listed = JSON.parse(source.match(/openWindow: \(kind, id\) => \{[\s\S]*?(\[[^\]]*\])\.includes\(kind\)/)[1].replaceAll("'", '"'));
  const kinds = Object.keys(require('./window_kinds.json')).filter((kind) => kind !== 'note');
  assert.deepEqual([...listed].sort(), kinds.sort());
});
