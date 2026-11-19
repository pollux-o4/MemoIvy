import type { Note } from '../../engine_contract';
import type { MemoStore } from '../../useMemoivy';
import { togglePreviewTask } from './model';

export function savePreviewTask(store: Pick<MemoStore, 'snapshot' | 'updateNote'>, note: Note, line: number) {
  const text = togglePreviewTask(note.text, line);
  // 화면에 표시한 본문의 revision으로 충돌을 확인한다. 실패는 저장소가 화면에 알린다.
  if (text !== note.text) void store.updateNote(note, text, undefined, undefined, store.snapshot?.revision).catch(() => undefined);
}
