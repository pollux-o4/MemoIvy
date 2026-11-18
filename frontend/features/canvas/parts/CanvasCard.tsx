import type { PointerEvent } from 'react';
import type { Note } from '../../../shared/engine_contract';
import { Icon } from '../../../shared/Icon';
import { checklist, descendantsOf, noteColor } from '../../../shared/model';
import { CARD_HEIGHT, CARD_WIDTH, type CanvasPoint } from '../model/align';
import { canvasTitle } from '../model/labels';
import { canvasPreview } from '../model/preview';

export function CanvasCard({ note, allNotes, position, selected, dropTarget, onPointerDown, onOpen, onConnectStart, onToggleDone }: {
  allNotes: Note[]; onToggleDone: () => void; dropTarget?: boolean;
  note: Note; position: CanvasPoint; selected: boolean;
  onPointerDown: (event: PointerEvent, id: string) => void; onOpen: (note: Note) => void;
  onConnectStart: (id: string) => void;
}) {
  const tasks = checklist(note.text);
  const preview = canvasPreview(note.text, 3);
  const rows = preview.rows;
  const title = preview.title.text || canvasTitle(note);
  const descendants = note.canvas.project ? descendantsOf(note.id, allNotes) : [];
  const projectTasks = [...descendants, note].reduce((sum, item) => { const value = checklist(item.text); return { done: sum.done + value.done, total: sum.total + value.total }; }, { done: 0, total: 0 });
  return <article className={`canvas-note${selected ? ' selected' : ''}${dropTarget ? ' drop-target' : ''}${note.canvas.done ? ' done' : ''}${note.canvas.project ? ' project' : ''}`} style={{ left: position.x, top: position.y, '--card-tone': noteColor(note.color).tone } as React.CSSProperties}
    data-note-id={note.id} onPointerDown={(event) => onPointerDown(event, note.id)} onDoubleClick={() => onOpen(note)} tabIndex={0} aria-label={`${title} 카드`}>
    <header>
      {preview.title.kind === 'task' && <span className="canvas-title-marker" aria-hidden="true">{preview.title.done ? '☑' : '☐'}</span>}
      {preview.title.kind === 'number' && <span className="canvas-title-marker" aria-hidden="true">{preview.title.mark}</span>}
      {preview.title.kind === 'bullet' && <span className="canvas-title-marker" aria-hidden="true">•</span>}
      <h3 className={preview.title.kind === 'task' && preview.title.done ? 'line-done' : ''}>{title}</h3>
      {note.canvas.project && <button className="card-enter" title="프로젝트 열기 · 두 번 누르기 또는 Enter" aria-label={`${title} 프로젝트 들어가기`} onPointerDown={(event) => event.stopPropagation()} onClick={() => onOpen(note)}><Icon name="arrow" size={16}/></button>}
      <button className="card-done" title={note.canvas.done ? '완료 취소' : '완료로 표시'} aria-label={note.canvas.done ? '완료 취소' : '완료로 표시'} onPointerDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); onToggleDone(); }}><Icon name="check" size={14}/></button>
    </header>
    {rows.length ? <ul className="canvas-checklist-preview">
      {rows.map((row, index) => <li className={`preview-${row.kind}`} key={index}>
        {row.kind === 'task' ? <span className={row.done ? 'checked' : ''} aria-hidden="true">{row.done ? <Icon name="check" size={12}/> : null}</span>
          : row.kind === 'number' ? <span className="preview-marker">{row.mark}</span>
            : row.kind === 'bullet' ? <span className="preview-marker">•</span> : <span className="preview-spacer"/>}
        <small className={row.done ? 'checked' : ''}>{row.text}</small>
      </li>)}
    </ul> : !note.text.trim() && <p>내용을 적어 보세요.</p>}
    {note.canvas.project && <div className="canvas-project-meta"><small><Icon name="project" size={14}/><b>메모 {descendants.length}</b></small>{projectTasks.total > 0 && <small>할 일 {projectTasks.done}/{projectTasks.total}</small>}</div>}
    {!note.canvas.project && <div className="canvas-card-meta">{tasks.total > 0 && <small>할 일 {tasks.done}/{tasks.total}</small>}{preview.remaining > 0 && <small>외 {preview.remaining}줄</small>}</div>}
    {note.canvas.project && projectTasks.total > 0 && <div className="canvas-project-progress"><span style={{ width: `${100 * projectTasks.done / projectTasks.total}%` }}/></div>}
    {note.canvas.next.length > 0 && <footer>
      <button title="끌어서 다음 메모에 연결" aria-label={`${title} 다음 메모 연결`}
        onPointerDown={(event) => { event.stopPropagation(); onConnectStart(note.id); }}
      >
        <Icon name="link" size={14}/>
      </button>
      <span>{note.canvas.next.length}</span>
    </footer>}
  </article>;
}
