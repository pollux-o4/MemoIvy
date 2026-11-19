import type { Note } from '../../../../shared/engine_contract';
import type { MemoStore } from '../../../../shared/useMemoivy';
import { savePreviewTask } from '../../../../shared/markdown/preview/savePreviewTask';

/**
 * 선택한 카드에 하는 동작의 단일 출처(Command 묶음). 키보드 단축키·카드 메뉴·드로어가 같은 명령을 부른다.
 * 대상은 항상 "지금 선택"이다. 카드 메뉴는 열 때 대상 카드를 먼저 선택한다.
 */
export function useSelectionActions({ store, selected, selectedNote, openCard, clipboard, toggleDone, requestDelete, requestRename }: {
  store: MemoStore; selected: string[]; selectedNote?: Note;
  openCard: (note: Note, asNote?: boolean) => void;
  clipboard: { copy: () => boolean; paste: () => Promise<void>; duplicate: () => Promise<void> };
  toggleDone: (ids: string[]) => void; requestDelete: (ids: string[]) => void; requestRename: () => void;
}) {
  const ignore = () => undefined;
  /** 기준 revision과 함께 저장하고 새 revision을 돌려준다. 충돌·실패는 호출자(초안)가 처리하도록 reject를 그대로 넘긴다. */
  const saveText = async (note: Note, text: string, baseRevision?: number) => (await store.updateNote(note, text, undefined, undefined, baseRevision)).snapshot?.revision;
  const recolor = (ids: string[], color: string) => {
    for (const note of store.notes) if (ids.includes(note.id) && note.color !== color) void store.updateNote(note, undefined, undefined, color).catch(ignore);
  };
  return {
    saveText, recolor,
    toggleTask: (note: Note, line: number) => savePreviewTask(store, note, line),
    open: () => { if (selectedNote) openCard(selectedNote); },
    /** 프로젝트도 들어가지 않고 메모 편집기로 연다(목록형 제목 클릭과 같은 경로). */
    openAsNote: () => { if (selectedNote) openCard(selectedNote, true); },
    rename: () => { if (selectedNote) requestRename(); },
    color: (color: string) => recolor(selected, color),
    toggleDone: () => toggleDone(selected),
    copy: () => { clipboard.copy(); },
    cut: () => { if (clipboard.copy()) requestDelete(selected); },
    paste: () => { void clipboard.paste().catch(ignore); },
    duplicate: () => { void clipboard.duplicate().catch(ignore); },
    remove: () => { if (selected.length) requestDelete(selected); },
  };
}
