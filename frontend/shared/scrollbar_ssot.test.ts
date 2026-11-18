import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

// 스크롤바 모양의 원본은 shared/scrollbar.css 하나다(UI-001). 다른 소스(CSS·TS·Electron 설정)는 스크롤바를 언급하지 않는다.
// 표준 scrollbar-width/color와 Chromium OverlayScrollbar는 원본의 ::-webkit-scrollbar를 무시하게 만든다.
const repo = join(__dirname, '../..');
const owner = 'frontend/shared/scrollbar.css';
const scanned = ['frontend', 'desktop'];
const sourceFile = /\.(css|tsx?|c?js)$/;
const skipped = new Set(['dist', 'node_modules']);
const scrollbarToken = /scrollbar(?!-gutter)|scrollbarWidth|scrollbarColor|OverlayScrollbar|insertCSS/i;
const self = relative(repo, __filename);
const ownerImport = /^.*import\b.*scrollbar\.css.*$/gm;

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    if (skipped.has(name)) return [];
    const path = join(dir, name);
    return statSync(path).isDirectory() ? sources(path) : sourceFile.test(name) ? [relative(repo, path)] : [];
  });
}

describe('scrollbar style SSoT', () => {
  it('only the owner stylesheet touches scrollbars', () => {
    const offenders = scanned.flatMap((dir) => sources(join(repo, dir))).filter((path) => path !== owner && path !== self && scrollbarToken.test(readFileSync(join(repo, path), 'utf8').replace(ownerImport, '')));
    expect(offenders).toEqual([]);
  });

  it('the owner styles the thumb without standard properties', () => {
    const css = readFileSync(join(repo, owner), 'utf8');
    expect(css).toMatch(/::-webkit-scrollbar-thumb\s*\{/);
    expect(css).not.toMatch(/scrollbar-(width|color)\s*:/);
  });
});
