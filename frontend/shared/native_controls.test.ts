import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { repo, sourceFiles } from '../test_support/source_files';

// UI-001 기본 컨트롤 금지: 앱 화면은 OS·브라우저 기본 모양의 고르기·입력·대화상자를 쓰지 않는다.
const source = /(?<!\.stories|\.test)\.tsx?$/; // Storybook 검수 도구·테스트는 제외
const nativeControl = /<select\b|<datalist\b|type=["'](date|datetime-local|month|week|time|color|file|range)["']|window\.(alert|confirm|prompt)\(/;
// shortcut: 링크 주소 수정이 window.prompt를 쓴다, 앱 대화상자로 링크 편집을 만들 때 이 예외를 지운다.
const knownExceptions = new Set(['frontend/features/editor/link_preview/link_preview.ts']);

describe('native controls (UI-001)', () => {
  it('app screens draw their own pickers and dialogs', () => {
    const offenders = sourceFiles('frontend', source)
      .filter((path) => !knownExceptions.has(path) && nativeControl.test(readFileSync(join(repo, path), 'utf8')));
    expect(offenders).toEqual([]);
  });
});
