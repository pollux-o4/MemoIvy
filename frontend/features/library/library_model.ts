import type { Note } from '../../shared/engine_contract';
import { createChildIndex } from '../../shared/hierarchy';
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
  if (!normalized) return [...notes].sort((a, b) => b.modified - a.modified);
  return notes
    .filter((note) => note.text.toLowerCase().includes(normalized))
    .map((note) => ({ note, titleMatch: firstLine(note.text).toLowerCase().includes(normalized) }))
    .sort((a, b) => Number(b.titleMatch) - Number(a.titleMatch) || b.note.modified - a.note.modified)
    .map(({ note }) => note);
}

const trees = new WeakMap<Note[], (parent: string | null) => Note[]>();

export function orderedLibraryChildren(parent: string | null, notes: Note[]): Note[] {
  let order = trees.get(notes);
  if (!order) {
    order = createLibraryOrder(notes);
    trees.set(notes, order);
  }
  return order(parent);
}

function createLibraryOrder(notes: Note[]) {
  const roots = notes.filter((note) => (note.canvas.parent ?? null) === null);
  const children = createChildIndex(notes);
  const modified = new Map<string, number>();
  const visiting = new Set<string>();
  const ordered = new Map<string | null, Note[]>();
  const newest = (note: Note): number => {
    const cached = modified.get(note.id);
    if (cached !== undefined) return cached;
    if (visiting.has(note.id)) return note.modified;
    visiting.add(note.id);
    let value = note.modified;
    for (const child of children(note.id)) value = Math.max(value, newest(child));
    visiting.delete(note.id);
    modified.set(note.id, value);
    return value;
  };
  return (parent: string | null) => {
    const cached = ordered.get(parent);
    if (cached) return cached;
    const siblings = [...(parent === null ? roots : children(parent))];
    siblings.sort((a, b) => Number(b.canvas.project === true) - Number(a.canvas.project === true) || newest(b) - newest(a));
    ordered.set(parent, siblings);
    return siblings;
  };
}

function firstLine(markdown: string): string {
  return markdown.split(/\r?\n/).map((line) => line.trim()).find(Boolean) ?? '';
}
