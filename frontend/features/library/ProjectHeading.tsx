import { Icon } from '../../shared/Icon';
import type { MemoStore } from '../../shared/useMemoivy';
import { NoteMenu } from './NoteMenu';
import type { Note } from '../../shared/engine_contract';
import { progressLabel } from '../../shared/progress';
import { useProgress } from '../../shared/ProgressProvider';
import { TaskCountLabel, TaskProgress } from '../../shared/TaskProgress';

export function ProjectHeading({ id, label, childCount, note, store, onOpenNote, onOpenCanvas }: { id: string; label: string; childCount: number; note: Note; store: MemoStore; onOpenNote: (id: string) => void; onOpenCanvas: (id?: string) => void }) {
  const progress = useProgress();
  const level = progress.of(note);
  const text = progressLabel(note, progress);
  const hasTasks = level.total > 0;
  return <div className="project-heading">
    <button className="project-title" onClick={() => onOpenNote(id)}>
      <strong>
        <span className="project-folder" aria-hidden="true"><Icon name="project" size={16}/></span>
        <span className="project-name">{label}</span>
        {hasTasks && <TaskCountLabel progress={level} title={text.title} className="task-chip">{text.short}</TaskCountLabel>}
      </strong>
      <span>메모 {childCount}개</span>
      {hasTasks && <TaskProgress className="project-progress" label={label} progress={level}/>}
    </button>
    <button className="icon-button enter-project" title="캔버스에서 프로젝트 열기" aria-label={`${label} 캔버스 열기`} onClick={() => onOpenCanvas(id)}><Icon name="enter" size={17}/></button>
    <NoteMenu id={id} title={label} store={store}/>
  </div>;
}
