import { describe, expect, test } from 'vitest';
import type { Note } from './engine_contract';
import { descendantsOf, joinTitle, splitTitle } from './model';

const note = (id: string, text: string, parent: string | null = null, project = false): Note => ({ id, text, color: 'yellow', modified: 0, canvas: { position: null, next: [], done: false, parent, project } });

describe('project aggregates', () => {
  const notes = [note('p', '- [ ] 계획', null, true), note('a', '- [x] 끝\n- [ ] 남음', 'p'), note('sub', '', 'p', true), note('b', '- [x] 깊은 일', 'sub'), note('x', '- [ ] 바깥')];

  test('collects nested descendants only', () => {
    expect(descendantsOf('p', notes).map((item) => item.id).sort()).toEqual(['a', 'b', 'sub']);
  });


  test('survives parent cycles', () => {
    const cyclic = [note('c1', '', 'c2', true), note('c2', '', 'c1', true)];
    expect(descendantsOf('c1', cyclic).map((item) => item.id)).toEqual(['c2']);
  });
});

test('splitTitle keeps heading marker and separates body; joinTitle round-trips', () => {
  const parts = splitTitle('\n# 프로젝트\n\n소개 첫 줄\n- [ ] 할 일\n');
  expect(parts).toEqual({ marker: '# ', title: '프로젝트', body: '소개 첫 줄\n- [ ] 할 일\n' });
  expect(joinTitle({ ...parts, title: '새 이름' })).toBe('# 새 이름\n\n소개 첫 줄\n- [ ] 할 일');
  expect(joinTitle({ title: '이름만', body: '' })).toBe('이름만');
  expect(splitTitle('')).toEqual({ marker: '', title: '', body: '' });
  expect(joinTitle({ title: '할 일', body: '- [ ] 하나\n  - [ ] \n\n' })).toBe('할 일\n\n- [ ] 하나\n  - [ ] ');
});
