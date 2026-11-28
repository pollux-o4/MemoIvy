#!/usr/bin/env node
// 설치본 산출물 검사 SSoT: 엔진 동봉 위치(electron-builder.yml extraResources)와 대상별 검사를 한곳에 둔다(OPS-005).
// 사용: node scripts/verify-package.mjs <windows|snap|linux>
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const installers = path.join(root, 'artifacts/installers');

function fail(message, detail = '') {
  if (detail) console.error(detail);
  throw new Error(message);
}

/** electron-builder.yml(SSoT)에서 동봉 엔진 경로와 유지할 언어 목록을 직접 읽는다. */
function readPackagingSpec() {
  const yml = readFileSync(path.join(root, 'desktop/electron-builder.yml'), 'utf8');
  const lines = yml.split(/\r?\n/);
  const languages = [];
  let section = null;
  let inLanguages = false;
  let inExtra = false;
  let unixEngine;
  let winEngine;

  for (const raw of lines) {
    const line = raw.replace(/#.*$/, '');
    if (!line.trim()) continue;
    const top = line.match(/^([A-Za-z0-9_]+):/);
    if (top) {
      section = top[1];
      inLanguages = section === 'electronLanguages';
      inExtra = false;
      continue;
    }
    if (inLanguages) {
      const item = line.match(/^\s+-\s+([^\s]+)\s*$/);
      if (item) languages.push(item[1].replace(/^['"]|['"]$/g, ''));
      continue;
    }
    if (section === 'mac' || section === 'win') {
      if (/^\s{2}extraResources:/.test(line)) {
        inExtra = true;
        continue;
      }
      if (/^\s{2}[A-Za-z0-9_]+:/.test(line)) {
        inExtra = false;
        continue;
      }
      if (inExtra) {
        const to = line.match(/^\s+to:\s+([^\s]+)\s*$/);
        if (to) {
          const target = `resources/${to[1].replace(/^['"]|['"]$/g, '')}`;
          if (section === 'mac') unixEngine = target;
          else winEngine = target;
        }
      }
    }
  }

  if (!languages.length) fail('electronLanguages missing in desktop/electron-builder.yml');
  if (!unixEngine) fail('mac.extraResources engine target missing in desktop/electron-builder.yml');
  if (!winEngine) fail('win.extraResources engine target missing in desktop/electron-builder.yml');
  return {
    unixEngine,
    winEngine,
    expectedLocales: languages.map((lang) => `${lang}.pak`).sort(),
  };
}

const SPEC = readPackagingSpec();

function readAsarEntries(asarPath) {
  const buf = readFileSync(asarPath);
  const headerSize = buf.readUInt32LE(12);
  const header = JSON.parse(buf.subarray(16, 16 + headerSize).toString('utf8'));
  return Object.keys(header.files ?? {});
}

function assertExpectedLocales(found, label) {
  const actual = [...found].sort();
  if (actual.length !== SPEC.expectedLocales.length || actual.some((item, index) => item !== SPEC.expectedLocales[index])) {
    fail(`${label} locales mismatch: expected [${SPEC.expectedLocales.join(', ')}], got [${actual.join(', ')}]`);
  }
}

function verifyUnpacked(dir, engineRelPath) {
  const engine = path.join(dir, engineRelPath);
  if (!existsSync(engine)) fail(`Engine missing: ${engine}`);
  const asarPath = path.join(dir, 'resources/app.asar');
  if (!existsSync(asarPath)) fail(`app.asar missing: ${asarPath}`);
  if (readAsarEntries(asarPath).includes('node_modules')) fail('app.asar must not bundle node_modules');
  const locales = readdirSync(path.join(dir, 'locales')).filter((name) => name.endsWith('.pak'));
  assertExpectedLocales(locales, path.basename(dir));
}

// 대상별 검사(Strategy): 새 배포 대상은 여기에 검사 하나만 더한다.
const checks = {
  windows() {
    const unpacked = path.join(installers, 'win-unpacked');
    verifyUnpacked(unpacked, SPEC.winEngine);
    const engine = path.join(unpacked, SPEC.winEngine);
    // 러너에는 VC 런타임이 깔려 있어 e2e로는 드러나지 않으므로, 엔진이 시스템 DLL만 쓰는지 직접 확인한다.
    const vswhere = path.join(process.env['ProgramFiles(x86)'] ?? '', 'Microsoft Visual Studio/Installer/vswhere.exe');
    const dumpbin = execFileSync(vswhere, ['-latest', '-find', 'VC\\Tools\\MSVC\\*\\bin\\Hostx64\\x64\\dumpbin.exe'], { encoding: 'utf8' }).split(/\r?\n/)[0];
    const deps = execFileSync(dumpbin, ['/dependents', engine], { encoding: 'utf8' });
    if (/vcruntime|msvcp|api-ms-win-crt/i.test(deps)) fail('Windows engine depends on the VC runtime', deps);
  },
  linux() {
    verifyUnpacked(path.join(installers, 'linux-unpacked'), SPEC.unixEngine);
  },
  snap() {
    const snaps = readdirSync(installers).filter((name) => name.endsWith('.snap'));
    if (snaps.length !== 1) fail(`Expected one snap, found ${snaps.length}`);
    const listing = execFileSync('unsquashfs', ['-ll', path.join(installers, snaps[0])], { encoding: 'utf8', maxBuffer: 64 << 20 });
    if (!listing.split('\n').some((line) => line.endsWith(SPEC.unixEngine))) fail(`Snap engine missing: ${SPEC.unixEngine}`);
    const locales = [];
    for (const line of listing.split('\n')) {
      const match = line.match(/\/locales\/([^/\s]+\.pak)$/);
      if (match) locales.push(match[1]);
    }
    assertExpectedLocales(locales, 'Snap');
  },
};

const target = process.argv[2];
if (!Object.hasOwn(checks, target)) fail(`Unknown target: ${target} (${Object.keys(checks).join('|')})`);
checks[target]();
console.log(`${target} package OK`);
