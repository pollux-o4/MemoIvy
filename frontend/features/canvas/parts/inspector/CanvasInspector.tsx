import { memo, useEffect, useMemo, useRef, type Ref } from 'react';
import './inspector.css';
import type { Note } from '../../../../shared/engine_contract';
import { progressLabel } from '../../../../shared/progress';
import { useProgress } from '../../../../shared/ProgressProvider';
import { InspectorIdentity } from './InspectorIdentity';
import { InspectorLinks } from './InspectorLinks';
import { InspectorOrganize } from './InspectorOrganize';
import { directChildren, outlineParent } from '../../model/links/outline_tree';
import { InspectorOutline } from './InspectorOutline';
import type { OutlineRow } from '../../model/links/types';
import { canvasTitle } from '../../model/labels';
import { InspectorPrimary } from './InspectorPrimary';

/**
 * 선택한 카드 하나의 드로어. 위에서 아래로 "무엇인가(이름·소개) → 지금 어떤가·바로 할 일 → 어디와 이어지나 → 어디에 두나 → 삭제".
 * 자주 쓰는 것일수록 위·크게, 위험한 것은 맨 아래·조용하게 둔다. 각 층은 inspector/ 부품이 그린다.
 */
/** memo: 끌기 중처럼 선택 메모·목록이 그대로면 다시 그리지 않는다(동작은 Canvas가 고정 참조로 넘긴다). */
export const CanvasInspector = memo(function CanvasInspector({ note, notes, projects, renameRequest, revision, onClose, onOpen, onOpenAsNote, onSaveText, onToggleTask, onColor, onLift, onMoveInto, onToggleDone, onSignal, onDelete, onSelect, openLinkPicker, onToggleLink, onAddBranch, onExternal, outline }: {
  note: Note; notes: Note[]; projects: Note[];
  /** 바뀔 때마다 이름 칸에 포커스(F2·메뉴의 이름 바꾸기). */
  renameRequest: number;
  onClose: () => void; onOpen: (note: Note) => void; onOpenAsNote: (note: Note) => void;
  onSaveText: (note: Note, text: string, baseRevision?: number) => Promise<number | undefined>; revision?: number; onColor: (note: Note, color: string) => void;
  onToggleTask: (note: Note, sourceLine: number) => void;
  onLift?: () => void; onMoveInto: (projectId: string) => void;
  onToggleDone: () => void; onSignal: () => void; onDelete: () => void;
  openLinkPicker: Ref<() => void>;
  onSelect: (id: string) => void; onToggleLink: (note: Note, id: string) => void;
  onExternal: (url: string) => void;
  /** 선택 카드 아래 새 가지(캔버스 Tab과 같은 동작). */
  onAddBranch: () => void;
  /** 가지 개요. 가지가 없으면(행 1개 이하) 칸을 숨긴다. */
  outline?: { rows: readonly OutlineRow[]; onIndent: (id: string) => boolean; onOutdent: (id: string) => boolean; onMove: (id: string, target: string, place: 'before' | 'after') => boolean; onShift: (id: string, step: -1 | 1) => boolean };
}) {
  // 개요 제목 찾기용 색인. 목록이 같으면 다시 만들지 않는다(O(n) 한 번).
  const byId = useMemo(() => new Map(notes.map((item) => [item.id, item])), [notes]);
  const titleRef = useRef<HTMLInputElement>(null);
  const project = Boolean(note.canvas.project);
  const progress = useProgress();
  const tasks = progressLabel(note, progress);
  const hasTasks = progress.of(note).total > 0;
  const showOutline = outline !== undefined && outline.rows.length > 1;
  // 개요는 한 뿌리의 나무라 여러 부모 중 하나만 담는다. 나머지 부모는 개요 안 "다른 곳에서도 이어짐"으로 함께 보인다.
  const treeParent = showOutline ? outlineParent(outline.rows, note.id) : undefined;
  const alsoFrom = useMemo(() => showOutline ? notes.filter((item) => item.id !== treeParent && item.canvas.next.includes(note.id)).map((item) => item.id) : [], [showOutline, notes, note.id, treeParent]);
  useEffect(() => { if (renameRequest) { titleRef.current?.focus(); titleRef.current?.select(); } }, [renameRequest]);
  return <aside className="canvas-inspector floating" aria-label={project ? '선택한 프로젝트' : '선택한 메모'}>
    <InspectorIdentity key={note.id} note={note} titleRef={titleRef} revision={revision} onSaveText={(text, base) => onSaveText(note, text, base)} onToggleTask={(line) => onToggleTask(note, line)} onColor={(color) => onColor(note, color)} onClose={onClose} onExternal={onExternal}/>
    <InspectorPrimary note={note} childCount={progress.children(note.id).length} tasks={hasTasks ? tasks : undefined} onAction={{ open: () => onOpen(note), openAsNote: () => onOpenAsNote(note) }} onToggleDone={onToggleDone}/>
    <InspectorLinks note={note} notes={notes} incomingInOutline={showOutline} childrenInOutline={showOutline ? directChildren(outline.rows, note.id) : undefined} onSelect={onSelect} openLinkPicker={openLinkPicker} onToggleLink={onToggleLink} onSignal={onSignal} onAddBranch={onAddBranch}/>
    {showOutline && <InspectorOutline rows={outline.rows} current={note.id} alsoFrom={alsoFrom} onSelect={onSelect} onUnlink={(id) => onToggleLink(note, id)} onIndent={outline.onIndent} onOutdent={outline.onOutdent} onMove={outline.onMove} onShift={outline.onShift}
      labelOf={(id) => { const item = byId.get(id); return item ? canvasTitle(item) : id; }}/>}
    <InspectorOrganize projects={projects} onLift={onLift} onMoveInto={onMoveInto}/>
    <footer><button className="inspector-delete" title="삭제 · Delete" onClick={onDelete}>{project ? '프로젝트 삭제…' : '메모 삭제…'}</button></footer>
  </aside>;
});
