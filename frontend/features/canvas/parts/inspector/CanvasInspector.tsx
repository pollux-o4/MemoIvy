import type { Note } from '../../../shared/engine_contract';
import { Icon } from '../../../shared/Icon';
import { checklist, descendantsOf } from '../../../shared/model';
import { canvasTitle } from '../model/labels';
import { canvasPreview } from '../model/preview';
import { CanvasProjectMenu } from './CanvasProjectMenu';

export function CanvasInspector({ note, notes, allNotes, projects, onClose, onOpen, onLift, onMoveInto, onToggleDone, onSignal, onDelete, onSelect, onDisconnect }: {
  note: Note; notes: Note[]; allNotes: Note[]; onClose: () => void; onOpen: (note: Note) => void;
  projects: Note[]; onLift?: () => void; onMoveInto: (projectId: string) => void;
  onToggleDone: () => void; onSignal: () => void; onDelete: () => void;
  onSelect: (id: string) => void; onDisconnect: (note: Note, id: string) => void;
}) {
  const rows = canvasPreview(note.text, 5).rows;
  const tasks = checklist(note.text);
  const incoming = notes.filter((item) => item.canvas.next.includes(note.id));
  const descendants = descendantsOf(note.id, allNotes);
  const actions = [
    ['메모 열기', () => onOpen(note)],
    [note.canvas.done ? '완료 취소' : '완료로 표시', onToggleDone],
    ['신호 보내기', onSignal],
  ] as const;
  return <aside className="canvas-inspector floating" aria-label="선택한 메모">
    <header><span className={`inspector-dot ${note.color}`}/><h2>{canvasTitle(note)}</h2>
      <button className="inspector-close" aria-label="선택 해제" title="선택 해제 · Esc" onClick={onClose}>×</button></header>
    {rows.length > 0 && <div className="inspector-preview">{rows.map((row, index) => <div className={`inspector-preview-row preview-${row.kind}`} key={index}>
      <span className={row.done ? 'checked' : ''}>{row.kind === 'task' && row.done ? <Icon name="check" size={12}/> : row.kind === 'number' ? row.mark : row.kind === 'bullet' ? '•' : null}</span><small className={row.done ? 'checked' : ''}>{row.text}</small>
    </div>)}</div>}
    {tasks.total > 0 && <small className="inspector-task-count">할 일 {tasks.done}/{tasks.total}</small>}
    <div className="inspector-actions">{actions.map(([label, action]) => <button className="inspector-action" key={label} title={label.includes('완료') ? '완료 상태 변경 · Ctrl+Enter' : label === '신호 보내기' ? '연결된 다음 메모로 신호를 보냅니다. 완료 상태는 바꾸지 않습니다.' : undefined} onClick={action}>{label}</button>)}</div>
    {(note.canvas.project || projects.length > 0 || onLift) && <div className="inspector-section"><b>프로젝트</b>
      {note.canvas.project && <button className="inspector-action" title={`안의 메모 ${descendants.length}개 보기 · Enter`} onClick={() => onOpen(note)}>프로젝트 들어가기</button>}
      {onLift && <button className="inspector-action" title="한 단계 위 보드로 옮기기 · Alt+Shift+G" onClick={onLift}>밖으로 꺼내기</button>}
      <CanvasProjectMenu projects={projects} onMove={onMoveInto}/>
    </div>}
    <div className="inspector-section"><b>다음 메모</b>
      {note.canvas.next.map((id) => { const target = notes.find((item) => item.id === id); return target && <div className="inspector-link" key={id}>
        <button className="inspector-link-title" title={canvasTitle(target)} onClick={() => onSelect(target.id)}>{canvasTitle(target)}</button><button className="inspector-disconnect" aria-label={`${canvasTitle(target)} 연결 해제`} title="이 연결 해제 · 메모는 유지" onClick={() => onDisconnect(note, id)}>×</button>
      </div>; })}
      <label className="inspector-add-link">연결 추가<select aria-label="다음 메모 연결 추가" value="" onChange={(event) => event.target.value && onDisconnect(note, event.target.value)}><option value="" disabled>메모 선택</option>{notes.filter((item) => item.id !== note.id && !note.canvas.next.includes(item.id)).map((item) => <option key={item.id} value={item.id}>{canvasTitle(item)}</option>)}</select></label>
    </div>
    {incoming.length > 0 && <div className="inspector-section"><b>이전 메모</b>{incoming.map((item) => <button className="inspector-action" key={item.id} onClick={() => onSelect(item.id)}>{canvasTitle(item)}</button>)}</div>}
    <button className="inspector-delete" title="이 메모 삭제 · Delete" onClick={onDelete}>삭제…</button>
  </aside>;
}
