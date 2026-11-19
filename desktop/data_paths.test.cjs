'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { test } = require('node:test');
const { legacyNotesPath, snapDataDirectory } = require('./data_paths.cjs');

test('Snap uses a shared SQLite directory and discovers the copied GTK revision', (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'memoivy-snap-paths-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const env = { SNAP_NAME: 'memoivy', SNAP_USER_COMMON: path.join(root, 'common'), SNAP_USER_DATA: path.join(root, '4') };
  const legacy = path.join(env.SNAP_USER_DATA, '.local', 'share', 'memoivy', 'notes.json');
  fs.mkdirSync(path.dirname(legacy), { recursive: true });
  fs.writeFileSync(legacy, '[]');
  const shared = snapDataDirectory(env);
  assert.equal(shared, path.join(env.SNAP_USER_COMMON, 'memoivy'));
  assert.equal(legacyNotesPath(shared, env, 'linux'), legacy);
  assert.equal(legacyNotesPath(shared, {}, 'linux'), undefined);
  assert.equal(snapDataDirectory({ ...env, SNAP_NAME: 'another' }), undefined);
  assert.equal(snapDataDirectory({ ...env, SNAP_USER_COMMON: '/' }), undefined);
});

test('native Linux can import the previous application directory', (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'memoivy-linux-paths-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const legacy = path.join(root, 'linux-sticky-memo', 'notes.json');
  fs.mkdirSync(path.dirname(legacy));
  fs.writeFileSync(legacy, '[]');
  assert.equal(legacyNotesPath(path.join(root, 'memoivy'), { XDG_DATA_HOME: root }, 'linux'), legacy);
});
