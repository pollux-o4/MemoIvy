import { describe, expect, it } from 'vitest';
import type { Note } from '../../shared/engine_contract';
import { libraryPath, libraryTitle, orderedLibraryChildren, searchLibraryNotes } from './library_model';

function note(id: string, text: string, modified: number, project = false, parent: string | null = null): Note {
  return { id, text, modified, color: 'yellow', canvas: { position: null, next: [], done: false, project, parent } };
}

describe('library model', () => {
  it('uses the first non-empty line as the title', () => {
    const item = note('n', '\n  # 제목  \n본문 **강조**', 1);
    expect(libraryTitle(item)).toBe('제목');
  });

  it('matches GTK case-insensitive search and puts title matches before newer body matches', () => {
    const items = [note('body', '다른 제목\nSticky body', 30), note('title', '\nSticky title', 10), note('newer', 'Sticky note', 20)];
    expect(searchLibraryNotes(items, 'sTICKY').map((item) => item.id)).toEqual(['newer', 'title', 'body']);
    expect(searchLibraryNotes(items, 'absent').map((item) => item.id)).toEqual([]);
  });

  it('sorts projects before notes and each subtree by its newest descendant', () => {
    const items = [note('old', 'old', 10), note('project', 'project', 1, true), note('child', 'child', 2, false, 'project'), note('recent', 'recent', 30, false, 'project')];
    expect(orderedLibraryChildren(null, items).map((item) => item.id)).toEqual(['project', 'old']);
    expect(orderedLibraryChildren('project', items).map((item) => item.id)).toEqual(['recent', 'child']);
  });

  it('uses project fallback titles in search locations', () => {
    const project = note('p', '', 1, true);
    const child = note('c', 'child', 2, false, 'p');
    expect(libraryPath(child, [project, child])).toEqual(['이름 없는 프로젝트']);
    expect(libraryTitle(note('blank', '', 1))).toBe('빈 메모');
  });
});
