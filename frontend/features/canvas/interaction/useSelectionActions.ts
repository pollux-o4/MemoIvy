import type { Note } from '../../../shared/engine_contract';
import type { MemoStore } from '../../../shared/useMemoivy';

/**
 * 선택한 카드에 하는 동작의 단일 출처(Command 묶음). 키보드 단축키·카드 메뉴·드로어가 같은 명령을 부른다.
 * 대상은 항상 "지금 선택"이다. 카드 메뉴는 열 때 대상 카드를 먼저 선택한다.
 */
export function useSelectionActions({ store, selected, selectedNote, openCard, clipboard, toggleDone, requestDelete, requestRename }: {
  store: MemoStore; selected: string[]; selectedNote?: Note;
  openCard: (note: Note) => void;
  clipboard: { copy: () => boolean; paste: () => Promise<void>; duplicate: () => Promise<void> };
  toggleDone: (ids: string[]) => void; requestDelete: (ids: string[]) => void; requestRename: () => void;
}) {
  const ignore = () => undefined;
  const saveText = (note: Note, text: string) => { void store.updateNote(note, text).catch(ignore); };
  const recolor = (ids: string[], color: string) => {
    for (const note of store.notes) if (ids.includes(note.id) && note.color !== color) void store.updateNote(note, undefined, undefined, color).catch(ignore);
  };
  return {
    saveText, recolor,
    open: () => { if (selectedNote) openCard(selectedNote); },
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
