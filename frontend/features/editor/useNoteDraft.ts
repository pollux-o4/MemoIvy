import { useCallback, useEffect, useRef, useState } from 'react';
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
  const timer = useRef<number | undefined>(undefined);
  const started = useRef<number | undefined>(undefined);
  latest.current = draft;

  useEffect(() => {
    if (note && !dirty) { setDraft(note.text); baseRevision.current = store.snapshot?.revision; }
  }, [note?.text, note, dirty, store.snapshot?.revision]);

  const save = useCallback(async (value = latest.current) => {
    if (composing.current) { timer.current = window.setTimeout(() => void save(value), 100); return false; }
    if (!note || value === note.text) { setDirty(false); return true; }
    window.clearTimeout(timer.current);
    window.clearTimeout(started.current);
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
  }, [note, store.updateNote, store.snapshot?.revision]);

  const change = useCallback((value: string) => {
    if (!dirty) baseRevision.current = store.snapshot?.revision;
    setDraft(value);
    setDirty(true);
    setError(null);
    latest.current = value;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void save(value), 300);
    if (started.current === undefined) started.current = window.setTimeout(() => void save(latest.current), 1000);
  }, [dirty, save, store.snapshot?.revision]);

  useEffect(() => () => {
    window.clearTimeout(timer.current);
    window.clearTimeout(started.current);
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
