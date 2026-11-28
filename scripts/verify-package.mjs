#!/usr/bin/env node
// 설치본 산출물 검사 SSoT: 엔진 동봉 위치(electron-builder.yml extraResources)와 대상별 검사를 한곳에 둔다(OPS-005).
// 사용: node scripts/verify-package.mjs <windows|snap|linux>
import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';

const installers = path.resolve(import.meta.dirname, '../artifacts/installers');
const ENGINE = 'resources/engine/memoivy-engine'; // electron-builder.yml의 extraResources `to`와 같아야 한다.
const EXPECTED_LOCALES = ['en-US.pak', 'ko.pak'];

function fail(message, detail = '') {
  if (detail) console.error(detail);
  throw new Error(message);
}

function readAsarEntries(asarPath) {
  const buf = readFileSync(asarPath);
  const headerSize = buf.readUInt32LE(12);
  const header = JSON.parse(buf.subarray(16, 16 + headerSize).toString('utf8'));
  return Object.keys(header.files ?? {});
}

function assertExpectedLocales(found, label) {
  const actual = [...found].sort();
  if (actual.length !== EXPECTED_LOCALES.length || actual.some((item, index) => item !== EXPECTED_LOCALES[index])) {
    fail(`${label} locales mismatch: expected [${EXPECTED_LOCALES.join(', ')}], got [${actual.join(', ')}]`);
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
    verifyUnpacked(unpacked, `${ENGINE}.exe`);
    const engine = path.join(unpacked, `${ENGINE}.exe`);
    // 러너에는 VC 런타임이 깔려 있어 e2e로는 드러나지 않으므로, 엔진이 시스템 DLL만 쓰는지 직접 확인한다.
    const vswhere = path.join(process.env['ProgramFiles(x86)'] ?? '', 'Microsoft Visual Studio/Installer/vswhere.exe');
    const dumpbin = execFileSync(vswhere, ['-latest', '-find', 'VC\\Tools\\MSVC\\*\\bin\\Hostx64\\x64\\dumpbin.exe'], { encoding: 'utf8' }).split(/\r?\n/)[0];
    const deps = execFileSync(dumpbin, ['/dependents', engine], { encoding: 'utf8' });
    if (/vcruntime|msvcp|api-ms-win-crt/i.test(deps)) fail('Windows engine depends on the VC runtime', deps);
  },
  linux() {
    verifyUnpacked(path.join(installers, 'linux-unpacked'), ENGINE);
  },
  snap() {
    const snaps = readdirSync(installers).filter((name) => name.endsWith('.snap'));
    if (snaps.length !== 1) fail(`Expected one snap, found ${snaps.length}`);
    const listing = execFileSync('unsquashfs', ['-ll', path.join(installers, snaps[0])], { encoding: 'utf8', maxBuffer: 64 << 20 });
    if (!listing.split('\n').some((line) => line.endsWith(ENGINE))) fail(`Snap engine missing: ${ENGINE}`);
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
