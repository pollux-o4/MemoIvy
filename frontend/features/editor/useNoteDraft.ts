import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createAutosave, LIVE_SAVE } from '../../shared/draft/autosave';
import type { Note } from '../../shared/engine_contract';
import type { MemoStore } from '../../shared/useMemoivy';

export function useNoteDraft(store: MemoStore, note: Note | undefined) {
  const [draft, setDraft] = useState(note?.text ?? '');
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [conflict, setConflict] = useState(false);
  const composing = useRef(false);
  const latest = useRef(draft);
  const baseRevision = useRef<number | undefined>(undefined);
  const saveRef = useRef<(value: string) => Promise<boolean>>(async () => true);
  // 저장 시점 규칙은 인스펙터와 공유한다.
  const autosave = useMemo(() => createAutosave<string>(LIVE_SAVE, (value) => void saveRef.current(value)), []);
  latest.current = draft;

  useEffect(() => {
    if (note && !dirty) { setDraft(note.text); baseRevision.current = store.snapshot?.revision; }
  }, [note?.text, note, dirty, store.snapshot?.revision]);

  const save = useCallback(async (value = latest.current) => {
    if (composing.current) { autosave.schedule(value); return false; }
    if (!note || value === note.text) { setDirty(false); return true; }
    autosave.cancel();
    const sentBase = baseRevision.current ?? store.snapshot?.revision;
    try {
      const result = await store.updateNote(note, value, undefined, undefined, sentBase);
      baseRevision.current = result.snapshot?.revision ?? store.snapshot?.revision ?? sentBase;
      setDirty(latest.current !== value);
      setError(null);
      setConflict(false);
      return true;
    } catch (reason) {
      setDirty(true);
      const failure = reason as { code?: string; message?: string };
      setConflict(failure.code === 'CONFLICT');
      setError(failure.message ?? (reason instanceof Error ? reason.message : String(reason)));
      return false;
    }
  }, [note, store.updateNote, store.snapshot?.revision, autosave]);
  saveRef.current = save;

  const change = useCallback((value: string) => {
    if (!dirty) baseRevision.current = store.snapshot?.revision;
    setDraft(value);
    setDirty(true);
    setError(null);
    latest.current = value;
    autosave.schedule(value);
  }, [dirty, autosave, store.snapshot?.revision]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: 메모가 바뀌거나 화면을 떠날 때만, 떠나는 메모의 클로저로 마지막 초안을 저장한다.
  useEffect(() => () => {
    autosave.cancel();
    if (dirty && note && latest.current !== note.text) void store.updateNote(note, latest.current).catch(() => undefined);
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
    setConflict(false);
    await save(latest.current);
  }, [save, store.snapshot?.revision]);

  const useRemoteVersion = useCallback(() => {
    if (!note) return;
    setDraft(note.text);
    latest.current = note.text;
    baseRevision.current = store.snapshot?.revision;
    setDirty(false);
    setConflict(false);
    setError(null);
  }, [note, store.snapshot?.revision]);

  return { draft, change, dirty, error, setError, conflict, composing, save, updateComposing, overwriteDraft, useRemoteVersion, latest };
}
