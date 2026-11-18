import { type CSSProperties, type PointerEvent, useMemo } from 'react';
import type { Note } from '../../../shared/engine_contract';
import { descendantsOf, noteColor, noteTasks } from '../../../shared/model';
import type { CanvasPoint } from '../model/align';
import { canvasTitle } from '../model/labels';
import { canvasPreview } from '../model/preview';
import { projectFan } from '../model/project_fan';
import { CardHeader } from './card/CardHeader';
import { CardPreview } from './card/CardPreview';
import { LinkHandle } from './card/LinkHandle';
import { ProjectFan } from './card/ProjectFan';
import { ProjectSummary } from './card/ProjectSummary';

const NO_NOTES: Note[] = [];

/** 캔버스 카드 조립: 상태 클래스·포인터 연결만 맡고 내용은 card/ 부품이 그린다. 프로젝트면 뒤에 부채를 함께 둔다. */
export function CanvasCard({ note, allNotes, position, selected, dropTarget, fanQuiet = false, onPointerLeave, onPointerDown, onOpen, onConnectStart, onToggleDone, onContextMenu }: {
  allNotes: Note[]; onToggleDone: () => void; dropTarget?: boolean; fanQuiet?: boolean; onPointerLeave?: (id: string) => void;
  note: Note; position: CanvasPoint; selected: boolean;
  onPointerDown: (event: PointerEvent, id: string) => void; onOpen: (note: Note) => void;
  onConnectStart: (id: string) => void; onContextMenu?: (event: React.MouseEvent, id: string) => void;
}) {
  const project = Boolean(note.canvas.project);
  const preview = useMemo(() => canvasPreview(note.text, 3), [note.text]);
  const descendants = useMemo(() => project ? descendantsOf(note.id, allNotes) : NO_NOTES, [project, note.id, allNotes]);
  const title = preview.title.text || canvasTitle(note);
  const tone = noteColor(note.color).tone;
  const tasks = noteTasks(note, descendants);
  const state = `${selected ? ' selected' : ''}${dropTarget ? ' drop-target' : ''}${note.canvas.done ? ' done' : ''}${project ? ' project' : ''}${fanQuiet ? ' fan-quiet' : ''}`;
  return <>
    <article className={`canvas-note${state}`} style={{ left: position.x, top: position.y, '--card-tone': tone } as CSSProperties}
      data-note-id={note.id} onPointerDown={(event) => onPointerDown(event, note.id)} onPointerLeave={onPointerLeave && (() => onPointerLeave(note.id))} onDoubleClick={() => onOpen(note)} onContextMenu={onContextMenu && ((event) => onContextMenu(event, note.id))} tabIndex={0} aria-label={`${title} 카드`}>
      <CardHeader title={title} marker={preview.title} project={project} done={note.canvas.done} onEnter={() => onOpen(note)} onToggleDone={onToggleDone}/>
      <CardPreview rows={preview.rows} empty={!note.text.trim()}/>
      {project ? <ProjectSummary label={title} count={descendants.length} tasks={tasks}/>
        : <div className="canvas-card-meta">{tasks.total > 0 && <small>할 일 {tasks.done}/{tasks.total}</small>}{preview.remaining > 0 && <small>외 {preview.remaining}줄</small>}</div>}
      {note.canvas.next.length > 0 && <LinkHandle title={title} count={note.canvas.next.length} onConnectStart={() => onConnectStart(note.id)}/>}
    </article>
    {project && <ProjectFan fan={projectFan(descendants)} position={position} tone={tone}/>}
  </>;
}
