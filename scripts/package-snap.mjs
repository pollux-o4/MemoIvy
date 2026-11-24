import { cpSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const { version } = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));
if (process.platform !== 'linux' || process.arch !== 'x64') throw new Error('Run on Linux x64.');
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Use a numeric release version.');
const output = path.join(root, 'artifacts/installers');
const staged = mkdtempSync(path.join(output, '.snap-project-'));

try {
  const payload = path.join(staged, 'payload');
  cpSync(path.join(output, 'linux-unpacked'), path.join(payload, 'app'), {
    recursive: true,
    // Snap confinement supplies isolation. The unused setuid helper must not ship.
    filter: (file) => path.basename(file) !== 'chrome-sandbox',
  });
  writeFileSync(path.join(payload, 'release-version'), version);
  cpSync(path.join(root, 'snap/snapcraft.yaml'), path.join(staged, 'snap/snapcraft.yaml'));
  const desktopFile = path.join(payload, 'usr/share/applications/io.github.pollux_o4.MemoIvy.desktop');
  const iconPath = 'usr/share/icons/hicolor/512x512/apps/memoivy.png';
  const icon = path.join(payload, iconPath);
  mkdirSync(path.dirname(desktopFile), { recursive: true });
  mkdirSync(path.dirname(icon), { recursive: true });
  const entry = readFileSync(path.join(root, 'integration/memoivy.desktop'), 'utf8')
    .replace(/^Exec=.*$/m, 'Exec=memoivy')
    .replace(/^Icon=.*$/m, 'Icon=${SNAP}/' + iconPath);
  writeFileSync(desktopFile, entry);
  cpSync(path.join(root, 'desktop/icon.png'), icon);

  const filename = `memoivy_${version}_amd64.snap`;
  const result = spawnSync('snapcraft', ['pack', '--use-lxd'], { cwd: staged, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Snapcraft exited with ${result.status ?? result.signal}`);
  cpSync(path.join(staged, filename), path.join(output, filename));
} finally {
  rmSync(staged, { recursive: true, force: true });
}
