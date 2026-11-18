import type { Note } from '../../../../shared/engine_contract';
import { Icon } from '../../../../shared/Icon';

/** 2층 · 상태와 주 동작: 한눈에 보는 상태 한 줄 + 가장 흔한 동작 하나(들어가기·열기). 완료는 상태 옆 토글. */
export function InspectorPrimary({ note, childCount, tasks, onOpen, onToggleDone }: {
  note: Note; childCount: number; tasks: { done: number; total: number };
  onOpen: () => void; onToggleDone: () => void;
}) {
  const project = Boolean(note.canvas.project);
  const facts = [project && `메모 ${childCount}개`, tasks.total > 0 && `할 일 ${tasks.done}/${tasks.total}`].filter(Boolean).join(' · ');
  return <section className="inspector-primary">
    <div className="inspector-status">
      <span>{facts || (project ? '빈 프로젝트' : '')}</span>
      <button className="inspector-done" aria-pressed={note.canvas.done} title="완료 상태 바꾸기 · Ctrl+Enter" onClick={onToggleDone}>
        <Icon name="check" size={13}/>{note.canvas.done ? '완료됨' : '완료'}
      </button>
    </div>
    <button className="inspector-main" title={project ? '프로젝트 들어가기 · 두 번 누르기 또는 Enter' : '메모 열기 · 두 번 누르기 또는 Enter'} onClick={onOpen}>
      <Icon name={project ? 'enter' : 'note'} size={16}/><span>{project ? '프로젝트 들어가기' : '메모 열기'}</span><kbd>Enter</kbd>
    </button>
  </section>;
}
