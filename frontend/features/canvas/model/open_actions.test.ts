import { describe, expect, test } from 'vitest';
import { keyCap, openActions } from './open_actions';

describe('openActions', () => {
  test('메모는 열기 하나, 프로젝트는 들어가기 다음 메모로 열기', () => {
    expect(openActions(false).map((action) => action.label)).toEqual(['메모 열기']);
    expect(openActions(true).map((action) => [action.id, action.shortcut])).toEqual([['open', 'Enter'], ['openAsNote', 'Shift+Enter']]);
  });
  test('단축키 칩은 Shift를 ⇧로 줄인다', () => {
    expect(keyCap('Shift+Enter')).toBe('⇧ Enter');
    expect(keyCap('Enter')).toBe('Enter');
  });
});
