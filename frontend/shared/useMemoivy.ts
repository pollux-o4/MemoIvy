import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { DesktopBridge, EngineEvent, EngineResult, Note, Snapshot } from './engine_contract';
import { createMockBridge } from './mock_bridge';

export function useMemoivy(injectedBridge?: DesktopBridge) {
  const bridge = useMemo(() => injectedBridge ?? window.memoivy ?? createMockBridge(), [injectedBridge]);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const snapshotRef = useRef<Snapshot | null>(null);
  const revisionRef = useRef(-1);
  const queue = useRef<Promise<unknown>>(Promise.resolve());

  useEffect(() => {
    let alive = true;
    const accept = (next: Snapshot) => {
      if (!alive || next.revision < Math.max(revisionRef.current, snapshotRef.current?.revision ?? -1)) return;
      revisionRef.current = next.revision;
      snapshotRef.current = next;
      setSnapshot(next);
    };
    snapshotRef.current = null;
    revisionRef.current = -1;
    setSnapshot(null);
    const unsubscribe = bridge.subscribe((event: EngineEvent) => {
      if (event.snapshot) accept(event.snapshot);
      if (event.error) setError(event.error.message);
    });
    bridge.request('snapshot').then((result) => { if (result.snapshot) accept(result.snapshot); }).catch((reason: unknown) => setError(messageOf(reason)));
    return () => { alive = false; unsubscribe(); };
  }, [bridge]);

  const enqueue = useCallback((op: string, prepare: (current: Snapshot | null) => Record<string, unknown> | undefined) => {
    const task = async (): Promise<EngineResult> => {
      const write = ['create', 'update', 'delete', 'canvas', 'importAsset', 'deleteAsset', 'preference'].includes(op);
      const current = snapshotRef.current;
      setSaving(write);
      try {
        const params = prepare(current);
        if (!params) return {};
        const { baseRevision: explicitBase, ...operationParams } = params;
        const result = await bridge.request(op, { ...operationParams, ...(write && current ? { baseRevision: explicitBase ?? current.revision } : {}) });
        if (result.snapshot && (!snapshotRef.current || result.snapshot.revision >= snapshotRef.current.revision)) {
          revisionRef.current = result.snapshot.revision;
          snapshotRef.current = result.snapshot;
          setSnapshot(result.snapshot);
        }
        if (write) setError(null);
        return result;
      } catch (reason) {
        const conflict = reason as { snapshot?: Snapshot; message?: string };
        if (conflict.snapshot && (!snapshotRef.current || conflict.snapshot.revision >= snapshotRef.current.revision)) {
          revisionRef.current = conflict.snapshot.revision;
          snapshotRef.current = conflict.snapshot;
          setSnapshot(conflict.snapshot);
        }
        setError(messageOf(reason));
        throw reason;
      } finally { if (write) setSaving(false); }
    };
    const pending = queue.current.then(task, task);
    queue.current = pending.then(() => undefined, () => undefined);
    return pending;
  }, [bridge]);

  const request = useCallback((op: string, params: Record<string, unknown> = {}) => enqueue(op, () => params), [enqueue]);
  const editNoteText = useCallback((id: string, apply: (currentText: string) => string | undefined) => enqueue('update', (current) => {
    const note = current?.notes.find((item) => item.id === id);
    if (!note) throw new Error('메모를 찾을 수 없어요.');
    const text = apply(note.text);
    if (text === undefined) throw new Error('메모가 바뀌었어요. 확인하고 다시 체크해 주세요.');
    return text === note.text ? undefined : { id, text };
  }), [enqueue]);

  const createNote = useCallback(async (options: { text?: string; color?: string; project?: boolean; parent?: string | null } = {}) => {
    const result = await request('create', options);
    return result.note;
  }, [request]);
  const updateNote = useCallback(async (note: Note, text?: string, canvas?: Partial<Note['canvas']>, color?: string, baseRevision?: number) => {
    return request('update', { id: note.id, ...(text !== undefined ? { text } : {}), ...(canvas ? { canvas } : {}), ...(color ? { color } : {}), ...(baseRevision !== undefined ? { baseRevision } : {}) });
  }, [request]);
  const mutateCanvas = useCallback(async (changes: Array<{ id: string; canvas: Partial<Note['canvas']> }>) => {
    if (!changes.length) return;
    return request('canvas', { changes });
  }, [request]);
  const removeNote = useCallback(async (id: string) => request('delete', { id }), [request]);

  return { bridge, snapshot, notes: snapshot?.notes ?? [], assets: snapshot?.assets ?? [], error, saving, request, editNoteText, createNote, updateNote, mutateCanvas, removeNote };
}

export type MemoStore = ReturnType<typeof useMemoivy>;
function messageOf(reason: unknown): string {
  const message = reason instanceof Error ? reason.message : String(reason);
  return message === 'This note changed in another window.' ? '다른 창에서 메모가 바뀌었어요.' : message;
}

