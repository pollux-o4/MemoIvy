import { useMemo, useState } from 'react';
import type { Note } from '../../../shared/engine_contract';
import { ancestry, scopeDepth, scopeShortcuts } from '../model/scope_model';

/** 캔버스가 보고 있는 프로젝트 범위: 경로·깊이·바로가기와 들어가기/나가기. */
export function useCanvasScope(notes: Note[], initial: string | undefined, select: (ids: string[]) => void, onLeaveRoot: () => void) {
  const [activeScope, setActiveScope] = useState<string | null>(initial ?? null);
  const [recent, setRecent] = useState<string[]>([]);
  const path = useMemo(() => ancestry(activeScope, notes), [activeScope, notes]);
  const scopedProject = path.at(-1);
  const depth = useMemo(() => scopeDepth(activeScope, notes), [activeScope, notes]);
  const shortcuts = useMemo(() => {
    const byId = new Map(notes.map((note) => [note.id, note]));
    return scopeShortcuts(new Set(path.map((item) => item.id)), recent, notes).flatMap((id) => byId.get(id) ?? []);
  }, [path, recent, notes]);
  const navigate = (id: string | null) => {
    select([]);
    setActiveScope(id);
    if (id) setRecent((items) => [id, ...items.filter((item) => item !== id)].slice(0, 5));
  };
  const leave = () => {
    if (!scopedProject) { onLeaveRoot(); return; }
    setActiveScope(scopedProject.canvas.parent ?? null);
    select([scopedProject.id]);
  };
  return { activeScope, scopedProject, path, depth, shortcuts, navigate, leave };
}
