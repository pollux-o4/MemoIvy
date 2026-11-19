'use strict';

const fs = require('node:fs');
const path = require('node:path');

function absoluteDirectory(value) {
  return value && path.isAbsolute(value) && path.resolve(value) !== path.parse(value).root ? path.resolve(value) : undefined;
}

function linuxDataHome(env) {
  return absoluteDirectory(env.XDG_DATA_HOME)
    ?? (absoluteDirectory(env.HOME) && path.join(env.HOME, '.local', 'share'));
}

function snapDataDirectory(env) {
  const common = absoluteDirectory(env.SNAP_USER_COMMON);
  return env.SNAP_NAME === 'memoivy' && common ? path.join(common, 'memoivy') : undefined;
}

function legacyNotesPath(dataDir, env = process.env, platform = process.platform, snapDir = snapDataDirectory(env)) {
  const candidates = [path.join(dataDir, 'notes.json')];
  if (platform === 'linux') {
    const home = linuxDataHome(env);
    const revision = absoluteDirectory(env.SNAP_USER_DATA);
    if (snapDir && revision) {
      // The GTK GNOME launcher stored JSON under the revision's XDG data directory.
      for (const folder of ['memoivy', 'linux-sticky-memo']) {
        candidates.push(path.join(revision, '.local', 'share', folder, 'notes.json'));
      }
    }
    if (home && !snapDir) candidates.push(path.join(home, 'linux-sticky-memo', 'notes.json'));
    if (revision && env.SNAP_NAME === 'memoivy') {
      candidates.push(path.join(revision, 'notes.json'), path.join(revision, 'memoivy', 'notes.json'));
    }
  }
  return candidates.find((candidate) => {
    try { return fs.lstatSync(candidate).isFile(); } catch { return false; }
  });
}

module.exports = { absoluteDirectory, linuxDataHome, snapDataDirectory, legacyNotesPath };
