import type { Note } from '../../../shared/engine_contract';
import { titleOf } from '../../../shared/model';

export function canvasTitle(note: Note): string {
  return note.text.trim() ? titleOf(note) : note.canvas.project ? '이름 없는 프로젝트' : '빈 메모';
}
