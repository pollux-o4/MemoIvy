import { describe, expect, test } from 'vitest';
import type { Note } from './engine_contract';
import { createProgress, progressLabel } from './progress';

const note = (id: string, text: string, parent: string | null = null, project = false, done = false): Note => ({ id, text, color: 'yellow', modified: 0, canvas: { position: null, next: [], done, parent, project } });

describe('hierarchical progress', () => {
  // p: 자기 할 일 1개 + 하위 a(할 일 일부), sub(하위 프로젝트), ref(참고 메모), flag(완료 표시만)
  const many = Array.from({ length: 100 }, (_, i) => `- [${i < 37 ? 'x' : ' '}] 일 ${i}`).join('\n');
  const notes = [
    note('p', '# 출시\n\n- [x] 계획', null, true),
    note('a', many, 'p'),
    note('sub', '', 'p', true), note('b', '- [x] 깊은 일', 'sub'),
    note('ref', '참고 링크만', 'p'),
    note('flag', '회의록', 'p', false, true),
    note('x', '- [ ] 바깥'),
  ];
  const progress = createProgress(notes);

  test('a memo counts its own tasks', () => {
    expect(progress.of(notes[1])).toEqual({ done: 37, total: 100 });
  });

  test('a project counts own tasks plus each direct child as one unit, skipping notes with nothing to do', () => {
    // 계획(완료) + a(진행 중) + sub(하위 모두 완료) + flag(완료 표시). ref는 할 일이 없어 빠진다.
    expect(progress.of(notes[0])).toEqual({ done: 3, total: 4 });
  });

  test('detail keeps every checkbox for the tooltip', () => {
    expect(progress.detail(notes[0])).toEqual({ done: 39, total: 102 });
  });

  test('labels say units for projects and tasks for memos', () => {
    expect(progressLabel(notes[0], progress)).toEqual({ short: '3/4', title: '하위 항목 4개 중 3개 완료 · 할 일 102개 중 39개' });
    expect(progressLabel(notes[1], progress)).toEqual({ short: '37/100', title: '할 일 100개 중 37개 완료' });
  });

  test('survives parent cycles', () => {
    const cyclic = [note('c1', '- [ ] 일', 'c2', true), note('c2', '', 'c1', true)];
    expect(createProgress(cyclic).of(cyclic[0])).toEqual({ done: 0, total: 1 });
  });
});
