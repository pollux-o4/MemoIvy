import type { DesktopBridge, EngineEvent, EngineResult, Note, Snapshot } from './engine_contract';
import { emptyCanvas } from './model';
import visualFixture from '../../fixtures/visual-notebook.json';

export const fixtureNoteIds = {
  ideas: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1',
  connect: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2',
  project: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa3',
  daily: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa4',
} as const;
const sampleNotes = visualFixture as unknown as Note[];

export function createMockBridge(initial?: Snapshot): DesktopBridge {
  let snapshot: Snapshot = initial ?? { notes: structuredClone(sampleNotes), assets: [], revision: 1, savedRevision: 1, preferences: { expandedProjects: [], expandedNotes: [], navigation: {}, autoPrune: false } };
  const listeners = new Set<(event: EngineEvent) => void>();
  const assetData = new Map<string, { data: string; mime: string }>();
  const publish = () => { for (const listener of listeners) listener({ version: 1, event: 'snapshot', snapshot: structuredClone(snapshot) }); };
  const update = (mutate: () => EngineResult): EngineResult => {
    const result = mutate();
    snapshot = { ...snapshot, revision: snapshot.revision + 1, savedRevision: snapshot.revision + 1 };
    result.snapshot = structuredClone(snapshot);
    publish();
    return result;
  };
  const bridge: DesktopBridge = {
    async request(op, params = {}) {
      if (op === 'snapshot' || op === 'flush') return { snapshot: structuredClone(snapshot) };
      if (['create', 'update', 'delete', 'canvas', 'importAsset', 'deleteAsset', 'preference'].includes(op) && params.baseRevision !== snapshot.revision) {
        const conflict = new Error('다른 창에서 메모가 변경되었습니다. 초안을 확인하고 다시 시도해 주세요.') as Error & { code: string; snapshot: Snapshot };
        conflict.code = 'CONFLICT'; conflict.snapshot = structuredClone(snapshot); throw conflict;
      }
      if (op === 'create') {
        return update(() => {
          const note: Note = {
            id: crypto.randomUUID(),
            text: String(params.text ?? ''),
            color: String(params.color ?? 'yellow'),
            modified: Math.floor(Date.now() / 1000),
            canvas: { ...emptyCanvas(), parent: (params.parent as string | null | undefined) ?? null, project: Boolean(params.project), position: (params.position as [number, number] | null | undefined) ?? null },
          };
          snapshot = { ...snapshot, notes: [note, ...snapshot.notes] };
          return { note };
        });
      }
      if (op === 'update') {
        return update(() => {
          const id = String(params.id);
          const note = snapshot.notes.find((item) => item.id === id);
          if (!note) throw new Error('메모를 찾을 수 없습니다.');
          const next = { ...note, ...(typeof params.text === 'string' ? { text: params.text } : {}), ...(typeof params.color === 'string' ? { color: params.color } : {}), ...(params.canvas ? { canvas: { ...note.canvas, ...(params.canvas as object) } } : {}), modified: Math.floor(Date.now() / 1000) };
          snapshot = { ...snapshot, notes: snapshot.notes.map((item) => item.id === id ? next : item) };
          return { note: next };
        });
      }
      if (op === 'delete') {
        return update(() => {
          const id = String(params.id);
          snapshot = { ...snapshot, notes: snapshot.notes.filter((note) => note.id !== id).map((note) => ({ ...note, canvas: { ...note.canvas, parent: note.canvas.parent === id ? null : note.canvas.parent, next: note.canvas.next.filter((target) => target !== id) } })) };
          return {};
        });
      }
      if (op === 'canvas') {
        return update(() => {
          const patches = params.changes as Array<{ id: string; canvas: Partial<Note['canvas']> }>;
          snapshot = { ...snapshot, notes: snapshot.notes.map((note) => {
            const patch = patches.find((change) => change.id === note.id);
            return patch ? { ...note, canvas: { ...note.canvas, ...patch.canvas } } : note;
          }) };
          return {};
        });
      }
      if (op === 'importAsset') {
        return update(() => {
          const data = String(params.data ?? '');
          const id = crypto.randomUUID().replaceAll('-', '').padEnd(64, '0').slice(0, 64);
          const asset = { id, name: String(params.name ?? '첨부 파일'), mime: String(params.mime ?? 'application/octet-stream'), bytes: Math.floor(data.length * 0.75), usedBy: [], added: Math.floor(Date.now() / 1000) };
          snapshot = { ...snapshot, assets: [...snapshot.assets, asset] };
          assetData.set(id, { data, mime: asset.mime });
          return { asset };
        });
      }
      if (op === 'preference') return update(() => { snapshot = { ...snapshot, preferences: { ...(snapshot.preferences ?? {}), [String(params.key)]: params.value } }; return {}; });
      if (op === 'readAsset') return { ...(assetData.get(String(params.id)) ?? { data: 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a4qkAAAAASUVORK5CYII=', mime: 'image/png' }) };
      if (op === 'deleteAsset') return update(() => { snapshot = { ...snapshot, assets: snapshot.assets.filter((asset) => asset.id !== params.id) }; return {}; });
      throw new Error(`지원하지 않는 mock 작업: ${op}`);
    },
    subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    async openNote(id) { window.dispatchEvent(new CustomEvent('memoivy:preview-window', { detail: { kind: 'note', id } })); },
    async openWindow(kind, id) { window.dispatchEvent(new CustomEvent('memoivy:preview-window', { detail: { kind, id } })); },
    async openExternal(url) { console.info('[mock] open external', url); },
    async importLegacy() { return null; },
    windowAction(action) { console.info('[mock] window action', action); },
    onFlush() { return () => undefined; },
    onWindowState(listener) { listener({ maximized: false, rounded: true }); return () => undefined; },
  };
  return bridge;
}
