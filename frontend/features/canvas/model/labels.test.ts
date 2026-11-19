import { describe, expect, test } from 'vitest';
import { canvasTitle } from './labels';
import type { Note } from '../../../shared/engine_contract';

const note = (text: string, project = false): Note => ({ id: 'id', text, color: 'yellow', modified: 0, canvas: { position: null, next: [], done: false, project } });

describe('canvas title labels', () => {
  test('uses GTK placeholders while preserving formatted titles', () => {
    expect(canvasTitle(note(''))).toBe('빈 메모');
    expect(canvasTitle(note('', true))).toBe('이름 없는 프로젝트');
    expect(canvasTitle(note('# 정리할 일'))).toBe('정리할 일');
  });
});
