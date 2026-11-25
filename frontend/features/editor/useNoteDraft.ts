import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createAutosave, LIVE_SAVE } from '../../shared/draft/autosave';
import type { Note } from '../../shared/engine_contract';
import { isConflict, TEXT_CONFLICT_MESSAGE } from '../../shared/text_write';
import type { MemoStore } from '../../shared/useMemoivy';

// shortcut: 기준 revision·출발 본문 추적이 shared/draft/useDraft와 겹친다. 편집기 IME·충돌 선택 UI가 useDraft로 옮겨질 때 통합한다.
export function useNoteDraft(store: MemoStore, note: Note | undefined) {
  const [draft, setDraft] = useState(note?.text ?? '');
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [conflict, setConflict] = useState(false);
  const composing = useRef(false);
  const latest = useRef(draft);
  const baseRevision = useRef<number | undefined>(undefined);
  // 초안이 출발한 본문. 충돌 시 원격 본문이 이와 같으면 캔버스 위치 등 본문 외 변경이므로 자동으로 이어 저장한다.
  const baseText = useRef(note?.text ?? '');
  const saveRef = useRef<(value: string) => Promise<boolean>>(async () => true);
  // 저장 시점 규칙은 인스펙터와 공유한다.
  const autosave = useMemo(() => createAutosave<string>(LIVE_SAVE, (value) => void saveRef.current(value)), []);
  latest.current = draft;

  useEffect(() => {
    if (note && !dirty) { setDraft(note.text); baseText.current = note.text; baseRevision.current = store.snapshot?.revision; }
  }, [note?.text, note, dirty, store.snapshot?.revision]);

  const save = useCallback(async (value = latest.current) => {
    if (composing.current) { autosave.schedule(value); return false; }
    if (!note || value === note.text) { setDirty(false); return true; }
    autosave.cancel();
    const sentBase = baseRevision.current ?? store.snapshot?.revision;
    try {
      const result = await store.saveText(note, value, sentBase, baseText.current);
      baseText.current = value;
      baseRevision.current = result.snapshot?.revision ?? store.snapshot?.revision ?? sentBase;
      setDirty(latest.current !== value);
      setError(null);
      setConflict(false);
      return true;
    } catch (reason) {
      setDirty(true);
      const conflict = isConflict(reason);
      setConflict(conflict);
      setError(conflict ? TEXT_CONFLICT_MESSAGE : reason instanceof Error ? reason.message : String(reason));
      return false;
    }
  }, [note, store.saveText, store.snapshot?.revision, autosave]);
  saveRef.current = save;

  const change = useCallback((value: string) => {
    if (!dirty) { baseRevision.current = store.snapshot?.revision; if (note) baseText.current = note.text; }
    setDraft(value);
    setDirty(true);
    setError(null);
    latest.current = value;
    autosave.schedule(value);
  }, [dirty, autosave, store.snapshot?.revision, note]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: 메모가 바뀌거나 화면을 떠날 때만, 떠나는 메모의 클로저로 마지막 초안을 저장한다.
  useEffect(() => () => {
    if (!dirty) return;
    autosave.cancel();
    if (note && latest.current !== note.text) void store.updateNote(note, latest.current).catch(() => undefined);
  }, [dirty, note?.id]);

  useEffect(() => store.bridge.onFlush(async () => {
    while (composing.current) await new Promise((resolve) => window.setTimeout(resolve, 25));
    if (dirty && !(await save())) throw new Error(error ?? '저장에 실패했습니다.');
  }), [store.bridge, dirty, save, error]);

  const updateComposing = useCallback((value: boolean) => {
    composing.current = value;
    if (!value && dirty) void save();
  }, [dirty, save]);

  const overwriteDraft = useCallback(async () => {
    baseRevision.current = store.snapshot?.revision;
    if (note) baseText.current = note.text;
    setConflict(false);
    await save(latest.current);
  }, [save, store.snapshot?.revision, note]);

  const useRemoteVersion = useCallback(() => {
    if (!note) return;
    setDraft(note.text);
    latest.current = note.text;
    baseText.current = note.text;
    baseRevision.current = store.snapshot?.revision;
    setDirty(false);
    setConflict(false);
    setError(null);
  }, [note, store.snapshot?.revision]);

  return { draft, change, dirty, error, setError, conflict, composing, save, updateComposing, overwriteDraft, useRemoteVersion, latest };
}
