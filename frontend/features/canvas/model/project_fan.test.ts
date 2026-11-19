import { describe, expect, test } from 'vitest';
import type { Note } from '../../../shared/engine_contract';
import { projectFan } from './project_fan';

const note = (id: string, modified: number): Note => ({ id, text: id, color: 'yellow', modified, canvas: { position: null, next: [], done: false, project: false } });

describe('project fan', () => {
  test('picks the most recently modified children and counts the rest', () => {
    const children = [note('a', 1), note('b', 4), note('c', 2), note('d', 3), note('e', 0)];
    const fan = projectFan(children);
    expect(fan.recent.map((item) => item.id)).toEqual(['b', 'd', 'c']);
    expect(fan.more).toBe(2);
    expect(children.map((item) => item.id)).toEqual(['a', 'b', 'c', 'd', 'e']);
  });

  test('shows everything without overflow when within the limit', () => {
    expect(projectFan([note('a', 1)])).toEqual({ recent: [note('a', 1)], more: 0 });
    expect(projectFan([])).toEqual({ recent: [], more: 0 });
  });
});
