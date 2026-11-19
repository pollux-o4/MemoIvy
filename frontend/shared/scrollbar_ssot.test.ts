import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { relativeToRepo, repo, sourceFiles } from '../test_support/source_files';

// 스크롤바 모양의 원본은 shared/scrollbar.css 하나다(UI-001). 다른 소스(CSS·TS·Electron 설정)는 스크롤바를 언급하지 않는다.
// 표준 scrollbar-width/color와 Chromium OverlayScrollbar는 원본의 ::-webkit-scrollbar를 무시하게 만든다.
const owner = 'frontend/shared/scrollbar.css';
const scanned = ['frontend', 'desktop'];
const sourceFile = /\.(css|tsx?|c?js)$/;
const scrollbarToken = /scrollbar(?!-gutter)|scrollbarWidth|scrollbarColor|OverlayScrollbar|insertCSS/i;
const self = relativeToRepo(__filename);
const ownerImport = /^.*import\b.*scrollbar\.css.*$/gm;

describe('scrollbar style SSoT', () => {
  it('only the owner stylesheet touches scrollbars', () => {
    const offenders = scanned.flatMap((dir) => sourceFiles(dir, sourceFile)).filter((path) => path !== owner && path !== self && scrollbarToken.test(readFileSync(join(repo, path), 'utf8').replace(ownerImport, '')));
    expect(offenders).toEqual([]);
  });

  it('the owner styles the thumb without standard properties', () => {
    const css = readFileSync(join(repo, owner), 'utf8');
    expect(css).toMatch(/::-webkit-scrollbar-thumb\s*\{/);
    expect(css).not.toMatch(/scrollbar-(width|color)\s*:/);
  });
});
