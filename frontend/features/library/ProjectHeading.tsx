import { Icon } from '../../shared/Icon';
import type { MemoStore } from '../../shared/useMemoivy';
import { NoteMenu } from './NoteMenu';
import { type TaskCount, TaskProgress } from '../../shared/TaskProgress';

export function ProjectHeading({ id, label, childCount, tasks, store, onOpenNote, onOpenCanvas }: { id: string; label: string; childCount: number; tasks: TaskCount; store: MemoStore; onOpenNote: (id: string) => void; onOpenCanvas: (id?: string) => void }) {
  const hasTasks = tasks.total > 0;
  return <div className="project-heading">
    <button className="project-title" onClick={() => onOpenNote(id)}>
      <strong>
        <span className="project-folder" aria-hidden="true"><Icon name="project" size={16}/></span>
        <span className="project-name">{label}</span>
        {hasTasks && <span className={`task-chip${tasks.done === tasks.total ? ' complete' : ''}`} title={`할 일 ${tasks.total}개 중 ${tasks.done}개 완료`}>{tasks.done}/{tasks.total}</span>}
      </strong>
      <span>메모 {childCount}개</span>
      {hasTasks && <TaskProgress className="project-progress" label={label} tasks={tasks}/>}
    </button>
    <button className="icon-button enter-project" title="캔버스에서 프로젝트 열기" aria-label={`${label} 캔버스 열기`} onClick={() => onOpenCanvas(id)}><Icon name="enter" size={17}/></button>
    <NoteMenu id={id} title={label} store={store}/>
  </div>;
}
