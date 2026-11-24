import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { DesktopBridge, EngineEvent, EngineResult, Note, Snapshot } from './engine_contract';
import { createMockBridge } from './mock_bridge';

type RequestQueue = { bridge: DesktopBridge; pending: Promise<unknown>; writes: number };
const WRITE_OPS = new Set(['create', 'update', 'delete', 'canvas', 'restore', 'importAsset', 'deleteAsset', 'preference']);

export function useMemoivy(injectedBridge?: DesktopBridge) {
  const bridge = useMemo(() => injectedBridge ?? window.memoivy ?? createMockBridge(), [injectedBridge]);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const snapshotRef = useRef<Snapshot | null>(null);
  const acceptSnapshot = useCallback((next: Snapshot) => {
    if (next.revision < (snapshotRef.current?.revision ?? -1)) return;
    snapshotRef.current = next;
    setSnapshot(next);
  }, []);
  const queue = useRef<RequestQueue | null>(null);
  if (queue.current?.bridge !== bridge) queue.current = { bridge, pending: Promise.resolve(), writes: 0 };

  useEffect(() => {
    let alive = true;
    const subscribedQueue = queue.current;
    snapshotRef.current = null;
    setSnapshot(null);
    const unsubscribe = bridge.subscribe((event: EngineEvent) => {
      if (!alive) return;
      if (event.snapshot) acceptSnapshot(event.snapshot);
      if (event.error) setError(event.error.message);
    });
    bridge.request('snapshot').then((result) => { if (alive && result.snapshot) acceptSnapshot(result.snapshot); }).catch((reason: unknown) => { if (alive) setError(messageOf(reason)); });
    if (subscribedQueue?.bridge === bridge && subscribedQueue.writes === 0) setSaving(false);
    return () => {
      alive = false;
      unsubscribe();
      if (queue.current === subscribedQueue) queue.current = { bridge, pending: Promise.resolve(), writes: 0 };
    };
  }, [acceptSnapshot, bridge]);

  const enqueue = useCallback((op: string, prepare: (current: Snapshot | null) => Record<string, unknown> | undefined) => {
    const activeQueue = queue.current;
    if (!activeQueue || activeQueue.bridge !== bridge) throw new Error('Memoivy bridge changed before the request was queued.');
    const write = WRITE_OPS.has(op);
    const task = async (): Promise<EngineResult> => {
      const current = snapshotRef.current;
      try {
        if (queue.current !== activeQueue) {
          if (write) throw new Error('메모 저장 중 창이 바뀌어 대기 중인 쓰기를 취소했어요.');
          return {};
        }
        const params = prepare(current);
        if (!params) return {};
        const { baseRevision: explicitBase, ...operationParams } = params;
        const result = await bridge.request(op, { ...operationParams, ...(write && current ? { baseRevision: explicitBase ?? current.revision } : {}) });
        if (queue.current === activeQueue && result.snapshot) acceptSnapshot(result.snapshot);
        if (queue.current === activeQueue && write) setError(null);
        return result;
      } catch (reason) {
        const conflict = reason as { snapshot?: Snapshot; message?: string };
        if (queue.current === activeQueue && conflict.snapshot) acceptSnapshot(conflict.snapshot);
        if (queue.current === activeQueue) setError(messageOf(reason));
        throw reason;
      } finally {
        if (write) {
          activeQueue.writes -= 1;
          if (queue.current === activeQueue) setSaving(activeQueue.writes > 0);
        }
      }
    };
    if (write) { activeQueue.writes += 1; setSaving(true); }
    const pending = activeQueue.pending.then(task, task);
    activeQueue.pending = pending.then(() => undefined, () => undefined);
    return pending;
  }, [acceptSnapshot, bridge]);

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
  // shortcut: 엔진 delete가 한 건씩이라 차례로 지운다(중간 실패 시 앞의 것만 지워짐), 여러 장 삭제가 느려지면 일괄 delete 검토.
  const removeNotes = useCallback(async (ids: readonly string[]) => { for (const id of ids) await removeNote(id); }, [removeNote]);
  const restoreNotes = useCallback(async (notes: Note[], changes: Array<{ id: string; canvas: Partial<Note['canvas']> }>) => request('restore', { notes, changes }), [request]);

  return { bridge, snapshot, notes: snapshot?.notes ?? [], assets: snapshot?.assets ?? [], error, saving, request, editNoteText, createNote, updateNote, mutateCanvas, removeNote, removeNotes, restoreNotes };
}

export type MemoStore = ReturnType<typeof useMemoivy>;
function messageOf(reason: unknown): string {
  const message = reason instanceof Error ? reason.message : String(reason);
  return message === 'This note changed in another window.' ? '다른 창에서 메모가 바뀌었어요.' : message;
}

