import { useMemo } from 'react';
import type { Note } from '../../../../shared/engine_contract';
import { branchRoot, branchSet, buildGraph, outline, scopeEdges } from '../../model/links/graph';
import { indentRow, moveRow, type NextChange, outdentRow, shiftRow } from '../../model/links/outline_edit';

/**
 * 가지 초점과 개요. 인접 그래프는 메모·단계가 바뀔 때만 한 번 만든다.
 * - focus: 선택 카드 하나의 가지. 그 밖은 흐리게 둔다.
 * - outlineRows: 선택 카드 가지 뿌리부터의 개요. 프로젝트는 접힌 항목.
 * - indent/outdent: 개요 편집을 next 변경으로 커밋한다. 바뀐 게 없으면 false.
 */
export function useBranchFocus({ allNotes, scope, noteById, selected, commit }: {
  allNotes: Note[]; scope: string | null; noteById: ReadonlyMap<string, Note>; selected: string[];
  commit: (changes: { id: string; canvas: { next: string[] } }[]) => void;
}) {
  const graph = useMemo(() => buildGraph(scopeEdges(allNotes, scope)), [allNotes, scope]);
  const focus = useMemo(() => {
    if (selected.length !== 1) return undefined;
    const branch = branchSet(graph, selected[0]);
    return branch.size > 1 ? branch : undefined;
  }, [graph, selected]);
  const outlineRows = useMemo(() => selected.length === 1 && noteById.has(selected[0])
    ? outline(graph, branchRoot(graph, selected[0]), (id) => Boolean(noteById.get(id)?.canvas.project)) : [], [graph, selected, noteById]);
  const nextOf = (item: string) => noteById.get(item)?.canvas.next ?? [];
  const apply = (changes: NextChange[]) => {
    if (!changes.length) return false;
    commit(changes.map(({ id: owner, next }) => ({ id: owner, canvas: { next } })));
    return true;
  };
  return {
    graph, focus, outlineRows,
    indent: (id: string) => apply(indentRow(outlineRows, id, nextOf)),
    outdent: (id: string) => apply(outdentRow(outlineRows, id, nextOf)),
    move: (id: string, target: string, place: 'before' | 'after') => apply(moveRow(outlineRows, id, target, place, nextOf)),
    shift: (id: string, step: -1 | 1) => apply(shiftRow(outlineRows, id, step, nextOf)),
  };
}
