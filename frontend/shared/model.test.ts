import { describe, expect, test } from 'vitest';
import type { Note } from './engine_contract';
import { descendantsOf, projectTasks } from './model';

const note = (id: string, text: string, parent: string | null = null, project = false): Note => ({ id, text, color: 'yellow', modified: 0, canvas: { position: null, next: [], done: false, parent, project } });

describe('project aggregates', () => {
  const notes = [note('p', '- [ ] 계획', null, true), note('a', '- [x] 끝\n- [ ] 남음', 'p'), note('sub', '', 'p', true), note('b', '- [x] 깊은 일', 'sub'), note('x', '- [ ] 바깥')];

  test('collects nested descendants only', () => {
    expect(descendantsOf('p', notes).map((item) => item.id).sort()).toEqual(['a', 'b', 'sub']);
  });

  test('sums project and descendant checklists', () => {
    expect(projectTasks(notes[0], descendantsOf('p', notes))).toEqual({ total: 4, done: 2 });
  });

  test('survives parent cycles', () => {
    const cyclic = [note('c1', '', 'c2', true), note('c2', '', 'c1', true)];
    expect(descendantsOf('c1', cyclic).map((item) => item.id)).toEqual(['c2']);
  });
});
