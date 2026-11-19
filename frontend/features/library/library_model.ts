import type { Note } from '../../shared/engine_contract';
import { projectPath, titleOf } from '../../shared/model';

export function libraryTitle(note: Note): string {
  if (!note.text.split('\n').some((line) => line.trim())) {
    return note.canvas.project ? '이름 없는 프로젝트' : '빈 메모';
  }
  return titleOf(note);
}

export function libraryPath(note: Note, notes: Note[]): string[] {
  return projectPath(note, notes, libraryTitle);
}

export function searchLibraryNotes(notes: Note[], query: string): Note[] {
  const normalized = query.toLowerCase();
  return notes
    .filter((note) => !normalized || note.text.toLowerCase().includes(normalized))
    .sort((a, b) => {
      const aTitle = firstLine(a.text).toLowerCase().includes(normalized);
      const bTitle = firstLine(b.text).toLowerCase().includes(normalized);
      return Number(bTitle) - Number(aTitle) || b.modified - a.modified;
    });
}

export function orderedLibraryChildren(parent: string | null, notes: Note[]): Note[] {
  const children = notes.filter((note) => (note.canvas.parent ?? null) === parent);
  const modified = new Map<string, number>();
  const newest = (note: Note, visiting = new Set<string>()): number => {
    const cached = modified.get(note.id);
    if (cached !== undefined) return cached;
    if (visiting.has(note.id)) return note.modified;
    visiting.add(note.id);
    const value = Math.max(note.modified, ...notes
      .filter((child) => child.canvas.parent === note.id)
      .map((child) => newest(child, visiting)));
    visiting.delete(note.id);
    modified.set(note.id, value);
    return value;
  };
  return children.sort((a, b) => Number(b.canvas.project) - Number(a.canvas.project) || newest(b) - newest(a));
}

function firstLine(markdown: string): string {
  return markdown.split(/\r?\n/).map((line) => line.trim()).find(Boolean) ?? '';
}
