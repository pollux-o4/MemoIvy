import type { Note } from '../../engine_contract';
import type { MemoStore } from '../../useMemoivy';
import { rebasePreviewTask } from './model';

export function savePreviewTask(store: Pick<MemoStore, 'editNoteText'>, note: Note, line: number) {
  void store.editNoteText(note.id, (current) => rebasePreviewTask(note.text, current, line)).catch(() => undefined);
}
