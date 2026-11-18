import { useEffect, useMemo, useRef } from 'react';
import './inspector.css';
import type { Note } from '../../../../shared/engine_contract';
import { descendantsOf, noteTasks } from '../../../../shared/model';
import { InspectorIdentity } from './InspectorIdentity';
import { InspectorLinks } from './InspectorLinks';
import { InspectorOrganize } from './InspectorOrganize';
import { InspectorPrimary } from './InspectorPrimary';

/**
 * 선택한 카드 하나의 드로어. 위에서 아래로 "무엇인가(이름·소개) → 지금 어떤가·바로 할 일 → 어디와 이어지나 → 어디에 두나 → 삭제".
 * 자주 쓰는 것일수록 위·크게, 위험한 것은 맨 아래·조용하게 둔다. 각 층은 inspector/ 부품이 그린다.
 */
export function CanvasInspector({ note, notes, allNotes, projects, renameRequest, revision, onClose, onOpen, onOpenAsNote, onSaveText, onColor, onLift, onMoveInto, onToggleDone, onSignal, onDelete, onSelect, onToggleLink, onExternal }: {
  note: Note; notes: Note[]; allNotes: Note[]; projects: Note[];
  /** 바뀔 때마다 이름 칸에 포커스(F2·메뉴의 이름 바꾸기). */
  renameRequest: number;
  onClose: () => void; onOpen: (note: Note) => void; onOpenAsNote: (note: Note) => void;
  onSaveText: (note: Note, text: string, baseRevision?: number) => Promise<number | undefined>; revision?: number; onColor: (note: Note, color: string) => void;
  onLift?: () => void; onMoveInto: (projectId: string) => void;
  onToggleDone: () => void; onSignal: () => void; onDelete: () => void;
  onSelect: (id: string) => void; onToggleLink: (note: Note, id: string) => void;
  onExternal: (url: string) => void;
}) {
  const titleRef = useRef<HTMLInputElement>(null);
  const project = Boolean(note.canvas.project);
  const descendants = useMemo(() => project ? descendantsOf(note.id, allNotes) : [], [project, note.id, allNotes]);
  const tasks = noteTasks(note, descendants);
  useEffect(() => { if (renameRequest) { titleRef.current?.focus(); titleRef.current?.select(); } }, [renameRequest]);
  return <aside className="canvas-inspector floating" aria-label={project ? '선택한 프로젝트' : '선택한 메모'}>
    <InspectorIdentity key={note.id} note={note} titleRef={titleRef} revision={revision} onSaveText={(text, base) => onSaveText(note, text, base)} onColor={(color) => onColor(note, color)} onClose={onClose} onExternal={onExternal} onOpenAsNote={() => onOpenAsNote(note)}/>
    <InspectorPrimary note={note} childCount={descendants.length} tasks={tasks} onOpen={() => onOpen(note)} onToggleDone={onToggleDone}/>
    <InspectorLinks note={note} notes={notes} onSelect={onSelect} onToggleLink={onToggleLink} onSignal={onSignal}/>
    <InspectorOrganize projects={projects} onLift={onLift} onMoveInto={onMoveInto}/>
    <footer><button className="inspector-delete" title="삭제 · Delete" onClick={onDelete}>{project ? '프로젝트 삭제…' : '메모 삭제…'}</button></footer>
  </aside>;
}
